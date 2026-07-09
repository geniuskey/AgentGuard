# Agent Guard

Agent Guard is a local-first desktop policy editor for coding agents.

It helps Windows users safely configure what Claude Code and other local coding
agents can access inside a project — without having to understand `settings.json`.

## Core Ideas

- Default Deny — allow only what is needed
- Local-only processing (no network, no telemetry)
- Visual project boundary editor (Explorer-style Allow / Ask / Deny)
- User / Project / Local settings support
- Effective access preview (merged result across scopes)
- Backup + diff before every save

## Status

Greenfield. This repository currently contains the **design & planning
documents** that make the v0.1 requirements executable. The stack is confirmed —
**Tauri 2 + SvelteKit (Svelte 5) + TypeScript + Rust + SQLite + Monaco**
(see `docs/tech-stack.md`) — and code scaffolding follows next.

## Development

Prerequisites: Node 18+, Rust (stable), and — for the desktop shell — the
[Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS.

```bash
npm install              # frontend deps

# Core logic (pure Rust, no WebView needed — runs anywhere)
cargo test -p agentguard-core

# Frontend (static SPA build + typecheck)
npm run build
npm run check

# Full desktop app (needs Tauri prerequisites; Windows is the primary target)
npm run tauri dev        # launches the window
npm run tauri build      # produces the installer
```

Layout: `crates/agentguard-core` holds all Tauri-independent logic (policy
conversion, risk scoring, storage) so it stays unit-testable on any host;
`src-tauri` is the thin Tauri 2 shell; `src` is the SvelteKit frontend.

## Documentation

| 문서 | 내용 |
|---|---|
| [docs/requirements.md](docs/requirements.md) | 원본 요구사항서 (v0.1) |
| [docs/architecture.md](docs/architecture.md) | 시스템 아키텍처, Rust 모듈, Tauri command 계약 |
| [docs/policy-model.md](docs/policy-model.md) | 중립 정책 모델 ↔ settings.json 변환 (팬아웃/왕복) |
| [docs/effective-policy.md](docs/effective-policy.md) | 병합 알고리즘, Default Deny 매핑, 충돌 탐지, Preview |
| [docs/data-model.md](docs/data-model.md) | SQLite 스키마, app-config, 백업 규칙 |
| [docs/risk-scanner.md](docs/risk-scanner.md) | 민감 경로 스캐너, 리스크 점수 함수 |
| [docs/security.md](docs/security.md) | 보안 원칙, 경고 규칙, Secret 감지 |
| [docs/ui-spec.md](docs/ui-spec.md) | 화면·상태·컴포넌트 명세 |
| [docs/tech-stack.md](docs/tech-stack.md) | 기술 스택 트레이드오프 & 권장안 (미확정) |
| [docs/roadmap.md](docs/roadmap.md) | 수직 슬라이스 우선 마일스톤 + 커버리지 추적표 |
| [docs/open-issues.md](docs/open-issues.md) | 오픈 이슈 결정 & 근거 |

## Key Design Decisions

- **경로 정책은 `Tool(specifier)`로 팬아웃** — Claude Code는 도구 중심 permission을
  사용하므로 "Allow `src/`"는 `Read/Edit/Write/Grep/Glob/NotebookEdit(./src/**)`로 확장된다.
- **Default Deny는 `defaultMode: "dontAsk"`로 구현** — catch-all `Deny(./**)`는
  allow-island을 덮어버리므로(deny 우선) 사용하지 않는다.
- **앱 메타데이터는 SQLite에만** — `settings.json`에는 순수 규칙만 기록하고 알 수 없는
  필드는 무손실 보존한다.
