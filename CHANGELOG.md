# Changelog

Agent Guard의 사용자에게 보이는 변경 사항을 기록합니다. 형식은
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/)를 따르고 버전은
[Semantic Versioning](https://semver.org/)을 사용합니다.

## [Unreleased]

### Added

- SQLite에 저장한 정책 메타데이터, 선택한 리스크 프로필, 스캐너 무시 상태의 재로드 경로.
- Policy Editor의 Read/Edit 도구 선택, 사유, 위험도, 메모 입력.
- 저장·복원 경로 검증과 파일/DB 부분 실패에 대한 오류 처리.
- Windows managed settings file tier의 읽기 전용 병합과 오프라인 Claude 설정 스키마 검증.
- 프로젝트 MCP 승인 상태와 command/prompt/agent/http/mcp hook handler 위험 가시화.
- 좁은 창 레이아웃, 미저장 변경 이탈 경고, 모달 키보드 접근성.
- Dependabot(npm, Cargo, GitHub Actions)과 주간 RustSec advisory 검사.
- 보안 제보 정책, 개인정보를 보호하는 진단 절차, Windows 릴리스 체크리스트.
- 설정 내용과 실제 경로를 제외한 인앱 진단 정보 복사.

### Changed

- 이름 기반 스캔 결과를 기본 미선택인 경로 후보 검토로 전환. 항목별 도구·권한 선택과 기존 규칙 교체 확인을 요구.
- 검증되지 않은 보안 프로필·프로젝트 위험 점수·에이전트별 일괄 보안 프리셋을 UI에서 제외.
- 미리보기와 파일 트리가 도구별 동일한 Rust 규칙 비교기를 사용. 실제 실행 결과와 명시 규칙 비교를 구분.
- 복합 명령·별칭·미지원 패턴은 판정하지 않고, 규칙 없음도 Ask로 단정하지 않음.
- 문서의 구현 현황을 실제 Raw JSON textarea, Claude Code 범위, 패키징 상태와 일치시킴.

### Security

- 경로 규칙 저장 시 기존 `permissions.defaultMode`와 디스크의 도구 제한을 보존.
- 확장자 없는 SSH 개인키를 파일로 처리하고 scope별 미저장 변경을 보존.
- Diff 확인 뒤 외부에서 설정이 바뀌었으면 구조화 규칙 저장을 중단.
- 백업과 import/export IPC가 앱이 발급한 경로/식별자와 허용된 대상만 처리하도록 경계를 강화.

## [0.1.0] - TBD

첫 공개 프리릴리스 후보입니다. 아직 태그가 만들어지지 않았으며, 코드 서명과 깨끗한
Windows VM 승인 검사가 끝날 때까지 production-ready 릴리스로 간주하지 않습니다.

### Added

- Claude Code user/project/local 설정의 시각적 Allow/Ask/Deny 편집과 Effective Preview.
- 저장 전 diff, 자동 백업, 백업 미리보기·복원, Raw JSON 편집.
- 민감 경로 스캔, 리스크 점수, 프로필, 최근 프로젝트 목록.
- 정책 시뮬레이터, MCP/hooks 보안 가시화, 환경/secret 경고.
- Windows NSIS/MSI 및 portable EXE를 만드는 수동 GitHub Actions 프리릴리스 workflow.

[Unreleased]: https://github.com/geniuskey/AgentGuard/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/geniuskey/AgentGuard/releases/tag/v0.1.0
