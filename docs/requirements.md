# Agent Guard 요구사항서

> **Agent Guard**는 Claude Code를 시작으로, 로컬 AI 코딩 에이전트가 프로젝트 내에서 접근할 수 있는 파일·폴더·도구·환경 설정 범위를 안전하게 관리하는 Windows용 로컬 GUI 도구이다.

- 문서 버전: v0.1
- 작성일: 2026-07-09
- 1차 대상: Windows + Claude Code + AWS Bedrock 사용자
- 확장 대상: OpenCode, Codex CLI, Roo Code, Gemini CLI 등 로컬/CLI 기반 코딩 에이전트
- 앱 형태: Tauri 기반 경량 데스크톱 실행 파일
- 실행 파일 예: `AgentGuard.exe`

---

## 1. 배경

AI 코딩 에이전트는 로컬 파일 시스템, 프로젝트 소스코드, 문서, 설정 파일, 빌드 산출물, 테스트 데이터 등에 접근할 수 있다.  
그러나 사내 개발 환경에서는 다음과 같은 문제가 발생한다.

1. 프로젝트 안에 민감한 데이터와 일반 소스코드가 섞여 있다.
2. `.env`, 인증서, 키 파일, 원본 데이터, 내부 문서, 로그, 산출물 등이 에이전트에게 노출될 수 있다.
3. 사용자가 `settings.json` 구조를 직접 이해하고 관리하기 어렵다.
4. User / Project / Local 설정의 적용 범위와 충돌 관계를 눈으로 파악하기 어렵다.
5. 프로젝트마다 어떤 폴더를 허용했고, 어떤 폴더를 차단했는지 기억하기 어렵다.
6. 사내 보안 원칙상 “대부분 차단하고, 필요한 폴더만 허용”하는 방식이 필요하다.

따라서 Agent Guard는 단순 JSON 편집기가 아니라, **AI 코딩 에이전트 접근 경계 설정 도구**로 설계한다.

---

## 2. 제품 한 줄 정의

> Agent Guard는 Windows 사용자가 Claude Code의 `settings.json`을 직접 몰라도, 프로젝트별 파일 접근 정책을 탐색기처럼 보고 안전하게 Allow / Ask / Deny 설정할 수 있게 해주는 로컬 우선 보안 편집기이다.

---

## 3. 제품 철학

### 3.1 Local First

- 모든 처리는 로컬 PC에서 수행한다.
- 프로젝트 파일 목록, 설정 파일, 리스크 기록은 외부 서버로 전송하지 않는다.
- 앱은 네트워크 연결 없이 동작 가능해야 한다.
- 클라우드 동기화 기능은 MVP에서 제외한다.

### 3.2 Default Deny

- 사내 보안 환경의 기본 정책은 `Deny by default`로 둔다.
- 필요한 폴더와 파일만 명시적으로 Allow 처리한다.
- 불확실한 경로는 Ask로 두어 사용자 확인 후 접근하게 한다.

### 3.3 Human Visible Boundary

- 사용자가 JSON을 보지 않아도 최종 접근 범위를 이해할 수 있어야 한다.
- “Claude Code가 볼 수 있는 것 / 볼 수 없는 것 / 물어봐야 하는 것”을 명확히 보여준다.

### 3.4 Reversible Change

- 설정 저장 전에는 항상 Diff를 보여준다.
- 저장 전 자동 백업을 생성한다.
- 이전 설정으로 복원할 수 있어야 한다.

### 3.5 Extensible Agent Policy

- 1차 구현은 Claude Code settings.json을 대상으로 한다.
- 내부 정책 모델은 Claude Code에 종속되지 않도록 설계한다.
- 향후 OpenCode, Codex CLI, Roo Code 등으로 확장 가능해야 한다.

---

## 4. 주요 사용자

### 4.1 1차 사용자

- Windows에서 Claude Code를 사용하는 개발자
- AWS Bedrock 기반 Claude Code 사용 환경을 가진 사용자
- 사내 보안 정책상 민감 데이터 유출을 우려하는 사용자
- CLI 설정 파일을 직접 다루기 부담스러운 사용자

### 4.2 2차 사용자

- 팀 내 AI 코딩 에이전트 사용 가이드를 만드는 담당자
- 비개발 조직에서 AI 코딩 에이전트를 도입하려는 실무자
- 프로젝트별 보안 정책 템플릿을 배포하려는 DX/AX 담당자

