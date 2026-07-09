# UI Specification

> 근거: 요구사항서 8.2(파일 탐색기), 10장(UI 화면 요구사항), 9.3(사용성).
> 프레임워크 비의존 — 화면·상태·컴포넌트 책임 수준으로 기술.

---

## 1. 사용성 원칙 (요구사항서 9.3)

- JSON을 몰라도 기본 정책 설정 가능(초보자 GUI) + 고급자용 Raw JSON 병행.
- 주요 기능은 **클릭 3회 이내** 접근.
- 저장 전 경고는 명확하고 짧게. Deny는 가장 강하게 표시.

---

## 2. 화면 목록

| 화면 | 목적 | 요구사항 |
|---|---|---|
| **Home** | 최근 프로젝트, 새 프로젝트/User settings 열기 | 10.1 |
| **Project Explorer** | 3분할 메인 작업 화면 | 10.2 |
| **Policy Editor** | 선택 경로 정책 편집(패널) | 8.3, 10.3 |
| **Effective Preview** | 병합 결과 6탭(패널) | 8.6, 10.4 |
| **Raw JSON** | scope별 JSON 편집 | 8.8, 10.5 |
| **Diff** | 저장 전/백업 비교 (모달) | 8.9 |
| **Bedrock / Environment** | AWS/Proxy 상태(읽기) | 8.13, 10.6 |

---

## 3. Home (요구사항서 10.1)

- 최근 프로젝트 카드: 프로젝트명 · 경로 · 마지막 열람일 · 리스크 등급 · 프로필 ·
  주요 Deny 경로 · 주요 Allow 경로 (요구사항서 8.11 카드 예시).
- 액션: New Project / Open User settings / App settings / Docs 링크.
- 데이터 출처: `list_recent_projects` (app-config 캐시 → DB).

---

## 4. Project Explorer (요구사항서 10.2)

```
┌─ Top Bar: Project | Scope 선택 | Profile | Save ──────────────┐
├──────────────┬────────────────────┬──────────────────────────┤
│ File Explorer│ Policy Editor       │ Effective Preview        │
│ (좌)         │ (중)                │ (우, 6탭)                │
└──────────────┴────────────────────┴──────────────────────────┘
```

### 4.1 파일 탐색기 상태 배지 (요구사항서 8.2 — 8종)

| 배지 | 의미 | 색/강조 |
|---|---|---|
| Inherited Deny | 상위 정책으로 차단 | 빨강(연) |
| Explicit Deny | 직접 차단 | **빨강(강)** |
| Inherited Allow | 상위 정책으로 허용 | 초록(연) |
| Explicit Allow | 직접 허용 | 초록(강) |
| Ask | 접근 전 확인 | 노랑 |
| Untracked | 정책 미정 | 회색 |
| Recommended Deny | 스캐너 차단 추천 | 빨강 점선 |
| Recommended Allow | 스캐너 허용 추천 | 초록 점선 |

- Explicit/Inherited 구분은 effective-policy.md 3장 규칙으로 계산.
- Deny는 항상 가장 강한 시각 처리(요구사항서 18.8).
- **Lazy loading**: 전개된 노드만 `list_dir` 호출. 대용량 폴더는 접힘 기본, 제외 폴더 흐림.

---

## 5. Policy Editor (요구사항서 8.3, 10.3)

필드: Path · Current effective policy · Explicit policy · Source scope ·
Applies to · Reason · Risk level · Notes · [Clear rule] · [Apply] · [Preview diff]

- 정책 옵션: Allow / Ask / Deny / Inherit / Clear explicit rule.
- 적용 범위: This file only / This folder only / This folder and children / Matching pattern / Tool-specific rule.
- Source Scope: User / Project / Local (선택에 따라 저장 파일 결정).
- Apply는 즉시 파일을 쓰지 않고 **미저장 변경(dirty)**으로 반영 → Preview/Diff 후 Save.

---

## 6. Effective Preview (요구사항서 10.4)

6탭: **Allowed · Denied · Ask · Conflicts · By Scope · Raw Rules**
(각 탭 내용은 effective-policy.md 5장 참조). 실시간 재계산, 파일 미수정.

---

## 7. Raw JSON (요구사항서 8.8, 10.5)

- 탭: User / Project / Local settings.
- 기능: Validate · Format · Diff · Save · Restore · Open in external editor.
- syntax highlight + JSON/스키마 검증. 잘못된 JSON은 저장 불가.
- GUI↔Raw 양방향 반영(요구사항서 8.8 수용 기준).

---

## 8. Diff (요구사항서 8.9)

- 저장 전 변경 파일별 Diff 모달.
- 선택지: 저장 / 취소 / 백업 후 저장. **자동 백업 기본 활성**.

---

## 9. Bedrock / Environment (요구사항서 10.6)

- AWS Profile / Region / Proxy / CA Bundle 상태(읽기) + Secret 감지 경고 + 추천 설정.
- 값은 마스킹, 저장 기능 없음(security.md 4장).

---

## 10. Could-have UI (요구사항서 15.3)

Dark mode, 키보드 단축키, 정책 템플릿 Export/Import, 앱 내 설정 가이드, Markdown 리포트.
