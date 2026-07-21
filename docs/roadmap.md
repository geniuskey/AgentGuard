# Roadmap — 수직 슬라이스 우선 마일스톤

> 근거: 요구사항서 15장(MVP 범위), 20장(초기 개발 마일스톤), 25장(3대 가치).
> 요구사항서 20장은 계층별(가로) 순서지만, 첫 반복은 **3대 가치를 관통하는 수직
> 슬라이스**를 우선한다(사용자 확정). 이후 M2~M6 순으로 폭을 넓힌다.

---

## 구현 현황 (2026-07-22)

| Iteration | 상태 | 비고 |
|---|---|---|
| 0 스캐폴딩 | ✅ 구현 | workspace + core + Tauri 셸 + SvelteKit |
| 1 3대 가치 슬라이스 | ✅ 구현 | 탐색기 편집·Preview·메타데이터/프로필 재로드와 회귀 테스트 구현. Windows 재실행 승인은 별도 |
| 2 Preview/Diff/Raw JSON | 🟡 부분 | By Scope·Conflicts·백업·검색 구현. Raw JSON은 Monaco가 아닌 textarea; 저장/복원 경계의 Windows 검증 필요 |
| 3 프로필/스캐너 | ✅ 구현 | 프로필·추천·개별 dismiss, 재스캔/재실행 영속성과 회귀 테스트 구현 |
| 4 Should-have | 🟡 부분 | Bedrock/Env·gitignore 추천 구현. WSL/UNC/OneDrive 지원 근거는 아직 없음 |
| 5 패키징 | 🟡 부분 | Windows MSI/NSIS/portable 빌드 성공. 코드 서명, clean-VM 설치·업그레이드·제거 승인 미완료 |
| Could-have | 🟡 부분 | 리포트·Import/Export·인앱 가이드·단축키·Dark mode 구현. 전체 접근성/경로 경계 검증은 진행 중 |
| 후속 (2026-07-12) | 🟡 부분 | 시뮬레이터·MCP/Hooks 가시화·외부 변경 감지·에이전트 상태 구현. 승인 키/handler 분류는 구현, 편집기는 미완료 |
| CC 설정 확장 Phase 1 | ✅ 구현 | `/user` 일반 설정 폼·lint·secret 감지. Phase 2~3는 [claude-code-settings-plan.md](claude-code-settings-plan.md) 참조 |

상태 의미: **구현**은 코드와 자동 테스트가 존재한다는 뜻이며, **승인 전**은
[`release-checklist.md`](release-checklist.md)의 실제 Windows/installer 검증까지 끝났다는 뜻이
아니다. 기존 문서의 “완료” 표기는 이 둘을 섞고 있어 보수적으로 수정했다.

> **Raw JSON 편집기**: 현재 `RawJsonEditor.svelte`는 textarea이며 JSON validate/format/save를
> 제공한다. Monaco 의존성, syntax highlighting, worker/CSP 통합은 구현되지 않았다.
> **검증 기준**: 고정된 테스트 개수는 코드가 바뀔 때 즉시 낡으므로 기록하지 않는다. CI의
> Rust workspace test/fmt/clippy, frontend test/check/build, docs build, npm audit, RustSec audit와
> Windows release checklist 결과를 각각 근거로 삼는다.

## 출시 전 알려진 갭

- production Windows 코드 서명 인증서/비밀키 인프라와 자동 updater endpoint/signing key가
  없다. 현재 정책은 수동 업데이트이며 자세한 조건은 [release-policy.md](release-policy.md)에 있다.
- clean Windows VM에서 NSIS/MSI install/upgrade/uninstall, standard-user, WebView2 유무를
  아직 승인하지 않았다.
- UNC·OneDrive·한글·260자 초과·읽기 전용 경로는 테스트 매트릭스 항목이다. WSL path 변환은
  비범위다.
- managed file scope는 로컬 preview 대상이지만 server/MDM/registry로 주입된 정책 탐지는
  지원하지 않는다.
- 번들 Claude 설정 스키마는 2026-07-22 snapshot이므로 새 Claude Code 릴리스 전에 갱신과
  회귀 검증이 필요하다.
- 영구 진단 로그는 개인정보 최소화를 위해 만들지 않는다. Home의 **진단 복사**는 버전·OS와
  설정 파일 존재 여부만 포함하고 경로·설정 내용은 제외한다.
- MCP 프로젝트 승인 키와 command/prompt/agent/http/mcp hook handler 가시화는 구현됐다.
  다만 위험도는 보수적 heuristic이며 hooks/MCP 편집기는 미완료다.
- Claude Code 권한은 shell 하위 프로세스를 격리하는 OS sandbox가 아니다.

---

## Iteration 0 — 스캐폴딩 (선행)

- 스택 확정됨: **Tauri 2 + SvelteKit(Svelte 5, adapter-static) + Rust + SQLite**. Raw JSON은
  textarea이며 Monaco는 후속 후보다(tech-stack.md).
