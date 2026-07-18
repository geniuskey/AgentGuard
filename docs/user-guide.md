# Agent Guard 사용자 가이드

Agent Guard는 로컬 코딩 에이전트(Claude Code, Codex CLI, OpenCode)가 내 PC와
프로젝트에서 **무엇에 접근할 수 있는지**를 GUI로 보고 설정하는 데스크톱 앱입니다.
모든 처리는 로컬에서만 이루어지며, 네트워크 전송·텔레메트리가 없습니다.

## 핵심 개념

| 용어 | 의미 |
|---|---|
| **Allow / Ask / Deny** | 경로 접근을 각각 허용 / 확인 후 실행 / 차단 |
| **Scope** | User(`~/.claude/settings.json`) · Project(`.claude/settings.json`) · Local(`.claude/settings.local.json`) |
| **병합 규칙** | Local > Project > User 순으로 우선하되, **Deny는 어느 Scope에서든 최우선** (deny > ask > allow) |
| **매칭 없는 경로** | 규칙이 없으면 Claude Code 기본 동작(실행 시 확인)을 따릅니다. 민감 경로는 폴더 단위로 **Deny**를 지정해 차단하세요 |

경로 규칙은 저장 시 Claude Code의 도구별 규칙으로 확장됩니다. 예:
`src/` Allow → `Read/Edit/Write/Grep/Glob/NotebookEdit(./src/**)`.

## 홈 화면

- **프로젝트 열기** — 폴더를 선택하면 위험 경로가 자동 스캔되고 리스크 점수가 계산됩니다.
- **최근 프로젝트** — 마지막 리스크 점수와 함께 복원됩니다.
- **전역 에이전트 설정** — 홈 폴더에서 발견한 에이전트별 전역 설정으로 바로 이동합니다
  (Claude Code는 구조화 편집기, Codex/OpenCode는 설정 폼 + Raw 편집기).
- **테마 전환** — 우측 상단 버튼으로 라이트/다크 전환 (창 타이틀바 포함, 재시작 후 유지).

## 프로젝트 화면 (3열)

### 왼쪽 — 파일 탐색기

트리의 배지 의미:

| 배지 | 의미 |
|---|---|
| `ALLOW` `ASK` `DENY` (실선) | 실제로 걸려 있는 규칙 |
| `ALLOW?` `DENY?` (점선) | 스캐너 **추천** — 아직 효력 없음. "추천 적용"으로 일괄 반영 |
| `검색 제외` (회색 점선) | `.gitignore` 경로 — 차단이 아니라 에이전트 검색에서 빠질 뿐 (아래 참고) |
| 흐림 + 기울임 | 스캔 제외 폴더 (`node_modules`, `.git`, `dist` 등) |

### 가운데 — Policy Editor

경로 선택 → Scope(project/local) → 적용 범위(파일만/폴더만/폴더+하위/패턴) →
**Allow / Ask / Deny** 지정. 단축키: 트리에서 <kbd>A</kbd>/<kbd>K</kbd>/<kbd>D</kbd>.

### 오른쪽 — Effective Preview / Raw JSON

3개 Scope가 병합된 **최종 결과**를 봅니다:

| 탭 | 내용 |
|---|---|
| Allowed / Denied / Ask | 최종 판정별 경로 목록 |
| Conflicts | Allow와 Deny/Ask가 겹친 경로 (deny 우선) |
| By Scope | Scope별 원본 규칙 |
| Raw Rules | 저장될 실제 `Tool(specifier)` 문자열 |
| **Simulator** | "이 경로/명령이 허용되나?" 테스터 (아래 참고) |
| **MCP/Hooks** | 권한 모델 밖에서 동작하는 요소 (아래 참고) |
| Raw JSON | settings 파일 원문 편집 (validate/format/저장) |

## 정책 시뮬레이터

경로 또는 Bash 명령을 입력하면 **어느 Scope의 어느 규칙이** 매칭되어
허용/확인/차단되는지 근거와 함께 보여줍니다.

- **경로** — 현재 편집 중인(저장 전 포함) 규칙 기준
- **Bash 명령** — 저장된 설정 파일의 `Bash(...)` 규칙 기준 (`Bash(npm run test:*)` = 접두사 매칭)
- 일치 규칙이 없으면 기본 동작에 따라 실행 시 확인을 요청

## MCP 서버 / Hooks

Hooks와 MCP 서버는 **경로 권한 규칙 밖에서** 동작하므로 읽기 전용으로 표시하고
리스크 점수에 반영합니다.

- **Hooks** — 도구 실행 전후에 임의 셸 명령을 실행합니다. 명령 내용을 반드시 확인하세요.
- **MCP 서버** — 원격(http/sse) 서버와 context7처럼 웹에 접근하는 서버에는
  `웹 접근` 배지가 붙습니다. 프롬프트·코드가 외부로 나갈 수 있으니 신뢰할 수 있는
  서버만 사용하세요.

## 상단 도구

| 버튼 | 기능 |
|---|---|
| Profile | Conservative / Balanced / Fast Dev / Custom 기본 규칙 세트 적용 |
| 추천 적용 | 스캐너의 Deny/Allow 추천을 일괄 반영 (저장 전 Diff로 확인) |
| Env | Bedrock/환경변수 상태와 Secret 노출 경고 |
| Backups | 백업 목록 · 미리보기 · 복원 (복원 전에도 자동 백업) |
| Report | 현재 정책의 Markdown 리포트 (클립보드/파일) |
| Export / Import | 정책 템플릿 JSON으로 팀 공유 |
| Save | 변경 Diff 확인 → **자동 백업 후** 원자적 저장 (<kbd>Ctrl</kbd>+<kbd>S</kbd>) |

