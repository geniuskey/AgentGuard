# Roadmap — 수직 슬라이스 우선 마일스톤

> 근거: 요구사항서 15장(MVP 범위), 20장(초기 개발 마일스톤), 25장(3대 가치).
> 요구사항서 20장은 계층별(가로) 순서지만, 첫 반복은 **3대 가치를 관통하는 수직
> 슬라이스**를 우선한다(사용자 확정). 이후 M2~M6 순으로 폭을 넓힌다.

---

## 구현 현황 (2026-07-09)

| Iteration | 상태 | 비고 |
|---|---|---|
| 0 스캐폴딩 | ✅ 완료 | workspace + core + Tauri 셸 + SvelteKit |
| 1 3대 가치 슬라이스 | ✅ 완료 | 탐색기 편집·Effective Preview·리스크 기억 |
| 2 Preview/Diff/Raw JSON | ✅ 완료 | Raw JSON(경량 에디터)·By Scope·Conflicts·백업 복원·스캐너 적용·검색 |
| 3 프로필/스캐너 | ✅ 완료 | 프로필 셀렉터·추천 일괄 적용 |
| 4 Should-have | ✅ 완료 | Bedrock/Env 화면·gitignore 추천 |
| 5 패키징 | 🟡 부분 | 아이콘 세트·번들 설정 완료. MSI/NSIS 빌드·서명은 Windows+정책(#4/#5) |
| Could-have | ✅ 완료 | Markdown 리포트·정책 템플릿 Import/Export·인앱 가이드·키보드 단축키·Dark mode(기본) |

> **Monaco 노트**: Raw JSON 탭은 현재 경량 textarea 에디터(validate/format/save 완비)다.
> 문서상 선택지인 Monaco는 헤드리스 환경에서 렌더링 검증 불가 + CSP `worker-src` 튜닝이
> 필요해, 실제 창(Windows)에서 붙이는 시각 폴리시 항목으로 보류한다. 기능 계약은 충족.
> **검증**: 모든 핵심 로직은 `cargo test --workspace`(35 tests) + 프론트 `npm run build/check`로
> 이 컨테이너에서 검증. Tauri 창 실행만 Windows `npm run tauri dev`.

---

## Iteration 0 — 스캐폴딩 (선행)

- 스택 확정됨: **Tauri 2 + SvelteKit(Svelte 5, adapter-static) + Rust + SQLite + Monaco** (tech-stack.md).
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
- Default Deny 프로필(Conservative) = `defaultMode: dontAsk`(D2) 적용.

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
- Raw JSON 탭(Monaco): validate/format/양방향 반영.

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

- Windows exe 빌드, 아이콘, 설치/실행 테스트, 릴리즈 패키지.
- 서명 인증서 여부는 Open Issue #5 결정에 따름.

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
| 3 제품 철학 | architecture(Local First), security, effective-policy(Default Deny) |
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
