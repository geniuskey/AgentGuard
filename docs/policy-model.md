# Policy Model & settings.json 변환

> 근거: 요구사항서 12장(내부 정책 모델), 13장(settings.json 변환 요구사항).
> 이 문서는 Agent Guard의 **중립 정책 모델**과 Claude Code `settings.json` 사이의
> 양방향 변환 규칙을 확정한다. 설계 결정 **D1(팬아웃), D3(메타데이터 분리),
> D5(왕복 안전성)**의 상세 명세다.

---

## 1. 왜 중립 모델이 필요한가

Claude Code의 permission은 **도구 중심**(`Read(...)`, `Edit(...)`)이지만 사용자가
탐색기에서 다루는 멘탈 모델은 **경로 중심**("`src/` 폴더는 허용, `secrets/`는 차단")이다.
또한 향후 OpenCode/Codex CLI 등으로 확장하려면 특정 에이전트 스키마에 종속되면 안 된다.
따라서 AG는 경로 중심 중립 모델을 유지하고, 저장 시점에만 에이전트별 형식으로 변환한다.

```
사용자 조작 (경로 중심)
      │
      ▼
중립 정책 모델 (PolicyRule[])  ◀── SQLite에 메타데이터 저장 (reason/riskLevel/notes)
      │  Adapter (Claude Code)
      ▼
settings.json permissions (Tool(specifier) 중심)
```

---

## 2. 중립 정책 모델 스키마

```jsonc
{
  "agent": "claude-code",          // 대상 에이전트 (확장 대비)
  "scope": "project",              // user | project | local
  "defaultPolicy": "deny",         // deny | ask | allow  (프로필에서 파생)
  "rules": [
    {
      "id": "uuid",                // AG 내부 식별자 (SQLite 키)
      "path": "src/**",            // 프로젝트 루트 기준 상대 경로 (POSIX 구분자)
      "policy": "allow",           // allow | ask | deny
      "appliesTo": "folder-and-children",
                                   // file | folder | folder-and-children | pattern
      "tools": null,               // null = FILE_ACCESS_TOOLS 전체, 또는 부분 집합
      "reason": "source code required for coding tasks",
      "riskLevel": "low",          // low | medium | high
      "notes": "",
      "managedByAg": true          // AG가 생성/관리하는 규칙인지
    }
  ]
}
```

- `path`는 항상 **프로젝트 루트 기준 상대·POSIX(`/`) 경로**로 정규화해 저장한다.
  Windows 백슬래시/드라이브 문자는 Rust `settings` 모듈에서 로드/저장 시 변환(→ D5, architecture 참조).
- `reason`, `riskLevel`, `notes`, `id`, `managedByAg`는 **settings.json에 쓰지 않는다**(D3).
  전부 SQLite `project_paths`에 저장하고, 규칙 본문(path/policy/tools)으로 매칭해 복원한다.

---

## 3. FILE_ACCESS_TOOLS (D1의 핵심 상수)

경로 지정이 가능한 Claude Code 도구만 대상으로 한다:

```
FILE_ACCESS_TOOLS = [Read, Edit, Write, Grep, Glob, NotebookEdit]
```

- `Bash`(명령 기반), `WebFetch`(도메인 기반) 등은 경로 정책으로 표현할 수 없으므로 **MVP 범위 밖**.
  파싱 시 만나면 "AG 미관리 규칙"으로 **원문 보존**한다(삭제 금지).
- 이 집합은 Claude Code adapter의 상수이며, 다른 에이전트 adapter는 자체 집합을 정의한다.

---

## 4. 경로 → specifier 변환 (팬아웃)

### 4.1 경로 앵커
Project/Local scope는 프로젝트 루트 기준이므로 `./` 앵커를 쓴다.
User scope는 절대/홈 경로를 쓴다.

| appliesTo | 상대 경로 `src` → specifier |
|---|---|
| `file` | `./src` |
| `folder` | `./src/*` (직속 자식만) |
| `folder-and-children` | `./src/**` (재귀) |
| `pattern` | 입력 패턴 그대로 (`./**/*.env` 등) |

### 4.2 팬아웃 규칙
`tools == null`이면 FILE_ACCESS_TOOLS 6개 전체로 확장, 부분 집합이면 그 도구만.

```
Allow  "src/**"      → permissions.allow += [
                         "Read(./src/**)", "Edit(./src/**)", "Write(./src/**)",
                         "Grep(./src/**)", "Glob(./src/**)", "NotebookEdit(./src/**)" ]
Deny   "secrets/**"  → permissions.deny  += [ 위 6개 도구의 ./secrets/** ]
Ask    "docs/**"     → permissions.ask   += [ 위 6개 도구의 ./docs/** ]
```

### 4.3 검증 예제 (요구사항서 8.1 프로젝트 기준)

