# Architecture

> 근거: 요구사항서 17장(기술 스택), 23장(Repo 구조). 현재 구현의
> **SvelteKit UI + Tauri command 경계 + 독립 Rust core**를 기준으로 기술한다.

---

## 1. 큰 그림

```
┌──────────────────────────────────────────────────────────┐
│  Frontend (WebView)                                        │
│  Home · Project Explorer · Policy Editor · Preview ·       │
│  Diff Viewer · Raw JSON Editor · Bedrock/Env               │
└───────────────▲───────────────────────┬───────────────────┘
                │  invoke() commands     │  events (progress)
                │  (JSON 직렬화)          ▼
┌───────────────┴───────────────────────────────────────────┐
│  Tauri commands (IPC + path boundary)                      │
│  agentguard-core (settings · policy · backup · db · risk) │
└───────────────┬───────────────────────────────────────────┘
                ▼
   로컬 파일시스템 · %APPDATA%\AgentGuard\ (app.db, backups/)
```

- **Local First**: 데스크톱 런타임은 네트워크/텔레메트리 없이 로컬에서 처리한다. 개발자용
  Claude permission probe는 사용자가 따로 실행하는 CLI/API 검증이며 앱 런타임에 포함되지 않는다.
- **경계 원칙**: 파일 I/O·JSON 직렬화·glob 매칭·DB는 **전부 Rust**. 프론트는 표시/상호작용만.
  민감 로직(변환·병합·백업)을 Rust에 두어 무결성과 테스트 용이성 확보.

---

## 2. Rust 모듈 책임 (요구사항서 23장 매핑)

| 모듈 | 책임 |
|---|---|
| `src-tauri/src/main.rs` | Tauri 부트스트랩, command 등록, DB 상태 초기화 |
| `src-tauri/src/commands.rs` | 프론트 IPC DTO, 허용 경로 검증, core I/O orchestration |
| `core/fs_scan.rs` | 프로젝트 파일 트리 lazy 스캔, 제외 폴더 처리, 위험 경로 스캔 |
| `core/settings.rs` | User/Project/Local/managed-file 설정 탐지, 보존 파싱, 원자적 쓰기 |
| `core/policy.rs`·`effective.rs` | 중립 모델 변환, Effective Policy 병합, 충돌 탐지 |
| `core/backup.rs`·`db.rs` | 백업/복원 primitive와 SQLite CRUD·마이그레이션 |
| `core/risk.rs`·`inspect.rs`·`simulate.rs` | 리스크 점수, 에이전트 표면 검사, 정책 시뮬레이션 |
| `core/env.rs`·`settings_lint.rs` | AWS/Proxy 상태, Secret/Claude 설정 검증 경고 |

각 모듈은 순수 함수(변환·병합·리스크 계산)와 I/O를 분리해 단위 테스트가 가능하게 한다.

---

## 3. Tauri Command 계약 (초안)

프론트-백엔드 계약. 이름/시그니처는 구현 시 조정 가능하나 이 범위를 벗어나지 않는다.

### 프로젝트 / 파일 트리
```
open_project(path) -> ProjectInfo            // .claude 탐지, 리스크 스캔, DB upsert
list_dir(path) -> DirEntry[]                 // lazy: 한 depth만, 제외 폴더 표시
scan_risks(projectRoot) -> RiskFinding[]     // 민감/허용 후보 (risk-scanner.md)
compute_risk_score(projectRoot) -> RiskScore // 점수+등급
```

### settings 로드/저장
```
load_settings(projectRoot) -> ScopedSettings // managed(read-only)+user/project/local 규칙
build_diff(scope, nextRules) -> DiffResult    // 저장 전 Diff (변경 없이 계산)
save_settings(scope, nextRules, {backup:true}) -> SaveResult  // 백업→검증→쓰기, 실패시 원본 보존
```

### 정책 / Preview
```
to_settings_preview(rules, scope) -> RawRules // 팬아웃 결과 미리보기
compute_effective(projectRoot, targetPath?) -> EffectivePolicy[]  // 병합 결과
detect_conflicts(projectRoot) -> Conflict[]
```

### Raw JSON
```
validate_json(text) -> ValidationResult      // 구문; 번들 스키마/lint는 별도 검증 계층
format_json(text) -> string
```

### 백업 / 복원
```
list_backups(projectId) -> Backup[]
preview_backup(backupId) -> string
restore_backup(backupId, {backup:true}) -> RestoreResult
```

### 최근 프로젝트 / 환경
```
list_recent_projects() -> RecentProject[]
get_env_status() -> EnvStatus                 // Should-have (env.rs)
```

---

## 4. 상태 관리 & 이벤트

- 대용량 스캔은 Tauri **event**로 진행률을 스트리밍(프론트 프리징 방지, 요구사항서 9.2).
- 프론트 전역 상태: 현재 프로젝트, 로드된 3 scope 규칙, 미저장 변경(dirty), Preview 캐시.
- Save는 항상 (1)변환 → (2)백업 → (3)Diff 확인 → (4)쓰기 파이프라인. 어느 단계 실패 시 롤백.

---

## 5. Windows 경로 처리 (Open Issue #3 부분)

- 내부 저장·정책은 **POSIX 상대 경로**로 정규화. 표시·파일 I/O 시에만 OS 경로로 변환.
- 드라이브 문자/백슬래시/대소문자 비교는 `settings.rs`에서 흡수.
- WSL/UNC 경로는 2차 지원(요구사항서 9.5) — 인터페이스는 열어두되 MVP 미구현.

---

## 6. 저장 위치 (요구사항서 11.1)

```
%APPDATA%\AgentGuard\
├─ app.db              # SQLite (data-model.md)
├─ app-config.json     # 예약된 경로(현재 생성/사용하지 않음)
└─ backups\            # 타임스탬프 백업 (backup.rs)
```

---

## 7. 초기 Repo 구조 (요구사항서 23장)

프론트엔드는 **SvelteKit(Svelte 5) + adapter-static(SPA)** 확정(tech-stack.md).

```
AgentGuard/
├─ crates/agentguard-core/src/       # Tauri-independent domain/I/O logic
├─ src-tauri/
│  ├─ src/{main,commands}.rs         # shell + narrow IPC layer
│  └─ tauri.conf.json
├─ src/                          # SvelteKit 프론트
│  ├─ routes/                    # 화면 (Home, Explorer 등) — SSR off
│  ├─ lib/
│  │  ├─ components/             # FileExplorer/PolicyEditor/EffectivePreview/
│  │  │                          #   RawJsonEditor/DiffViewer .svelte
│  │  ├─ ipc.ts                  # Tauri invoke 래퍼
│  │  └─ {policy,settings,risk}.ts  # command 타입 래퍼 (로직은 Rust)
│  └─ app.html
├─ svelte.config.js              # adapter-static
├─ docs/                         # 본 설계 문서 묶음
└─ README.md
```

- SvelteKit은 `adapter-static`으로 정적 SPA 빌드 → Tauri WebView가 로드.
- 상태는 Svelte 5 runes(`$state`/`$derived`)로 관리(전역은 `lib`의 runes 모듈).
- Raw JSON은 현재 `RawJsonEditor.svelte`의 textarea로 편집하고 Rust command에서
  검증·포맷한다. Monaco는 구현된 구성요소가 아니라 후속 후보다.
