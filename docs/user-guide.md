# Agent Guard 사용자 가이드

> 현재 동작: 추천 대신 검토 후보를 제공합니다. 스캔은 프로젝트 루트 한 단계의 이름만 확인하며 하위 폴더·비밀값은 검사하지 않습니다.
> 기존 규칙은 기본 보존하고, 교체하려는 경로마다 별도 확인이 필요합니다. 저장은 scope별로 수행하며 다른 scope의 미저장 변경은 유지됩니다.
> 미리보기와 트리 색상은 선택한 Read/Edit 도구의 명시 규칙 근사 비교입니다. 실행 결과를 보장하지 않습니다.
> 복합 명령·별칭·지원하지 않는 패턴은 판정하지 않습니다. 기존 `permissions.defaultMode`는 저장해도 보존합니다.
> 근거가 부족한 보안 프로필과 프로젝트 위험 점수는 표시하지 않습니다.

Agent Guard는 로컬 코딩 에이전트(Claude Code, Codex CLI, OpenCode)가 내 PC와
프로젝트에서 **무엇에 접근할 수 있는지**를 GUI로 보고 설정하는 데스크톱 앱입니다.
모든 처리는 로컬에서만 이루어지며, 네트워크 전송·텔레메트리가 없습니다.
단, 개발자가 명시적으로 실행하는 Claude permission probe는 인증된 Claude CLI/API를 사용하며
앱 런타임 기능이 아닙니다.

## 핵심 개념

| 용어 | 의미 |
|---|---|
| **Allow / Ask / Deny** | Claude 도구 호출을 각각 사전 승인 / 확인 후 실행 / 차단. Allow만으로 미등록 접근을 차단하지는 않음 |
| **Scope** | Managed file(읽기 전용) · User(`~/.claude/settings.json`) · Project(`.claude/settings.json`) · Local(`.claude/settings.local.json`) |
| **병합 규칙** | Managed > Local > Project > User 순으로 우선하되, **Deny는 어느 Scope에서든 최우선** (deny > ask > allow) |
| **매칭 없는 경로** | 일치하는 명시 규칙이 없다는 뜻이며 Ask를 의미하지 않습니다. 작업 디렉터리의 읽기는 기본 허용될 수 있고, 실제 승인은 도구·실행 모드에 따라 달라집니다 |

경로 규칙은 저장 시 Claude Code의 도구별 규칙으로 확장됩니다. 예:
`src/` Allow → `Read/Edit(./src/**)`.