---

## 5. 지원 범위

### 5.1 MVP 지원 대상

- Claude Code `settings.json`
- Windows 파일 시스템
- User settings
- Project settings
- Local settings
- 파일/폴더 접근 정책 편집
- 최근 프로젝트 및 리스크 기록
- 설정 백업 및 복원

### 5.2 향후 지원 대상

- Claude Code hooks
- Claude Code MCP 설정
- Claude Code skills 관련 경로 관리
- AWS Bedrock 환경변수 도우미
- OpenCode 설정 파일
- Codex CLI 설정 파일
- Roo Code 설정 파일
- 팀 정책 템플릿 배포

---

## 6. 용어 정의

| 용어 | 의미 |
|---|---|
| Agent | Claude Code, OpenCode, Codex CLI 등 로컬 프로젝트를 읽고 수정할 수 있는 AI 코딩 도구 |
| Policy | 특정 경로, 도구, 명령, 환경변수에 대해 허용/차단/확인 여부를 정의한 규칙 |
| Allow | 에이전트 접근 허용 |
| Ask | 접근 전 사용자 확인 |
| Deny | 에이전트 접근 차단 |
| Scope | 설정 적용 범위. User / Project / Local 등 |
| Effective Policy | 여러 Scope의 설정을 병합한 최종 적용 정책 |
| Risk Profile | 프로젝트의 민감도와 권장 보안 정책 |
| Sensitive Path | 민감 데이터가 있을 가능성이 높은 파일 또는 폴더 |

---

## 7. 설정 Scope 요구사항

Agent Guard는 최소 다음 3개 Scope를 지원해야 한다.

| Scope | 파일 위치 | 목적 |
|---|---|---|
| User | `~/.claude/settings.json` | 모든 프로젝트에 적용되는 개인 기본 설정 |
| Project | `<project>/.claude/settings.json` | 프로젝트에 공유 가능한 팀 설정 |
| Local | `<project>/.claude/settings.local.json` | 특정 PC/사용자 전용 프로젝트 설정 |

### 7.1 User Settings

- 전역 기본 정책을 설정할 수 있어야 한다.
- 공통 Deny 패턴을 설정할 수 있어야 한다.
- 예: `.env`, `*.pem`, `*.key`, `secrets/**` 등

### 7.2 Project Settings

- 프로젝트 저장소에 포함 가능한 설정으로 취급한다.
- 공유 가능한 정책만 저장하도록 안내한다.
- 개인 PC 경로, 개인 AWS 프로필, 개인 인증서 경로 등은 Project settings에 저장하지 않도록 경고한다.

### 7.3 Local Settings

- 프로젝트 내 개인 전용 설정으로 취급한다.
- `.gitignore`에 포함되도록 권장한다.
- 개인 예외, 임시 Allow, 로컬 인증서 경로, 프록시 경로 등을 저장할 수 있다.

---

## 8. 핵심 기능 요구사항

## 8.1 프로젝트 열기

### 요구사항

- 사용자는 프로젝트 루트 폴더를 선택할 수 있어야 한다.
- 앱은 프로젝트 안의 `.claude` 폴더와 설정 파일을 탐지해야 한다.
- 설정 파일이 없으면 생성 여부를 묻거나 자동 생성 옵션을 제공해야 한다.

### 수용 기준

- 사용자가 `D:\work\my-project`를 열면 파일 트리가 표시된다.
- `.claude/settings.json`이 없을 경우 “Project settings 생성” 버튼이 보인다.
- `.claude/settings.local.json`이 없을 경우 “Local settings 생성” 버튼이 보인다.

---

## 8.2 VS Code 스타일 파일 탐색기

### 요구사항

- 왼쪽 패널에 프로젝트 파일 트리를 표시한다.
- 폴더/파일별 접근 정책 상태를 표시한다.
- 상속 정책과 명시 정책을 구분해야 한다.
- 대용량 폴더는 lazy loading 방식으로 탐색해야 한다.

### 표시 상태

