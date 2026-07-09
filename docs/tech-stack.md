# Tech Stack — 트레이드오프 & 권장안 (미확정)

> 근거: 요구사항서 17장. 사용자가 이 문서를 검토한 뒤 최종 확정한다.
> 스택 확정 전까지 `architecture.md`는 프론트 프레임워크 비의존적으로 유지한다.

---

## 1. 확정에 가까운 항목 (이견 적음)

| 계층 | 선택 | 이유 |
|---|---|---|
| 셸 | **Tauri 2** | 경량 exe, Rust 백엔드, Windows 우선, 작은 번들. Electron 대비 메모리/용량 우위 |
| 백엔드 | **Rust** | 파일 I/O·JSON 보존 파싱(serde_json)·glob(globset)·SQLite에 강함. 무결성 중요 로직 |
| 저장소 | **SQLite** (`rusqlite`) | 로컬 단일 파일 DB, 메타데이터 저장에 충분 |
| JSON 에디터 | **Monaco** | Raw JSON 하이라이트/검증/포맷에 사실상 표준 |
| 언어(프론트) | **TypeScript** | 타입 안전성 |

Tauri 2 채택 시 파일 대화상자·경로·이벤트 API를 그대로 활용(요구사항서 9.2 lazy/이벤트).

---

## 2. 유일한 결정 지점 — 프론트엔드 프레임워크

### SvelteKit (요구사항서 추천안)
| 장점 | 단점 |
|---|---|
| 작은 번들·빠른 반응성(컴파일 타임) | 생태계·컴포넌트 라이브러리가 React보다 작음 |
| 파일트리/패널 상태를 store로 간결하게 | 팀 내 숙련자 적을 수 있음(러닝커브) |
| Tauri 공식 예제·궁합 좋음 | Monaco 통합 예제가 React만큼 많지 않음 |

### React (+ Vite) — 대안
| 장점 | 단점 |
|---|---|
| 최대 생태계, 트리/Diff/Monaco 컴포넌트 풍부 | 번들 큼, 리렌더 관리 비용 |
| 채용/유지보수 인력 풀 넓음 | 상태관리 보일러플레이트 |
| VS Code류 UI 참고 자원 많음 | Tauri 궁합은 무난하나 Svelte보다 "경량" 철학과는 거리 |

---

## 3. 권장안

**Tauri 2 + SvelteKit + TypeScript + Rust + SQLite + Monaco** (요구사항서 추천안 유지).

근거:
- 무거운 로직은 전부 Rust에 있어 프론트는 "표시/상호작용"이 대부분 → Svelte의
  작은 번들·간결한 반응성이 이 앱의 성격(경량 로컬 도구)과 잘 맞는다.
- Monaco는 Svelte에서도 web component/래퍼로 통합 가능(Raw JSON 탭 한 곳).

**단, 팀에 React 숙련자가 많거나 재사용할 사내 React 컴포넌트가 있다면 React로 전환 권장.**
아키텍처가 프레임워크 비의존이라 전환 비용은 프론트 계층에 한정된다.

---

## 4. 확정 시 영향

- 선택 후 `agent-guard/src/` 하위 구조(요구사항서 23장)를 해당 프레임워크로 스캐폴딩.
- 컴포넌트: FileExplorer, PolicyEditor, EffectivePreview, RawJsonEditor, DiffViewer
  (ui-spec.md 매핑).
- `lib/{policy,settings,risk}.ts`는 Rust command의 얇은 타입 래퍼로 유지(로직은 Rust).
