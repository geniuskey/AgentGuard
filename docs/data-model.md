# Data Model: SQLite · app-config · 백업

> 근거: 요구사항서 8.10(백업/복원), 8.11(최근 프로젝트), 11장(데이터 저장 설계).
> 설계 결정 **D3**(앱 메타데이터는 DB에만)을 반영한다.

---

## 1. 저장 위치

```
%APPDATA%\AgentGuard\
├─ app.db              # SQLite: 프로젝트·정책 메타데이터·백업 인덱스
├─ app-config.json     # 예약된 앱 전역 설정 경로(현재 생성/사용하지 않음)
└─ backups\            # 실제 백업 파일들
```

- **DB에는 정책 메타데이터만 저장**(요구사항서 18.9). Secret 값·파일 내용은 절대 저장하지 않는다.

---

## 2. SQLite 스키마

### 2.1 `projects` (요구사항서 11.2)
```sql
CREATE TABLE projects (
  id             TEXT PRIMARY KEY,   -- UUID
  path           TEXT NOT NULL UNIQUE,
  name           TEXT NOT NULL,
  last_opened_at TEXT NOT NULL,      -- ISO8601
  risk_profile   TEXT,               -- Conservative|Balanced|FastDev|Custom
  risk_score     INTEGER,
  risk_level     TEXT,               -- Low|Medium|High
  notes          TEXT
);
```

### 2.2 `project_paths` — 정책 메타데이터 (D3의 저장소)
```sql
CREATE TABLE project_paths (
  id         TEXT PRIMARY KEY,       -- UUID (중립 모델 rule.id)
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  path       TEXT NOT NULL,          -- 프로젝트 루트 기준 POSIX 상대 경로
  policy     TEXT NOT NULL,          -- allow|ask|deny
  scope      TEXT NOT NULL,          -- user|project|local
  applies_to TEXT NOT NULL,          -- file|folder|folder-and-children|pattern
  tools      TEXT,                   -- NULL=전체, 아니면 JSON 배열
  reason     TEXT,
  risk_level TEXT,                   -- Low|Medium|High
  notes      TEXT,
  managed_by_ag INTEGER NOT NULL DEFAULT 1,
  updated_at TEXT NOT NULL,
  UNIQUE(project_id, scope, path, policy)
);
```
> settings.json에는 `path/policy/tools`만 반영되고, 나머지(reason/risk_level/notes 등)는 여기 남는다.
> 로드 시 `(project_id, scope, path, policy)` 매칭으로 메타데이터를 복원한다.

### 2.3 `backups` (요구사항서 11.2)
```sql
CREATE TABLE backups (
  id            TEXT PRIMARY KEY,
  project_id    TEXT REFERENCES projects(id) ON DELETE CASCADE, -- user scope는 NULL 가능
  scope         TEXT NOT NULL,        -- user|project|local
  original_path TEXT NOT NULL,        -- 원본 settings 파일 경로
  backup_path   TEXT NOT NULL,        -- backups\ 내 파일 경로
  created_at    TEXT NOT NULL
);
```

### 2.4 `known_sensitive_paths` (요구사항서 8.11 knownSensitivePaths)
```sql
CREATE TABLE known_sensitive_paths (
  id         TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  path       TEXT NOT NULL,
  source     TEXT NOT NULL,           -- scanner|user
  dismissed  INTEGER NOT NULL DEFAULT 0
);
```

---

## 3. app-config.json (예약)

코드에는 향후 전역 앱 설정을 위한 경로 helper가 있지만 현재 파일을 생성하거나 읽는 기능은
없다. 최근 프로젝트, 프로필, known sensitive path, 정책 메타데이터의 유일한 앱 저장소는
SQLite다. 구현되지 않은 캐시를 운영 데이터로 간주해서는 안 된다.

---

## 4. 백업 파일 규칙 (요구사항서 8.10)

경로: `%APPDATA%\AgentGuard\backups\`

기본 파일명: `{yyyy-MM-dd}_{HHmmss}_{project}_{scope}.json`. 같은 초·이름에 충돌하면 기존
백업을 덮어쓰지 않고 stem 뒤에 UUID를 붙인다.
```
2026-07-09_173000_my-project_project-settings.json
2026-07-09_173000_my-project_local-settings.json
2026-07-09_173000_user-settings.json          # user scope는 project 생략
```

저장 파이프라인(요구사항서 9.4 안정성):
1. 저장 요청 → 원본 존재 시 백업 생성.
2. **백업 실패 시 저장 중단**하고 명확히 경고.
3. 새 내용 검증(JSON 유효 + 변환 성공) 후 원자적 쓰기(임시 파일 → rename).
4. 쓰기 실패 시 원본 보존.

복원 기능: 최근 백업 목록 → 미리보기 → 현재와 Diff → 선택 복원(복원도 백업 후 수행).

---

## 5. 마이그레이션

`app.db`에 `schema_version` 메타 테이블(또는 PRAGMA user_version)을 두고
앱 시작 시 버전 확인·마이그레이션. 초기 버전은 v1.