| 상태 | 의미 |
|---|---|
| Inherited Deny | 상위 정책으로 인해 차단 |
| Explicit Deny | 해당 경로에 직접 차단 규칙 적용 |
| Inherited Allow | 상위 정책으로 인해 허용 |
| Explicit Allow | 해당 경로에 직접 허용 규칙 적용 |
| Ask | 접근 전 사용자 확인 |
| Untracked | 아직 정책 미정 |
| Recommended Deny | 위험 스캐너가 차단 추천 |
| Recommended Allow | 위험 스캐너가 허용 추천 |

### 예시

```text
project-root                 DENY by default
├─ src                       ALLOW
├─ tests                     ALLOW
├─ docs                      ASK
├─ raw                       DENY
├─ data                      DENY
├─ secrets                   DENY
├─ .env                      DENY
└─ README.md                 ALLOW
```

---

## 8.3 경로 정책 편집

### 요구사항

파일 또는 폴더를 선택하면 오른쪽 패널에서 정책을 수정할 수 있어야 한다.

### 정책 옵션

- Allow
- Ask
- Deny
- Inherit
- Clear explicit rule

### 적용 범위

- This file only
- This folder only
- This folder and children
- Matching pattern
- Tool-specific rule

### 입력 항목

| 항목 | 설명 |
|---|---|
| Path | 선택한 파일/폴더 경로 |
| Policy | Allow / Ask / Deny |
| Source Scope | User / Project / Local |
| Applies To | 적용 범위 |
| Reason | 정책 설정 이유 |
| Risk Level | Low / Medium / High |
| Notes | 추가 메모 |

---

## 8.4 Default Deny + Allow Island 프로필

### 요구사항

보안 프로필을 제공해야 한다.

### 기본 프로필

| 프로필 | 설명 |
|---|---|
| Conservative | 전체 Deny, 사용자가 필요한 경로만 Allow |
| Balanced | 민감 경로 Deny, 일반 소스와 문서는 Ask 또는 Allow |
| Fast Dev | 개발 속도 우선, 민감 패턴만 Deny |
| Custom | 사용자 정의 |

### Conservative 기본 추천

- 프로젝트 루트: Deny
- `src/**`: 사용자가 선택 시 Allow
- `tests/**`: 사용자가 선택 시 Allow
- `docs/**`: 사용자가 선택 시 Allow 또는 Ask
- `raw/**`: Deny 추천
- `data/**`: Deny 추천
- `.env*`: Deny 추천
- `secrets/**`: Deny 추천

---

## 8.5 위험 경로 자동 스캐너

### 요구사항

프로젝트를 열 때 민감할 가능성이 높은 파일/폴더를 자동 탐지한다.

### Deny 추천 패턴

```text
.env
.env.*
*.pem
*.key
*.p12
*.pfx
id_rsa
id_ed25519
secrets/
secret/
credentials/
credential/
keys/
certs/
certificates/
raw/
data/
dataset/
datasets/
exports/
export/
backup/
backups/
dump/
dumps/
logs/
private/
confidential/
personal/
```

### Allow 추천 패턴

```text
src/
source/
tests/
test/
docs/
doc/
README.md
CLAUDE.md
AGENTS.md
package.json
pyproject.toml
Cargo.toml
```

### 수용 기준

- 프로젝트 열기 후 위험 후보가 탐색기에 표시된다.
- 사용자는 추천을 일괄 적용하거나 개별 무시할 수 있다.
- 추천 적용 전 Diff가 제공된다.

---

## 8.6 Effective Access Preview

### 요구사항

여러 Scope의 설정을 병합한 최종 접근 결과를 보여줘야 한다.

### 화면 구성

```text
Claude Code can access:
✅ src/**
✅ tests/**
✅ docs/**
✅ README.md

Claude Code cannot access:
⛔ raw/**
⛔ data/**
⛔ secrets/**
⛔ .env
⛔ *.pem
⛔ *.key

Claude Code will ask before:
❓ notebooks/**
❓ scripts/deploy/**
```

### 필수 정보

- 최종 정책
- 정책 출처 Scope
- 명시 규칙인지 상속 규칙인지
- 충돌 여부
- Deny 우선 적용 여부

---

## 8.7 Scope별 충돌 탐지

### 요구사항

User / Project / Local 설정 간 충돌을 탐지해야 한다.

### 충돌 예시