- SvelteKit 프론트(`src/routes`, `src/lib`, adapter-static) + Tauri 2 셸 스캐폴딩.
- Tauri 2 프로젝트 생성, `src-tauri/src/{main,fs_scan,settings,policy,backup,db,env}.rs` 스텁.
- SQLite 초기화(data-model.md v1 스키마), `%APPDATA%\AgentGuard\` 부트스트랩.
- **산출물**: 빈 창이 뜨고 DB/폴더가 생성되는 실행 파일.

---

## Iteration 1 — 핵심 3대 가치 수직 슬라이스 (최우선)

요구사항서 25장의 3대 가치를 최소 경로로 관통한다.

### 1) 탐색기 기반 Allow/Ask/Deny 편집
- `open_project` → `.claude` 탐지 → `list_dir` lazy 트리(제외 폴더 처리).
- 경로 선택 → Policy Editor에서 Allow/Ask/Deny + Applies To 지정 → dirty 반영.
- `load_settings`/`save_settings`: 보존 파싱 + 팬아웃(policy-model.md D1) + 백업 + 원자적 쓰기.

### 2) Effective Access Preview
- `compute_effective`: 3 scope 병합(effective-policy.md D4), deny>ask>allow.
- Preview 탭 최소 구성: Allowed / Denied / Ask / Raw Rules.
- 프로필(Conservative 등)은 민감 경로 Deny + 소스 Allow 규칙만 적용(D2/Default Deny 폐기).

### 3) 최근 프로젝트 리스크 기억
- `scan_risks` + `compute_risk_score`(risk-scanner.md) → `projects`/`known_sensitive_paths` 저장.
- Home에서 최근 프로젝트 카드 복원(`list_recent_projects`).

### DoD (완료 정의)
실제 `.claude/settings.json`을 읽어 → GUI에서 `src/**` Allow, `secrets/**` Deny →
Preview에서 병합 결과 확인 → 백업 후 저장(왕복 무손실 = 재로드 시 동일) →
앱 재실행 시 Home에 리스크 기록 복원. 이 경로가 D1~D5를 실제로 검증.

---

## Iteration 2 — Preview/Diff/충돌 완성 (요구사항서 M4)

- 저장 전 Diff 뷰어(파일별), 백업 목록/미리보기/복원.
- 충돌 탐지 전체(effective-policy.md 4장) + Conflicts/By Scope 탭.
- Raw JSON 탭(textarea): validate/format/양방향 반영. Monaco/syntax highlight는 후속 후보.

---

## Iteration 3 — 스캐너/프로필 강화 (요구사항서 M5)

- Deny/Allow 추천 일괄 적용 + 개별 무시 + 적용 전 Diff.
- 4개 프로필(Conservative/Balanced/Fast Dev/Custom) 완성.
- 재열기 시 신규 민감 폴더 추천(시나리오 2).

---

## Iteration 4 — Should-have (요구사항서 15.2)

- Bedrock/Environment 읽기 화면(env.rs) + Secret 경고.
- `.gitignore`에 `settings.local.json` 추가 추천.
- 검색/필터, 대용량 폴더 제외 설정.

---

## Iteration 5 — 패키징 (요구사항서 M6)

- Windows exe/MSI/NSIS 빌드와 아이콘은 구현됨.
- 설치/업그레이드/제거는 clean VM 체크리스트 승인이 필요하다.
- production 배포에는 Authenticode 코드 서명이 필수이며 인증서 조달은 외부 blocker다.

---

## Could-have (요구사항서 15.3, 시점 유연)

정책 템플릿 Export/Import, Markdown 리포트, 앱 내 가이드, Dark mode, 단축키.

---

## 명시적 비범위 (요구사항서 15.4 — MVP Won't Have)

Secret 저장 · 클라우드 동기화 · 중앙 정책 서버 · 자동 Claude Code 실행 ·
외부 네트워크 업로드 · 다중 에이전트 완전 지원 · MCP/Hooks 고급 편집기.

---

## 요구사항 커버리지 추적표

| 요구사항 장 | 다루는 문서 |
|---|---|
| 3 제품 철학 | architecture(Local First), security, effective-policy(명시적 경로 규칙) |
| 5 지원 범위 | roadmap(Iteration/비범위) |
| 7 Scope | policy-model, effective-policy, security |
| 8.1~8.3 열기/탐색기/편집 | architecture, ui-spec, policy-model |
| 8.4 프로필 | effective-policy(2장) |
| 8.5 스캐너 | risk-scanner |
| 8.6 Preview | effective-policy(5장), ui-spec |
| 8.7 충돌 | effective-policy(4장) |
| 8.8 Raw JSON | ui-spec(7장), architecture |
| 8.9/8.10 Diff/백업 | data-model(4장), architecture |
| 8.11 최근 프로젝트 | data-model(2·3장), ui-spec(3장) |
| 8.12 리스크 점수 | risk-scanner(4장) |
| 8.13 Bedrock | security(4장), ui-spec(9장) |
| 9 비기능 | architecture, security, data-model |
| 10 UI | ui-spec |
| 11 데이터 저장 | data-model |
| 12/13 정책 모델/변환 | policy-model |
| 14 보안 경고 | security(2·3장) |
| 15 MVP 범위 | roadmap |
| 17 기술 스택 | tech-stack |
| 18 보안 원칙 | security(1장) |
| 20 마일스톤 | roadmap |
| 22 오픈 이슈 | open-issues |
| 23 Repo 구조 | architecture(7장) |