## 저장 · 백업 · 외부 변경 감지

- 저장 전 항상 파일별 Diff를 보여주고, 기존 파일은 `%APPDATA%\AgentGuard\backups`에 백업합니다.
- 설정 파일이 **앱 밖에서** 수정되면(에이전트/에디터): 편집 중이 아니면 자동으로 다시
  불러오고, 편집 중이면 경고 배너에서 "다시 불러오기 / 무시"를 선택합니다.
- `settings.json`의 알 수 없는 필드와 수기 규칙은 저장 시 **그대로 보존**됩니다.

## 사용자(전역) 설정 — `~/.claude/settings.json`

모든 프로젝트에 공통 적용되는 설정입니다.

- **규칙 목록** — 경로 트리로 규칙을 확인/수정. 패턴 직접 입력 가능
  (`~/.aws/**`, `//c/Windows/**`, `//**/*.key`).
- **시스템 탐색기** — 전체 드라이브를 탐색해 폴더를 바로 규칙으로 지정.
- **작업 폴더 (Allow) / 민감 폴더 (Deny)** — 폴더 선택 시 올바른 전역 패턴으로 자동 변환
  (`C:\Windows` → `//c/Windows/**`, 홈 아래는 `~/…/**`).
- **보안 베이스라인** — SSH/클라우드 자격증명, 인증서·키, `.env`, Windows 시스템 경로를
  일괄 Deny.
- **웹·네트워크 차단** — `WebSearch`/`WebFetch`/`curl`/`wget` 차단 (프롬프트·데이터 유출 방지).

### 일반 설정 탭

권한 규칙 외의 Claude Code 전역 설정을 질문형 폼으로 편집합니다.

- **모델·응답** — 기본 모델, 추론 노력 수준, 확장 사고, 응답 언어, 출력 스타일.
- **동작** — 자동 업데이트(채널), 대화 기록 보관 일수, 컨텍스트 자동 압축, 자동 메모리,
  파일 체크포인트(/rewind).
- **Git·커밋** — 커밋에 Claude 서명(Co-Authored-By) 포함 여부.
- **권한·보안** — 권한 우회 모드 금지(권장), 프로젝트 MCP 서버 자동 승인(주의),
  추가 접근 허용 폴더(폴더 픽커).
- **환경변수(env)** — Claude Code 세션 전용 환경변수. 비밀값이 평문으로 들어가면
  경고합니다 (`$VAR` 참조는 안전).
- 모든 필드는 "(기본값 — 키 없음)"을 선택하면 키를 제거해 Claude Code 기본 동작을 따르고,
  폼에 없는 키는 Raw JSON 탭에서 편집하며 **그대로 보존**됩니다. 저장 전 Diff + 자동 백업은
  동일하게 적용됩니다.

## 에이전트 설정 (Codex CLI / OpenCode)

JSON/TOML로만 관리되던 전역 설정을 GUI로 편집합니다.

- **설정 폼** — 모델, 승인 정책, 샌드박스, 권한(permission), 공유/자동 업데이트 등
  핵심 키를 드롭다운/입력으로 편집. 폼에 없는 키는 Raw 탭에서 편집하며 **그대로 보존**됩니다.
- **보안 상태 칩** — 현재 설정의 보안 관련 값 요약 (안전 = 녹색, 주의 = 황색).
- **보안 베이스라인** — 권장 보안 설정을 기존 설정에 병합 (검토 후 저장).
- 참고: TOML 주석은 재직렬화 시 사라집니다 — 저장 전 Diff에서 확인하세요.

## `.gitignore`와 에이전트 가시성

`.gitignore` 경로는 **차단된 것이 아닙니다**. 도구별로 다르게 동작합니다:

| 도구 | 동작 |
|---|---|
| Grep 검색 | **발견 못 함** (ripgrep이 .gitignore 존중) |
| Glob 나열 | 보임 |
| Read (경로 지정) | 가능 — 접근 권한은 Allow/Ask/Deny 규칙이 결정 |

에이전트가 이 경로를 활용하게 하려면: Policy Editor에서 **Allow 규칙**을 걸고
**"CLAUDE.md에 알리기"** 버튼으로 경로를 기록하세요. 에이전트가 직접 읽거나
`rg --no-ignore`로 검색하게 됩니다.

## 리스크 점수

프로젝트를 열 때 아래 신호로 0~100점을 계산합니다 (Low 0–20 · Medium 21–60 · High 61+):

| 신호 | 가중치 |
|---|---:|
| `.env` 계열 존재 | +20 |
| `secrets/` 존재 | +30 |
| `raw/` 존재 | +20 |
| `data/` 존재 | +15 |
| 인증서/키 파일 (`*.pem`, `id_rsa` 등) | +30 |
| 소스와 민감 데이터 혼재 | +20 |
| `settings.local.json` 없음 | +5 |
| 사실상 전체 Allow 상태 | +50 |
| Hooks 설정됨 | +15 |
| MCP 서버 설정됨 | +10 |

## 키보드 단축키

| 키 | 동작 |
|---|---|
| <kbd>Ctrl/⌘</kbd>+<kbd>S</kbd> | 저장 다이얼로그 |
| <kbd>A</kbd> / <kbd>K</kbd> / <kbd>D</kbd> | 선택 경로를 Allow / Ask / Deny (폴더+하위) |

## 데이터 위치 & 안전장치

- 앱 데이터(SQLite, 백업): `%APPDATA%\AgentGuard\`
- `settings.json`에는 순수 규칙만 기록하고, 메모/사유 등 앱 메타데이터는 로컬 DB에만 저장합니다.
- Secret 값은 저장·전송하지 않습니다.
- `settings.local.json`이 `.gitignore`에 없으면 배너로 등록을 권장합니다.