| User | Project | Local | 결과 |
|---|---|---|---|
| Deny `raw/**` | Allow `raw/sample/**` | 없음 | Deny 우선 경고 |
| Allow `src/**` | Deny `src/secret/**` | 없음 | 일부 차단 |
| 없음 | Allow `docs/**` | Deny `docs/private/**` | 일부 차단 |
| Deny `.env` | 없음 | Allow `.env` | Deny 우선 경고 |

### 수용 기준

- 충돌이 있으면 저장 전 경고한다.
- 최종 적용 결과를 사용자가 확인할 수 있다.
- Deny가 우선되는 경우 시각적으로 강조한다.

---

## 8.8 Raw JSON 편집

### 요구사항

GUI 편집 외에도 Raw JSON 탭을 제공해야 한다.

### 기능

- JSON syntax highlighting
- JSON validation
- JSON formatting
- 설정 스키마 검증
- 저장 전 Diff
- 자동 백업
- 복원

### 수용 기준

- 잘못된 JSON은 저장할 수 없다.
- GUI에서 수정한 정책이 Raw JSON에 반영된다.
- Raw JSON에서 수정한 정책이 GUI에 반영된다.

---

## 8.9 저장 전 Diff

### 요구사항

설정 저장 전 변경 내용을 diff로 보여줘야 한다.

### Diff 대상

- User settings
- Project settings
- Local settings

### 수용 기준

- 변경된 파일별로 Diff를 확인할 수 있다.
- 사용자는 저장, 취소, 백업 후 저장 중 선택할 수 있다.
- 저장 전 자동 백업이 기본 활성화되어야 한다.

---

## 8.10 자동 백업 및 복원

### 요구사항

설정 파일 저장 전 기존 파일을 자동 백업한다.

### 백업 위치

```text
%APPDATA%\AgentGuard\backups\
```

### 백업 파일명 예시

```text
2026-07-09_173000_my-project_project-settings.json
2026-07-09_173000_my-project_local-settings.json
2026-07-09_173000_user-settings.json
```

### 복원 기능

- 최근 백업 목록 보기
- 백업 파일 미리보기
- 현재 설정과 Diff
- 선택 복원

---

## 8.11 최근 프로젝트 및 리스크 기억

### 요구사항

한 번 열었던 프로젝트는 앱이 기억해야 한다.

### 저장 정보

```json
{
  "projectPath": "D:\\work\\pixel-tools",
  "projectName": "pixel-tools",
  "lastOpenedAt": "2026-07-09T17:30:00+09:00",
  "riskProfile": "Conservative",
  "knownSensitivePaths": [
    "raw/",
    "data/",
    "secrets/",
    ".env",
    "exports/"
  ],
  "allowedPaths": [
    "src/",
    "tests/",
    "docs/",
    "README.md"
  ],
  "notes": "raw 폴더는 원본 데이터. Agent 접근 금지."
}
```

### 최근 프로젝트 화면

```text
Recent Projects

pixel-tools
Risk: High
Profile: Conservative
Deny: raw/, data/, secrets/, .env
Allow: src/, tests/, docs/

llm-wiki
Risk: Medium
Profile: Balanced
Deny: raw/, private-notes/
Allow: wiki/, scripts/, docs/
```

---

## 8.12 프로젝트 리스크 점수

### 요구사항

프로젝트를 열면 리스크 점수를 계산해 표시한다.

### 계산 기준 예시

| 항목 | 점수 |
|---|---:|
| `.env` 존재 | +20 |
| `secrets/` 존재 | +30 |
| `raw/` 존재 | +20 |
| `data/` 존재 | +15 |
| 인증서/키 파일 존재 | +30 |
| `src/`와 민감 데이터 혼재 | +20 |
| `.claude/settings.local.json` 없음 | +5 |
| 모든 경로 Allow | +50 |

### 등급

| 점수 | 등급 |
|---:|---|
| 0-20 | Low |
| 21-60 | Medium |
| 61+ | High |

---

## 8.13 AWS Bedrock 환경 도우미

### MVP 포함 여부

- 1차 MVP에서는 읽기/경고 중심으로 포함한다.
- Secret 저장 기능은 제공하지 않는다.

### 요구사항

- AWS 관련 환경변수 존재 여부를 확인한다.
- `AWS_PROFILE` 사용을 권장한다.
- `AWS_SECRET_ACCESS_KEY`를 settings.json에 직접 저장하려는 경우 강하게 경고한다.
- Region, Profile, Proxy, CA bundle 경로를 표시한다.

