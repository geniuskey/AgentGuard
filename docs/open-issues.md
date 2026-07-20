# Open Issues — 결정 & 근거

> 근거: 요구사항서 22장의 오픈 이슈 8건. 각 항목에 이번 구체화 단계의 결정,
> 근거, 남은 리스크를 정리한다.

---

## #1. Claude Code permission 규칙 변환 형식
**결정(해소).** `permissions.allow/deny/ask`는 `Tool(specifier)` 문자열 배열이며,
경로 정책은 `FILE_ACCESS_TOOLS = [Read, Edit]`로 팬아웃한다(Claude Code 파일 권한 검사가 매칭하는 도구는 Read·Edit뿐).
상세: `policy-model.md` (D1). 검증 예제 포함.
- 남은 리스크: Claude Code가 도구 목록/스키마를 바꾸면 상수 갱신 필요 → adapter로 격리.

## #2. `settings.local.json` 우선순위/병합
**결정(해소).** 설정 병합 우선순위는 **Local > Project > User**. 단 permission 평가는
**deny > ask > allow, first-match이며 deny는 전 scope 최우선**. 상세: `effective-policy.md` (D4).
- 남은 리스크: managed(사내 정책) settings가 있을 경우 최상위 → 2차 반영 검토.

## #3. Windows native path ↔ WSL path 매핑
**결정(부분).** 내부는 POSIX 상대 경로로 정규화, I/O 시 OS 경로 변환(`settings.rs`).
**WSL/UNC는 2차 지원**(요구사항서 9.5). 인터페이스만 열어두고 MVP 미구현.
- 남은 리스크: WSL 경로 매핑 규칙은 2차에서 별도 설계 필요.

## #4. 사내 exe 배포 방식
**미결(정책 사안).** 기술 결정 아님 — 사내 배포 채널(사내 포털/그룹웨어/수동)은
조직 정책에 따름. 빌드 산출물은 `roadmap.md` Iteration 5에서 준비.
- 필요한 것: 배포 채널 결정자 확인.

## #5. 앱 서명 인증서 필요 여부
**미결(정책 사안).** Windows SmartScreen 경고 회피를 위해 코드 서명 권장하나,
사내 CA/인증서 조달은 조직 결정. Iteration 5 전 확정 필요.

## #6. 다중 에이전트 adapter 인터페이스
**결정(문서 수준).** `AgentAdapter` 트레잇 초안 정의(policy-model.md 7장).
Claude Code가 첫 구현체, OpenCode/Codex/Roo는 후속. MVP는 단일 adapter.

## #7. JSON Schema 제공 방식
**결정.** Claude Code 공식 스키마(`https://json.schemastore.org/claude-code-settings.json`,
`$schema` 필드)를 참조해 Raw JSON 검증에 사용. AG 자체 스키마는 두지 않는다.
- 남은 리스크: 오프라인(Local First) 환경 대비 스키마를 앱에 **번들**해 네트워크 없이 검증.

## #8. 앱 메타데이터: settings.json vs 별도 DB
**결정(해소).** `reason/riskLevel/notes/managedByAg` 등 앱 메타데이터는 **SQLite에만**
저장하고 settings.json에는 순수 규칙만 기록한다(D3, data-model.md). 알 수 없는 필드는 무손실 보존(D5).
- 남은 리스크: DB와 파일이 어긋날 때(외부 편집) 메타데이터 매칭 실패 → 규칙 본문 기준 재매칭,
  없으면 빈 메타로 표시.

---

## 요약: 이번 단계에서 해소 / 미결

| 이슈 | 상태 |
|---|---|
| #1 변환 형식 | ✅ 해소 |
| #2 우선순위/병합 | ✅ 해소 |
| #3 WSL 경로 | 🟡 부분(2차 지원) |
| #4 배포 방식 | ⛔ 미결(정책) |
| #5 서명 인증서 | ⛔ 미결(정책) |
| #6 adapter | ✅ 문서 수준 결정 |
| #7 JSON Schema | ✅ 해소(번들) |
| #8 메타데이터 위치 | ✅ 해소(DB) |