Managed는 Windows `C:\Program Files\ClaudeCode\`의 file-based 정책만 읽습니다. 조직이
server/MDM/registry로 주입한 정책은 Preview에 보이지 않을 수 있습니다.

> **권한 경계:** `Read/Edit` Deny는 Claude의 내장 파일 도구를 막지만, 이미 허용된
> Bash·PowerShell 프로세스의 OS 파일 접근까지 차단하는 샌드박스는 아닙니다. 셸 명령
> 권한을 좁게 유지하고, 강제 격리가 필요하면 Claude Code sandbox 같은 별도 OS 경계를
> 함께 사용하세요.

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
| `검토 후보` / `작업 경로 후보` (점선) | 이름 기반 관찰입니다. 비밀값·적절한 권한은 확인하지 않았으며 자동 적용되지 않습니다 |
| `검색 제외` (회색 점선) | `.gitignore` 경로 — 차단이 아니라 에이전트 검색에서 빠질 뿐 (아래 참고) |
| 흐림 + 기울임 | 스캔 제외 폴더 (`node_modules`, `.git`, `dist` 등) |

### 가운데 — Policy Editor

경로 선택 → Scope(project/local) → 적용 범위(파일만/폴더만/폴더+하위/패턴) →
**Allow / Ask / Deny** 지정. 단축키: 트리에서 <kbd>A</kbd>/<kbd>K</kbd>/<kbd>D</kbd>.

공유 Project 설정에 Allow가 있는데 Claude Code가 아직 해당 폴더를 신뢰하지 않은 경우,
상단에 경고가 표시됩니다. 이때 공유 Allow는 무시되고 Deny만 적용되므로 프로젝트 폴더에서
대화형 `claude`를 실행해 trust 확인을 승인해야 합니다. Agent Guard는 신뢰 상태를 자동으로
변경하지 않습니다.

### 오른쪽 — Effective Preview / Raw JSON

발견된 managed 파일 및 User/Project/Local의 **명시 규칙 근사 비교**를 봅니다:

| 탭 | 내용 |
|---|---|
| Allow 규칙 / Deny 규칙 / Ask | 도구별 일치 규칙 목록. 실제 승인 결과는 보장하지 않음 |
| Conflicts | Allow와 Deny/Ask가 겹친 경로 (deny 우선) |
| By Scope | Scope별 원본 규칙 |
| Raw Rules | 저장될 실제 `Tool(specifier)` 문자열 |
| **Simulator** | 경로/단순 명령의 명시 규칙 비교기 (아래 참고) |
| **MCP/Hooks** | 권한 모델 밖에서 동작하는 요소 (아래 참고) |
| Raw JSON | settings 파일 원문 편집 (validate/format/저장) |

## 정책 시뮬레이터

경로 또는 Bash·PowerShell 명령을 입력하면 **어느 Scope의 어느 규칙이** 매칭되어
어떤 명시 규칙이 일치하는지 보여줍니다. 지원하지 않는 입력은 판정하지 않습니다.

- **경로** — 현재 편집 중인(저장 전 포함) 규칙 기준
- **셸 명령** — Bash/PowerShell 도구를 선택해 저장된 해당 규칙을 평가. `*`는 위치와
  관계없이 여러 글자·공백에 매칭되고, 끝의 ` *`와 `:*`는 인자 없는 명령도 포함
- 일치 규칙이 없으면 판정하지 않음. 실제 승인 동작은 실행 환경에 따라 달라짐

## MCP 서버 / Hooks

Hooks와 MCP 서버는 **경로 권한 규칙 밖에서** 동작하므로 읽기 전용으로 표시하고
현재 구성과 실행 내용을 확인할 수 있습니다.

- **Hooks** — 도구 실행 전후에 임의 셸 명령을 실행합니다. 명령 내용을 반드시 확인하세요.
- **MCP 서버** — 원격(http/sse) 서버와 context7처럼 웹에 접근하는 서버에는
  `웹 접근` 배지가 붙습니다. 프롬프트·코드가 외부로 나갈 수 있으니 신뢰할 수 있는
  서버만 사용하세요.

## 상단 도구

| 버튼 | 기능 |
|---|---|
| 경로 후보 검토 | 루트 이름 기반 후보를 기본 미선택 상태로 표시. 항목별 Read/Edit와 Allow/Ask/Deny를 직접 선택 |
| Env | Bedrock/환경변수 상태와 Secret 노출 경고 |
| Backups | 백업 목록 · 미리보기 · 복원 (복원 전에도 자동 백업) |
| Report | 현재 정책의 Markdown 리포트 (클립보드/파일) |
| Export / Import | 정책 템플릿 JSON으로 팀 공유 |
| Save | 변경 Diff 확인 → **자동 백업 후** 원자적 저장 (<kbd>Ctrl</kbd>+<kbd>S</kbd>) |

## Bedrock · 환경변수 상태

상단 도구의 **Env** 버튼으로 열리는 **읽기 전용** 화면입니다.

- 에이전트·Bedrock 관련 환경변수의 **설정 여부**를 표로 보여줍니다.
- 환경에 **AWS Secret 값이 노출**되면 경고합니다 — 가능하면 `AWS_PROFILE` 또는 사내 인증
  체계를 사용하고 `settings.json`에 Secret을 직접 넣지 마세요.
- Agent Guard는 Secret 값을 **저장하거나 전송하지 않습니다** (표시는 마스킹).

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
- **보호 경로 후보 검토** — SSH/클라우드 자격증명, 인증서·키, `.env`, Windows 시스템 경로의 패턴을
  검토합니다. 항목은 기본 미선택이며 도구와 권한을 직접 선택합니다.
- **웹 접근 제한** — `WebSearch`/`WebFetch`, Bash `curl`/`wget`, PowerShell
  `Invoke-WebRequest`/`Invoke-RestMethod`/BITS와 그 별칭, `curl.exe`/`wget.exe`를 차단.
  일부만 설정된 기존 구성은 `PARTIAL`로 표시됩니다. 임의 인터프리터의 소켓 접근까지
  막는 OS 방화벽은 아니므로 셸 Allow도 함께 최소화해야 합니다.

### 일반 설정 탭

권한 규칙 외의 Claude Code 전역 설정을 질문형 폼으로 편집합니다.

- **모델·응답** — 기본 모델, 추론 노력 수준, 확장 사고, 응답 언어, 출력 스타일.
- **동작** — 자동 업데이트(채널), 대화 기록 보관 일수, 컨텍스트 자동 압축, 자동 메모리,
  파일 체크포인트(/rewind).
  여기서 자동 업데이트는 **Claude Code 자체 설정**이며 Agent Guard 업데이트 기능이 아닙니다.
  Agent Guard는 현재 [수동 업데이트 정책](release-policy.md)을 사용합니다.
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
- 검증되지 않은 에이전트별 일괄 보안 프리셋은 제공하지 않습니다. 필요한 설정을 직접 편집하고 Diff를 확인하세요.
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