### 감지 대상

```text
AWS_REGION
AWS_DEFAULT_REGION
AWS_PROFILE
AWS_ACCESS_KEY_ID
AWS_SECRET_ACCESS_KEY
AWS_SESSION_TOKEN
HTTPS_PROXY
HTTP_PROXY
NO_PROXY
REQUESTS_CA_BUNDLE
SSL_CERT_FILE
```

### 보안 경고

```text
AWS_SECRET_ACCESS_KEY를 settings.json에 직접 저장하지 않는 것을 권장합니다.
가능하면 AWS_PROFILE 또는 사내 인증 체계를 사용하세요.
```

---

## 9. 비기능 요구사항

## 9.1 보안

- 앱은 외부 서버로 프로젝트 파일 정보를 전송하지 않는다.
- 앱은 Secret 값을 수집하거나 자체 DB에 저장하지 않는다.
- 설정 저장 전 Diff를 보여준다.
- 자동 백업을 제공한다.
- Project settings에 개인 경로나 Secret으로 보이는 값이 들어가면 경고한다.
- Local settings는 `.gitignore` 등록을 권장한다.

## 9.2 성능

- 10만 개 이하 파일을 가진 프로젝트를 열 수 있어야 한다.
- 파일 트리는 lazy loading 방식으로 표시한다.
- 대용량 폴더는 자동으로 접은 상태로 표시한다.
- `node_modules`, `.git`, `.venv`, `dist`, `build` 등은 기본적으로 탐색 제외 또는 접힘 처리한다.

## 9.3 사용성

- JSON을 몰라도 기본 정책 설정이 가능해야 한다.
- 주요 기능은 클릭 3회 이내로 접근 가능해야 한다.
- 저장 전 경고는 명확하고 짧아야 한다.
- 초보자용 설명과 고급 사용자용 Raw JSON을 모두 제공해야 한다.

## 9.4 안정성

- 잘못된 JSON 저장을 방지한다.
- 파일 쓰기 실패 시 원본을 보존한다.
- 백업 실패 시 저장을 중단하거나 명확히 경고한다.
- 설정 파일이 손상되어도 복구할 수 있어야 한다.

## 9.5 호환성

- Windows 10 이상
- Windows 11 권장
- WSL 프로젝트 경로는 2차 지원
- UNC 경로는 2차 지원
- macOS/Linux는 향후 지원 가능하도록 구조 설계

---

## 10. UI 화면 요구사항

## 10.1 Home

### 구성

- 최근 프로젝트 목록
- 새 프로젝트 열기
- User settings 열기
- 앱 설정
- 문서 링크

### 최근 프로젝트 카드

- 프로젝트명
- 경로
- 마지막 열람일
- 리스크 등급
- 적용 프로필
- 주요 Deny 경로
- 주요 Allow 경로

---

## 10.2 Project Explorer

### 레이아웃

```text
┌─────────────────────────────────────────────────────────────┐
│ Top Bar: Project / Scope / Profile / Save                   │
├───────────────┬─────────────────────┬───────────────────────┤
│ File Explorer │ Policy Editor        │ Effective Preview     │
│               │                     │                       │
│ src/          │ Path                │ Can access            │
│ raw/          │ Policy              │ Cannot access         │
│ data/         │ Scope               │ Will ask              │
│ .env          │ Reason              │ Conflicts             │
└───────────────┴─────────────────────┴───────────────────────┘
```

---

## 10.3 Policy Editor

### 필드

- Path
- Current effective policy
- Explicit policy
- Source scope
- Applies to
- Reason
- Risk level
- Notes
- Clear rule
- Apply
- Preview diff

---

## 10.4 Effective Preview

### 탭

- Allowed
- Denied
- Ask
- Conflicts
- By Scope
- Raw Rules

---

## 10.5 Raw JSON

### 탭

- User settings
- Project settings
- Local settings

### 기능

- Validate
- Format
- Diff
- Save
- Restore
- Open in external editor

---

## 10.6 Bedrock / Environment

### 구성

- AWS Profile 상태
- AWS Region 상태
- Proxy 상태
- CA Bundle 상태
- Secret 감지 경고
- 추천 설정

---

## 11. 데이터 저장 설계

## 11.1 앱 내부 저장 위치

```text
%APPDATA%\AgentGuard\
├─ app.db
├─ app-config.json
└─ backups\
```

