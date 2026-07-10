# Risk Scanner & 리스크 점수

> 근거: 요구사항서 8.5(위험 경로 자동 스캐너), 8.12(프로젝트 리스크 점수).
> 스캐너의 패턴, 결정적 점수 함수, 추천 워크플로를 확정한다.

---

## 1. 스캐너 동작

프로젝트 열기 시 파일 트리를 스캔해 민감/허용 후보를 분류하고, 탐색기에
**Recommended Deny / Recommended Allow** 배지로 표시한다(요구사항서 8.2 상태 8종).

- 스캔은 lazy 트리와 별개로 백그라운드에서 수행하고 event로 결과를 흘린다.
- 제외 폴더(`node_modules`, `.git`, `.venv`, `dist`, `build`)는 스캔에서 건너뛴다(요구사항서 9.2).
- 결과는 `known_sensitive_paths`에 저장, 사용자는 일괄 적용 또는 개별 무시(dismiss) 가능.
- **추천 적용 전 Diff 제공**(요구사항서 8.5 수용 기준).

---

## 2. Deny 추천 패턴 (요구사항서 8.5)

```
.env            .env.*
*.pem  *.key  *.p12  *.pfx
id_rsa  id_ed25519
secrets/  secret/  credentials/  credential/  keys/
certs/  certificates/
raw/  data/  dataset/  datasets/
exports/  export/  backup/  backups/  dump/  dumps/
logs/  private/  confidential/  personal/
```

## 3. Allow 추천 패턴 (요구사항서 8.5)

```
src/  source/  tests/  test/  docs/  doc/
README.md  CLAUDE.md  AGENTS.md
package.json  pyproject.toml  Cargo.toml
```

> 패턴은 대소문자 무시, 디렉터리 패턴(`raw/`)은 폴더에만, 확장자 패턴(`*.pem`)은 파일에만 매칭.
> 패턴 목록은 `app-config.json` 또는 코드 상수로 관리하고, Custom 프로필에서 사용자 편집 가능(후속).

---

## 4. 리스크 점수 (결정적 함수)

요구사항서 8.12 표를 순수 함수로 확정한다. 각 조건은 프로젝트당 **한 번만** 가산.

```
score = 0
if exists(".env" 계열)                 score += 20
if exists("secrets/")                  score += 30
if exists("raw/")                      score += 20
if exists("data/")                     score += 15
if exists(인증서/키: *.pem|*.key|*.p12|*.pfx|id_rsa|id_ed25519)  score += 30
if (src/ 존재 AND 위 민감 항목 ≥1 존재)  score += 20   # 민감 데이터 혼재
if not exists(".claude/settings.local.json")            score += 5
if (모든 경로가 Allow 상태 == 사실상 무제한)             score += 50
score = min(score, 100)   # 표시상 상한
```

### 등급 (요구사항서 8.12)
| 점수 | 등급 |
|---:|---|
| 0–20 | Low |
| 21–60 | Medium |
| 61+ | High |

> "모든 경로 Allow(+50)"는 현재 유효 정책이 사실상 무제한일 때만 가산한다
> (예: defaultMode가 dontAsk가 아니고 deny/ask가 비어 있으며 루트 allow가 있는 경우).
> 이 항목은 열람 시점의 effective policy(effective-policy.md)에서 판정한다.

---

## 5. 추천 워크플로 (요구사항서 시나리오 1·2)

1. 프로젝트 열기 → 스캐너가 Deny/Allow 후보 표시 + 리스크 점수 계산.
2. 사용자가 프로필 선택(예: Conservative) → 추천이 프로필 규칙과 결합.
3. "추천 일괄 적용" 또는 개별 선택 → 중립 모델 규칙 생성.
4. Effective Preview로 결과 확인 → 저장 전 Diff → 백업 후 저장.
5. 재열기 시(시나리오 2) 이전 기록과 비교해 **새로 생긴 민감 폴더**(예: `exports/`)를 신규 Deny 추천.

---

## 6. 점수 예시 (요구사항서 8.11 pixel-tools)

`raw/`, `data/`, `secrets/`, `.env`, `exports/` 존재 + `src/` 혼재 가정:
```
.env(+20) + secrets/(+30) + raw/(+20) + data/(+15) + 혼재(+20) = 105 → min 100 → High
```
요구사항서의 "Risk: High"와 일치.
