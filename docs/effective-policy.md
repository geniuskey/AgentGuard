# Effective Policy: 병합 · Default Deny · 충돌 탐지 · Preview

> 근거: 요구사항서 8.4(프로필), 8.6(Effective Access Preview), 8.7(충돌 탐지).
> 설계 결정 **D2(Default Deny 매핑)**, **D4(병합 알고리즘)**의 상세 명세.

---

## 1. Claude Code 평가 규칙 (변경 불가한 전제)

- **평가 순서: deny > ask > allow**, first-match.
- **deny는 어느 Scope에서든 최우선**. 더 구체적인 allow가 있어도 deny가 이긴다.
- **Scope 우선순위(설정 병합): Local > Project > User.** 단, 위 deny 우선 규칙이 Scope보다 강하다.
  즉 User의 deny가 Local의 allow를 이긴다.
- 매칭되는 규칙이 하나도 없으면 `defaultMode`에 따른다.

---

## 2. D2 — Default Deny + Allow Island 매핑 (치명적 gotcha)

### 반례
직관적으로 "루트 전체 Deny + 필요한 곳만 Allow"를 쓰고 싶지만:
```
deny:  Read(./**)          ← 모든 경로에 매칭
allow: Read(./src/**)      ← src에도 매칭
결과:  deny가 우선 → src까지 차단됨 ❌
```
**catch-all deny로는 Allow Island을 구현할 수 없다.**

### 해법
`permissions.defaultMode`를 사용한다. `dontAsk` = "미리 승인(allow)되지 않으면 자동 거부".
```
defaultMode: "dontAsk"
allow: Read(./src/**), …     ← 이 섬들만 열림
deny:  Read(./secrets/**), … ← 민감 경로는 belt-and-suspenders로 명시 deny도 추가
결과:  src=허용, secrets=거부, 그 외 전부 자동 거부 ✅
```

### 프로필별 매핑 (요구사항서 8.4)

| 프로필 | defaultMode | allow | ask | deny |
|---|---|---|---|---|
| **Conservative** | `dontAsk` | 사용자가 고른 최소 경로만 | (선택) | 민감 패턴 명시 |
| **Balanced** | `default` | 일반 소스 | 문서/불확실 경로 | 민감 패턴 |
| **Fast Dev** | `default` | (넓게) | — | 민감 패턴만 |
| **Custom** | 사용자 지정 | 사용자 지정 | 사용자 지정 | 사용자 지정 |

> Conservative의 민감 경로 deny는 중복처럼 보이지만, 이후 사용자가 실수로 allow를 넓혀도
> deny 우선 규칙이 방어선을 유지하므로 유지한다.

---

## 3. D4 — Effective Policy 계산 알고리즘

특정 경로(및 도구)에 대한 최종 정책을 구한다.

```
입력: targetPath, (선택) tool
      userRules, projectRules, localRules  (각 scope의 allow/ask/deny)
      effectiveDefaultMode  (Local>Project>User 우선, 마지막에 명시된 값)

절차 (도구별 또는 대표 도구 Read 기준):
1. 세 scope의 모든 규칙을 (policy, scope, ruleString, matches(targetPath)) 로 평탄화.
2. targetPath에 매칭되는 규칙만 남긴다 (gitignore 스타일 glob 매칭).
3. deny 매칭이 하나라도 있으면      → EFFECTIVE = DENY  (출처 = 매칭된 deny 규칙들)
   else ask 매칭이 하나라도 있으면  → EFFECTIVE = ASK
   else allow 매칭이 하나라도 있으면 → EFFECTIVE = ALLOW
   else defaultMode 로 폴백:
        dontAsk → DENY(default), default/acceptEdits → ASK(런타임 프롬프트), plan → 읽기전용
4. 결과 객체 부착:
   { effective, sourceScope, explicitOrInherited, matchedRules[], conflict }
```

### 명시(explicit) vs 상속(inherited)
- 선택한 경로 자체에 규칙이 있으면 **explicit**.
- 상위 폴더 규칙(`src/**`)이 하위 파일(`src/app.ts`)에 적용되면 **inherited**.
- 파일 트리 배지(요구사항서 8.2의 8종 상태)는 이 구분으로 결정:
  Explicit/Inherited × Allow/Deny, Ask, Untracked, Recommended Allow/Deny.

---

## 4. 충돌 탐지 (요구사항서 8.7)

충돌 = 서로 다른 Scope가 같은(또는 겹치는) 경로에 **상반된 정책**을 지정한 경우.

| 유형 | 조건 | 표시 |
|---|---|---|
| Deny-overrides-Allow | 한 scope deny, 다른 scope allow, 경로 겹침 | 🔴 Deny 우선 경고 (가장 강조) |
| Partial block | allow 경로 하위에 더 구체적 deny | 🟠 일부 차단 (예: `src/**` allow + `src/secret/**` deny) |
| Ask-overrides-Allow | allow + ask 겹침 | 🟡 확인 필요 |
| Redundant | 동일 정책 중복 | ⚪ 정리 제안 |

요구사항서 8.7 예시 4건은 모두 이 표로 커버된다:
- `Deny raw/**` + `Allow raw/sample/**` → Deny-overrides-Allow (🔴)
- `Allow src/**` + `Deny src/secret/**` → Partial block (🟠)
- `Allow docs/**` + `Deny docs/private/**` → Partial block (🟠)
- `Deny .env` + `Allow .env` → Deny-overrides-Allow (🔴)

**수용 기준**: 충돌은 저장 전 경고로 노출하고, Deny 우선 케이스는 시각적으로 최강조한다.

---

## 5. Effective Access Preview 명세 (요구사항서 8.6, 10.4)

우측 패널 탭 구성:

| 탭 | 내용 |
|---|---|
| **Allowed** | 최종 ALLOW 경로 목록 (✅) |
| **Denied** | 최종 DENY 경로 목록 (⛔), deny 출처 scope 표시 |
| **Ask** | 최종 ASK 경로 목록 (❓) |
| **Conflicts** | 4장 충돌 목록, 심각도순 정렬 |
| **By Scope** | User/Project/Local 각각의 기여 규칙 |
| **Raw Rules** | 실제 생성될 `Tool(specifier)` 문자열 (읽기 전용 미리보기) |

각 항목에 필수 정보(요구사항서 8.6): 최종 정책 / 출처 Scope / 명시·상속 / 충돌 여부 / Deny 우선 여부.

Preview는 **저장 전 계산**되며 실제 파일을 수정하지 않는다. Raw Rules 탭은
policy-model.md 4장의 팬아웃 결과를 그대로 보여줘 "저장하면 무엇이 쓰이는지"를 투명하게 노출한다.

---

## 6. 성능 참고

- Preview 계산은 규칙 수(수십~수백) 기준이라 저렴하다. 파일 트리 배지 계산은
  화면에 보이는(전개된) 노드에 한해 lazy로 수행한다(요구사항서 9.2).
- glob 매칭은 Rust 측(`globset` 등)에서 수행하고 결과만 프론트로 전달하는 것을 권장.