## 11.2 SQLite 테이블 초안

### projects

| 컬럼 | 타입 | 설명 |
|---|---|---|
| id | text | UUID |
| path | text | 프로젝트 경로 |
| name | text | 프로젝트명 |
| last_opened_at | text | 마지막 열람 시각 |
| risk_profile | text | 적용 프로필 |
| risk_score | integer | 리스크 점수 |
| risk_level | text | Low / Medium / High |
| notes | text | 메모 |

### project_paths

| 컬럼 | 타입 | 설명 |
|---|---|---|
| id | text | UUID |
| project_id | text | 프로젝트 ID |
| path | text | 상대 경로 |
| policy | text | allow / ask / deny |
| scope | text | user / project / local |
| reason | text | 정책 이유 |
| risk_level | text | Low / Medium / High |
| updated_at | text | 수정 시각 |

### backups

| 컬럼 | 타입 | 설명 |
|---|---|---|
| id | text | UUID |
| project_id | text | 프로젝트 ID |
| scope | text | user / project / local |
| original_path | text | 원본 설정 파일 |
| backup_path | text | 백업 파일 |
| created_at | text | 생성 시각 |

---

## 12. 내부 정책 모델

Agent Guard 내부에서는 Claude Code 원본 설정 구조와 별개로 중립 정책 모델을 유지한다.

```json
{
  "agent": "claude-code",
  "scope": "project",
  "rules": [
    {
      "path": "src/**",
      "policy": "allow",
      "appliesTo": "folder-and-children",
      "reason": "source code required for coding tasks",
      "riskLevel": "low"
    },
    {
      "path": "raw/**",
      "policy": "deny",
      "appliesTo": "folder-and-children",
      "reason": "raw internal data",
      "riskLevel": "high"
    }
  ]
}
```

이 내부 모델을 Claude Code settings.json 형식으로 변환한다.

---

## 13. settings.json 변환 요구사항

### 요구사항

- GUI 정책을 Claude Code settings.json에 반영해야 한다.
- 기존 settings.json의 알 수 없는 필드는 보존해야 한다.
- 앱이 관리하는 영역과 사용자가 직접 작성한 영역을 충돌 없이 다뤄야 한다.

### 원칙

1. 기존 JSON 구조를 최대한 보존한다.
2. 알 수 없는 필드는 삭제하지 않는다.
3. 앱이 관리하는 규칙에는 식별 가능한 메타데이터를 남길 수 있다.
4. 저장 전 변경 Diff를 보여준다.
5. 변환 실패 시 저장하지 않는다.

---

## 14. 보안 경고 규칙

### Project settings에 저장 시 경고할 값

```text
C:\Users\<username>\
D:\private\
AWS_SECRET_ACCESS_KEY
AWS_SESSION_TOKEN
password
passwd
token
secret
api_key
private_key
```

### 위험한 정책 경고

```text
Allow project root /**
Allow raw/**
Allow data/**
Allow secrets/**
Allow .env
Allow *.pem
Allow *.key
```

### 경고 문구 예시

```text
이 설정은 프로젝트 전체를 에이전트에게 허용합니다.
사내 보안 환경에서는 필요한 폴더만 Allow하는 것을 권장합니다.
```

---

## 15. MVP 범위

## 15.1 MVP Must Have

- Tauri 기반 Windows 실행 파일
- 프로젝트 폴더 열기
- Claude Code User / Project / Local settings.json 로드
- 파일 탐색기 트리
- Allow / Ask / Deny 정책 편집
- Default Deny 프로필
- 위험 폴더 자동 추천
- Effective Access Preview
- Scope별 충돌 표시
- Raw JSON 탭
- 저장 전 Diff
- 자동 백업
- 최근 프로젝트 기억
- 프로젝트 리스크 점수

## 15.2 MVP Should Have

- Bedrock / Environment 읽기 전용 상태 표시
- `.gitignore`에 `settings.local.json` 추가 추천
- 정책 Reason 입력
- 검색/필터
- 대용량 폴더 제외 설정

## 15.3 MVP Could Have

- 정책 템플릿 Export / Import
- 팀 공유용 Markdown 리포트 생성
- 앱 내 설정 가이드
- Dark mode
- 키보드 단축키

## 15.4 MVP Won't Have