내부 모델:
```jsonc
{ "scope": "project", "rules": [
  { "path": "src",     "policy": "allow", "appliesTo": "folder-and-children" },
  { "path": "tests",   "policy": "allow", "appliesTo": "folder-and-children" },
  { "path": "docs",    "policy": "ask",   "appliesTo": "folder-and-children" },
  { "path": "secrets", "policy": "deny",  "appliesTo": "folder-and-children" },
  { "path": ".env",    "policy": "deny",  "appliesTo": "pattern" },
  { "path": "README.md","policy": "allow","appliesTo": "file" }
]}
```

→ Claude Code `settings.json` (project):
```json
{
  "permissions": {
    "allow": [
      "Read(./src/**)", "Edit(./src/**)", "Write(./src/**)", "Grep(./src/**)", "Glob(./src/**)", "NotebookEdit(./src/**)",
      "Read(./tests/**)", "Edit(./tests/**)", "Write(./tests/**)", "Grep(./tests/**)", "Glob(./tests/**)", "NotebookEdit(./tests/**)",
      "Read(./README.md)", "Edit(./README.md)", "Write(./README.md)", "Grep(./README.md)", "Glob(./README.md)", "NotebookEdit(./README.md)"
    ],
    "ask": [
      "Read(./docs/**)", "Edit(./docs/**)", "Write(./docs/**)", "Grep(./docs/**)", "Glob(./docs/**)", "NotebookEdit(./docs/**)"
    ],
    "deny": [
      "Read(./secrets/**)", "Edit(./secrets/**)", "Write(./secrets/**)", "Grep(./secrets/**)", "Glob(./secrets/**)", "NotebookEdit(./secrets/**)",
      "Read(./.env)", "Edit(./.env)", "Write(./.env)", "Grep(./.env)", "Glob(./.env)", "NotebookEdit(./.env)"
    ]
  }
}
```

> `permissions.defaultMode`(deny-by-default)는 더 이상 사용하지 않는다. 정책은 명시적 경로
> 규칙만으로 통제하며, 파일에 남은 `defaultMode`는 저장 시 제거된다(effective-policy.md §2 참조).

---

## 5. specifier → 경로 역변환 (파싱/접기)

`settings.json`을 읽어 내부 모델로 되돌릴 때:

1. 각 배열(allow/deny/ask)의 규칙 문자열을 `Tool(specifier)`로 파싱.
   - FILE_ACCESS_TOOLS에 없는 도구(예: `Bash(...)`, `WebFetch(...)`)는 **`unmanagedRules`로 보존**.
   - 파싱 불가 문자열도 그대로 보존.
2. `(policy, specifier)`로 그룹핑해 그 specifier에 걸린 도구 집합을 모은다.
3. **접기(fold)**:
   - 도구 집합 == FILE_ACCESS_TOOLS 전체 → `tools: null`인 단일 경로 규칙.
   - 부분 집합 → `tools: [...]`를 명시한 tool-specific 규칙.
4. specifier → path/appliesTo 역매핑: `./x/**`→(x, folder-and-children), `./x/*`→(x, folder),
   `./x`→(x, file), 그 외 → (원문, pattern).
5. SQLite에서 `(scope, path, policy)` 매칭으로 reason/riskLevel/notes 복원. 없으면 빈 값.

---

## 6. 왕복 안전성 (D5)

요구사항서 13장 원칙을 구현 규칙으로 확정:

1. **보존 파싱**: Rust `serde_json::Value`로 전체 트리를 읽는다. AG는
   `permissions.allow/deny/ask`만 다루고(그리고 legacy `permissions.defaultMode`는 저장 시 제거)
   **나머지 모든 키·값은 손대지 않는다**.
   (top-level `env`, `hooks`, `model`, `$schema`, 그리고 `permissions` 내 `additionalDirectories` 등)
2. **미관리 규칙 보존**: allow/deny/ask 배열 안에서도 AG가 파싱하지 못한 원소는 원위치·원순서로 유지.
   재작성 시 [AG 관리 규칙] + [보존된 미관리 규칙] 순으로 병합, 중복 제거.
3. **결정적 정렬**: AG가 쓰는 규칙은 (tool 순서 = FILE_ACCESS_TOOLS 순, path 사전순)으로 정렬해
   Diff 노이즈를 줄인다.
4. **실패 시 중단**: 파싱/변환/직렬화 중 오류가 나면 **저장하지 않고** 원본을 보존(요구사항 9.4).
5. **저장 전 백업 + Diff 필수**(요구사항 8.9, 8.10; data-model.md 참조).

---

## 7. 향후 확장 (adapter 인터페이스)

Open Issue #6 대비, adapter는 다음 인터페이스로 추상화한다(문서 수준):

```
trait AgentAdapter {
  fn file_access_tools() -> &[Tool];
  fn to_settings(rules: &[PolicyRule], scope: Scope) -> Result<serde_json::Value>;
  fn from_settings(json: &serde_json::Value, scope: Scope) -> Result<(Vec<PolicyRule>, UnmanagedRules)>;
  fn settings_paths(project_root: &Path) -> ScopePaths;   // user/project/local 파일 위치
}
```

Claude Code adapter가 첫 구현체. OpenCode/Codex/Roo는 후속 반복.
