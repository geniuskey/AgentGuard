# Tech Stack — 확정

> 근거: 요구사항서 17장. **결정 기준**: 구현은 에이전트가 담당하고 사용자는 프론트 프레임워크에
> 관여하지 않으므로, "더 가볍고 / 에이전트가 다루기 좋은" 것을 우선한다(사용자 위임).

---

## 확정 스택

| 계층 | 선택 |
|---|---|
| 셸 | **Tauri 2** |
| 프론트엔드 | **SvelteKit (Svelte 5, runes)** — SSR 끄고 `adapter-static`(SPA) |
| 언어(프론트) | **TypeScript** |
| 백엔드 | **Rust** |
| 저장소 | **SQLite** (`rusqlite`) |
| JSON 에디터 | **네이티브 textarea** + Rust 검증/포맷 (현재); Monaco는 후속 후보 |

---

## 1. 이견 적은 계층

- **Tauri 2**: 경량 exe, Rust 백엔드, Windows 우선, 작은 번들(Electron 대비 우위).
  파일 대화상자·경로·이벤트 API를 그대로 활용(요구사항서 9.2 lazy/이벤트).
- **Rust**: 파일 I/O·JSON 보존 파싱(`serde_json`)·glob(`globset`)·SQLite에 강함. 무결성 중요 로직.
- **SQLite**: 로컬 단일 파일 DB, 메타데이터 저장에 충분.
- **Raw JSON textarea**: 추가 런타임·worker 없이 로드되며 Rust command로 JSON 검증/포맷을
  수행한다. 구문 하이라이트는 제공하지 않는다.

---

## 2. 프론트엔드 결정: SvelteKit vs React

| 기준 | SvelteKit (Svelte 5) | React (+Vite) | 판정 |
|---|---|---|---|
| 번들 무게 | 런타임 거의 없음(컴파일 타임 반응성) | React+ReactDOM 런타임 상시 | **Svelte** |
| 에이전트 생성 코드의 단순함/버그 | `$state`/`$derived`/`$effect` → 상태가 곧 변수, 함정 적음 | hooks 규칙·의존성 배열·memo·리렌더 추론 → 버그 여지 | **Svelte** |
| 생태계/기성 컴포넌트 | 상대적으로 적음 | 트리/Diff/Monaco 래퍼 풍부 | React |
| Tauri 궁합 | 공식 템플릿 1급 (SPA) | 공식 템플릿, 무난 | 비등 |
| 나중에 사람이 읽기 | HTML+JS에 가까워 읽기 쉬움 | JSX+훅 개념 필요 | **Svelte** |

---

## 3. 결정 근거 (SvelteKit)

1. **가장 가벼움** — 런타임이 거의 없어 "경량 로컬 도구" 성격에 정확히 맞는다.
2. **에이전트가 생성하는 코드의 미묘한 버그가 적다** — Svelte 5 runes는 상태가 곧 변수라
   보일러플레이트와 함정(stale closure, useEffect 의존성 등)이 적다.
3. **React 생태계 이점이 이 앱에선 거의 활용되지 않는다** — 무거운 로직은 전부 Rust에 있고
   프론트는 얇다. 파일트리는 정책 배지 때문에 어차피 커스텀 구현이며, Diff는 Rust에서 계산 가능.
4. **Raw JSON은 현재 textarea 한 곳뿐** — Monaco의 syntax highlighting은 유용하지만
   worker/CSP/번들 비용과 실제 Windows WebView 검증이 필요하다. 현재 제품은 편집·검증·포맷
   계약을 가벼운 textarea로 제공하고 Monaco 도입을 별도 UX 개선으로 남긴다.

> React가 앞서는 유일한 축(생태계)은 이 프로젝트에서 거의 쓰이지 않고, Svelte의 장점(가벼움·단순함)은
> 매 순간 작동한다. 아키텍처가 프레임워크 비의존이라 향후 전환 비용도 프론트 계층에 한정된다.

---

## 4. 확정 시 영향

- `agent-guard/src/`를 SvelteKit(`src/routes`, `src/lib`, `svelte.config.js`+`adapter-static`)로 스캐폴딩.
- 컴포넌트: FileExplorer, PolicyEditor, EffectivePreview, RawJsonEditor, DiffViewer (ui-spec.md 매핑).
- `lib/{policy,settings,risk}.ts`는 Rust command의 얇은 타입 래퍼(로직은 Rust).
- `RawJsonEditor.svelte`는 현재 textarea를 사용하고 Rust command에 검증/포맷을 위임한다.
- Monaco를 도입한다면 동적 import, `worker-src` CSP, 키보드 접근성, Windows WebView2에서의
  worker 로딩을 먼저 검증해야 한다. 현재 의존성에는 Monaco가 포함되어 있지 않다.