- Secret 저장
- 클라우드 동기화
- 중앙 정책 서버
- 자동 Claude Code 실행
- 외부 네트워크 업로드
- 다중 에이전트 완전 지원
- MCP/Hooks 고급 편집기

---

## 16. 향후 확장 요구사항

## 16.1 Hooks 편집기

- hook 목록 보기
- hook 실행 조건 편집
- hook 명령어 위험도 표시
- shell command 경고

## 16.2 MCP 편집기

- MCP server 목록 보기
- command / args / env 편집
- 민감 env 감지
- 서버별 활성화/비활성화

## 16.3 Agent별 Adapter

- Claude Code adapter
- OpenCode adapter
- Codex CLI adapter
- Roo Code adapter

## 16.4 Policy Report

- 프로젝트 보안 정책 Markdown 리포트 생성
- Allowed / Denied / Ask 목록 출력
- 리스크 점수 출력
- 팀 공유 가능

---

## 17. 기술 스택

### 추천 스택

```text
Tauri 2
SvelteKit
TypeScript
Rust
SQLite
Monaco Editor
```

### Rust Backend 역할

- 파일 시스템 접근
- 설정 파일 읽기/쓰기
- JSON validation
- 백업 생성
- 프로젝트 스캔
- SQLite 저장
- Windows path normalize
- 환경변수 읽기

### Frontend 역할

- 파일 탐색기 UI
- 정책 편집 UI
- Effective Preview
- Diff Viewer
- Raw JSON Editor
- Bedrock / Environment 상태 화면

---

## 18. 보안 설계 원칙

1. 앱은 프로젝트 내용을 외부로 전송하지 않는다.
2. 앱은 Secret 값을 저장하지 않는다.
3. settings.json 저장 전 자동 백업한다.
4. settings.json 저장 전 Diff를 표시한다.
5. Project settings에는 공유 가능한 정보만 저장하도록 안내한다.
6. Local settings는 개인 환경 설정용으로 유도한다.
7. 위험한 Allow 규칙은 저장 전 경고한다.
8. Deny 규칙은 UI에서 가장 강하게 표시한다.
9. 앱 내부 DB에는 정책 메타데이터만 저장한다.
10. 원본 설정 파일의 알 수 없는 필드는 보존한다.

---

## 19. 예시 사용자 시나리오

### 시나리오 1: 새 프로젝트 보안 설정

1. 사용자가 Agent Guard 실행
2. “Open Project” 클릭
3. 프로젝트 루트 선택
4. Agent Guard가 위험 경로 스캔
5. `raw/`, `data/`, `.env`, `secrets/` Deny 추천
6. 사용자가 Conservative 프로필 선택
7. `src/`, `tests/`, `docs/`만 Allow
8. Effective Preview 확인
9. 저장 전 Diff 확인
10. Project / Local settings 저장

### 시나리오 2: 기존 프로젝트 재열기

1. 사용자가 Home에서 최근 프로젝트 선택
2. Agent Guard가 이전 리스크 기록 표시
3. 새로 생긴 `exports/` 폴더를 Deny 추천
4. 사용자가 Deny 적용
5. 저장 전 Diff 확인
6. 설정 저장

### 시나리오 3: Project와 Local 충돌

1. Project settings에서 `src/**` Allow
2. Local settings에서 `src/secret/**` Deny
3. Effective Preview에서 `src/secret/**` 차단 표시
4. 충돌/우선순위 설명 표시
5. 사용자가 최종 정책 확인 후 저장

---

## 20. 초기 개발 마일스톤

### Milestone 0: 조사 및 모델링

- Claude Code settings 구조 정리
- 권한 규칙 변환 방식 정의
- 내부 policy model 정의
- Windows 경로 처리 방식 정의

### Milestone 1: 프로젝트 열기 및 파일 트리

- Tauri 프로젝트 생성
- 프로젝트 폴더 선택
- 파일 트리 렌더링
- 대용량 폴더 lazy loading

### Milestone 2: settings.json 로드/저장

- User / Project / Local settings 탐지
- JSON 읽기/쓰기
- Raw JSON 편집
- 백업 생성

### Milestone 3: 정책 편집

- Allow / Ask / Deny UI
- Scope 선택
- Reason 입력
- 설정 변환

### Milestone 4: Preview와 Diff

