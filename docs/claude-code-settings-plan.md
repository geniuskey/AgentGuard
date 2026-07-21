# Claude Code 설정 확장 계획 (검토 기록)

> 2026-07-18 검토. 최신 Claude Code 설정 표면 조사(공식 문서 + 공식 JSON Schema
> https://www.schemastore.org/claude-code-settings.json ) 결과와, AgentGuard에
> 그 설정들을 "비개발자도 쉽게" 다루게 하는 단계별 계획·백로그를 기록한다.

---

## 1. 배경

AgentGuard는 지금까지 `permissions`(명시적 allow/ask/deny 경로 규칙) 중심이었다
(`defaultMode`는 2026-07-16 별도 PR에서 관리 대상에서 제외됨 — §2 참고).
Claude Code의 설정 표면은 그보다 훨씬 넓다:

| 영역 | 파일 | 스코프 |
|---|---|---|
| 권한 | settings.json `permissions` | user / project / local / **managed** |
| 일반 동작 | settings.json 최상위 키 30여 개 | 주로 user |
| 환경변수 | settings.json `env` | 전 스코프 |
| Hooks | settings.json `hooks` (이벤트·핸들러 타입 대폭 확장) | 전 스코프 |
| MCP 서버 | `.mcp.json`(project), `~/.claude.json`(user) + 승인 키 | project / user |
| 메모리·지침 | `CLAUDE.md` 계층, `CLAUDE.local.md`, `.claude/rules/*.md` | 계층 병합 |
| Subagents | `.claude/agents/*.md` | user / project |
| Skills | `.claude/skills/*/SKILL.md` | user / project / plugin |
| Keybindings | `~/.claude/keybindings.json` | user |
| 관리형 정책 | `C:\Program Files\ClaudeCode\` (Windows) | 최우선 |

## 2. 공식 스키마로 확정한 사실 (드리프트 주의 포함)

- `permissions.defaultMode` enum: `default | acceptEdits | plan | auto | dontAsk |
  delegate | bypassPermissions` — `auto`/`delegate`가 새로 추가됨. **단, AgentGuard는 이 키를
  더 이상 관리하지 않는다**(2026-07-16, #2) — 규칙 편집기 저장 시 항상 제거되고 UI 토글도 없음.
  `settings_lint`는 Raw로 남은 값의 타입/enum만 참고용으로 검사한다.
- `permissions.disableBypassPermissionsMode`: **문자열 enum `["disable"]`** (boolean 아님).
- `includeCoAuthoredBy`: deprecated (→ `attribution` 객체) 이지만 여전히 동작.
- `spinnerTips`: 과거 boolean → 현재 **배열**. `statusLine`: 과거 객체 → 현재 문자열.
  → 버전 드리프트가 실재하므로 이런 키는 lint에서 타입 경고를 내지 않는다(`Expect::Any`).
- MCP 승인 키: `enableAllProjectMcpServers`, `enabledMcpjsonServers`,
  `disabledMcpjsonServers` — MCP 켜기/끄기 UI의 구현 수단.
- 확인된 유용 키: `model`, `availableModels`, `effortLevel(low|medium|high|xhigh)`,
  `language`, `outputStyle`, `cleanupPeriodDays(≥1)`, `autoUpdates`,
  `autoUpdatesChannel(stable|latest)`, `autoCompactEnabled`, `autoMemoryEnabled`,
  `fileCheckpointingEnabled`, `respectGitignore`, `plansDirectory`, `claudeMdExcludes`,
  `apiKeyHelper`, `forceLoginMethod`, `modelOverrides`.

## 3. UX 원칙 (비개발자 기준)

1. JSON 키가 아니라 **질문형 한국어 라벨** ("커밋에 Claude 서명 포함") + 키는 툴팁.
2. 스코프는 생활 언어: "내 컴퓨터 전체 / 이 프로젝트(팀 공유) / 이 프로젝트(나만)".
3. 위험도는 기존 Allow/Ask/Deny 색상 언어 재사용 (위험 값 선택 시 amber caution).
4. 점진 노출: 기본 6~8개, 나머지는 고급/Raw. 모든 필드에 "(기본값 — 키 없음)" 옵션.
5. 저장 전 Diff + 자동 백업 — "언제든 되돌릴 수 있다"는 안전감.
6. hooks/MCP는 **템플릿 카탈로그 우선**, 자유 입력은 고급 뒤로.

## 4. 단계별 계획

### Phase 1 — 일반 설정 폼 (2026-07-18 구현 완료)

- `/user` 화면 3모드: **접근 권한 / 일반 설정 / Raw JSON**.
- `ClaudeSettingsPanel.svelte`: 모델·응답 / 동작 / Git·커밋 / 권한·보안 / env 섹션.
  `config_set_value`(dotted-path, 타 키 보존) + `save_raw_settings`(검증·백업) 재사용.
- `permissions.additionalDirectories`: 네이티브 폴더 픽커 기반 목록 편집.
- `env` key-value 편집기: 평문 secret 감지 경고 (이름 힌트 + 값 프리픽스, `$참조` 제외).
- `settings_lint.rs`(core) + `lint_claude_settings` 명령: 알려진 키 타입/enum 검사,
  모르는 키는 info("보존됩니다"), 드리프트 키는 경고 안 함. 단위 테스트 6종.

### Phase 2 — 보안 가치의 연장 (Guard 정체성 핵심)

- MCP 관리: 서버별 켜기/끄기 = `enabledMcpjsonServers`/`disabledMcpjsonServers` 편집
  (서버 정의는 지우지 않음), 서버 추가/삭제, 웹 접근 경고 유지.
- Hooks 편집: 템플릿 카탈로그("저장 후 포맷", "커밋 전 테스트", "민감 명령 차단") →
  매개변수 폼 → 검토 화면(빨강 강조: "이 명령이 자동 실행됩니다") → Diff → 저장.
- managed 스코프(읽기 전용 4번째 스코프)를 `effective.rs` 병합에 반영.
  **2026-07-22 구현:** Windows file tier(`managed-settings.json` + 정렬된
  `managed-settings.d/*.json`)를 로드한다. server/MDM/registry 정책 탐지는 후속 범위다.

### Phase 3 — 컨텍스트·확장 기능

- CLAUDE.md 계층 뷰어/편집기 + `.claude/rules/*.md`(paths 프론트매터).
- agents/skills 목록·프론트매터 폼 (보안 관점: 임의 지침/도구 표면 가시화).
- statusLine, keybindings, 플러그인 가시화, `attribution` 객체 지원.

## 5. 개선 백로그 (검토에서 발견한 문제 전부)

Phase 배정과 무관하게, 발견된 문제를 빠짐없이 기록한다. 완료 시 체크.

- [x] **managed 파일 스코프**: Windows `C:\Program Files\ClaudeCode\`의 기본 파일과
  drop-in을 읽기 전용 최상위 tier로 Effective Preview/시뮬레이션에 반영. 단
  server/MDM/registry 정책은 로컬 파일 loader로 추론하지 않음. (2026-07-22)
- [x] **defaultMode 신규 enum**: 2026-07-16 별도 PR(#2, 093bbea)에서 `defaultMode` 개념
  자체를 제거함 — RuleListEditor에 선택지가 없고, 규칙 저장(`settings::render`) 시 파일에
  남은 값도 항상 지운다. `auto`/`delegate` 반영 여부는 더 이상 해당 없음.
- [x] **defaultMode 잔존 값 안내**: 규칙 편집기 저장(`render`)은 `permissions.defaultMode`를
  제거하지만 일반 설정/Raw 저장은 보존한다. `settings_lint`가 값을 발견하면 Agent Guard 관리
  대상이 아니며 접근 권한 규칙 저장 시 제거된다는 info를 표시한다. (2026-07-22)
- [x] **hooks handler 가시화**: 이벤트 이름은 새 값을 포함해 그대로 표시하고
  command/prompt/agent/http/mcp handler의 실행 대상·웹 사용·위험도를 분류한다. 이벤트별 의미를
  모두 모델링한 것은 아니므로 위험도는 보수적 heuristic이다. (2026-07-22)
- [x] **MCP 승인 키 반영**: `enableAllProjectMcpServers` / `enabled/disabledMcpjsonServers`
  를 병합해 project MCP의 활성/승인 대기/명시적 비활성 상태를 표시한다. (2026-07-22)
- [ ] **hooks/MCP 편집 불가**: 읽기 전용 → 템플릿 기반 편집기. (Phase 2)
- [ ] **CLAUDE.md / rules/ 미지원**: 계층 뷰·편집·`@import`·paths 스코핑. (Phase 3)
- [ ] **agents/skills 가시화 없음**: 스킬·서브에이전트는 임의 지침 주입/도구 사용 표면인데
  리스크 스캔 대상이 아님. (Phase 3)
- [x] **오프라인 스키마 검증**: 2026-07-22 official SchemaStore snapshot을 core asset으로
  번들하고 `settings_lint`에서 전체 schema + 호환성 경고를 함께 실행. 네트워크 없이 동작한다.
  새 Claude Code release 전 snapshot 갱신·회귀 검증은 계속 필요한 운영 작업이다.
- [ ] **includeCoAuthoredBy deprecated**: `attribution` 객체 지원 후 폼 항목 교체.
- [ ] **프로젝트/로컬 스코프 일반 설정**: ClaudeSettingsPanel은 현재 user 고정 —
  project/local 스코프 파라미터화해 `/project` 화면에도 노출.
- [ ] **keybindings / statusLine / plugins**: 미지원. (Phase 3)
- [ ] **설정 검색**: 한국어 라벨 + 영문 키 동시 매칭 검색 (허브가 커지면 필수).
- [ ] **"내 설정 한 줄 요약"**: 현재 상태를 사람이 읽는 문장으로 (policy report 축약판).

## 6. 구현 메모

- 폼은 레지스트리(데이터) 기반 — 새 키 추가/드리프트 대응은 레지스트리 항목 수정으로 끝.
- 모든 쓰기는 "한 키만 변경, 나머지 보존" (`config_set_value`) + 저장 전 Diff + 백업.
- lint는 **모르는 키를 절대 경고하지 않는다** (신버전 키 오탐 방지) — info로만 알림.