- Effective Access Preview
- Scope 충돌 탐지
- 저장 전 Diff
- 복원 기능

### Milestone 5: 리스크 스캐너

- 민감 경로 감지
- 추천 정책 표시
- 리스크 점수 계산
- 최근 프로젝트 DB 저장

### Milestone 6: 패키징

- Windows exe 빌드
- 아이콘
- 기본 설치/실행 테스트
- 릴리즈 패키지 생성

---

## 21. 성공 기준

### 사용자 관점

- 사용자는 JSON을 몰라도 프로젝트 접근 정책을 설정할 수 있다.
- 사용자는 Claude Code가 접근 가능한 경로를 명확히 이해할 수 있다.
- 사용자는 민감 경로를 실수로 Allow할 가능성이 줄어든다.
- 사용자는 한번 설정한 프로젝트 리스크를 다시 확인할 수 있다.

### 기술 관점

- settings.json을 안전하게 읽고 저장한다.
- 기존 설정의 알 수 없는 필드를 보존한다.
- 저장 전 백업과 Diff를 제공한다.
- 대용량 프로젝트에서도 UI가 멈추지 않는다.

### 보안 관점

- 앱은 외부 서버로 데이터를 전송하지 않는다.
- Secret 값을 저장하지 않는다.
- Project settings에 민감 정보가 들어가는 것을 경고한다.
- Default Deny 방식의 보안 프로필을 제공한다.

---

## 22. 오픈 이슈

1. Claude Code settings.json의 permission 규칙을 정확히 어떤 형식으로 변환할지 추가 검증 필요
2. `settings.local.json`의 실제 우선순위와 병합 방식 검증 필요
3. Windows native path와 WSL path 간 매핑 지원 여부 결정 필요
4. 사내 보안 환경에서 exe 배포 방식 결정 필요
5. 앱 서명 인증서 필요 여부 결정 필요
6. 향후 다중 에이전트 지원을 위한 adapter interface 설계 필요
7. JSON Schema 제공 방식 결정 필요
8. Project settings에 앱 메타데이터를 남길지 별도 DB에만 저장할지 결정 필요

---

## 23. 권장 초기 Repo 구조

```text
agent-guard/
├─ src-tauri/
│  ├─ src/
│  │  ├─ main.rs
│  │  ├─ fs_scan.rs
│  │  ├─ settings.rs
│  │  ├─ policy.rs
│  │  ├─ backup.rs
│  │  └─ db.rs
│  └─ tauri.conf.json
├─ src/
│  ├─ routes/
│  ├─ components/
│  │  ├─ FileExplorer.svelte
│  │  ├─ PolicyEditor.svelte
│  │  ├─ EffectivePreview.svelte
│  │  ├─ RawJsonEditor.svelte
│  │  └─ DiffViewer.svelte
│  ├─ lib/
│  │  ├─ policy.ts
│  │  ├─ settings.ts
│  │  └─ risk.ts
│  └─ app.html
├─ docs/
│  ├─ requirements.md
│  ├─ policy-model.md
│  └─ security.md
├─ README.md
└─ package.json
```

---

## 24. README 초안 문구

```markdown
# Agent Guard

Agent Guard is a local-first desktop policy editor for coding agents.

It helps Windows users safely configure what Claude Code and other local coding agents can access inside a project.

## Core Ideas

- Default Deny
- Allow only what is needed
- Local-only processing
- Visual project boundary editor
- User / Project / Local settings support
- Effective access preview
- Backup before save
```

---

## 25. 결론

Agent Guard는 단순한 `settings.json` GUI 편집기가 아니다.  
이 도구의 핵심은 **AI 코딩 에이전트가 프로젝트 내부에서 넘지 말아야 할 경계를 사용자가 눈으로 확인하고 설계할 수 있게 하는 것**이다.

첫 버전은 Claude Code settings.json을 대상으로 작게 시작하되, 내부 모델은 Agent 일반 정책 편집기로 확장 가능하게 설계한다.

가장 중요한 MVP 가치는 다음 세 가지다.

1. **탐색기 기반 Allow / Ask / Deny 편집**
2. **Effective Access Preview**
3. **최근 프로젝트 리스크 기억**

이 세 가지가 제대로 동작하면 Agent Guard는 Windows 기반 사내 Claude Code 사용자에게 실질적인 보안 보조 도구가 될 수 있다.
