# Security Policy

## 지원 범위

Agent Guard는 아직 `0.1.x` 프리릴리스 단계입니다. 보안 수정은 기본 브랜치와 가장 최근에
게시된 프리릴리스에 우선 적용합니다. 이전 프리릴리스에 별도 패치를 제공한다고 보장하지
않습니다.

| 버전 | 보안 수정 |
|---|---|
| 기본 브랜치 | 지원 |
| 최신 `0.1.x` 프리릴리스 | 최선 노력으로 지원 |
| 더 오래된 빌드 | 미지원; 최신 빌드로 업그레이드 필요 |

## 취약점 제보

공개 Issue에 취약점, 실제 `settings.json`, 토큰, 키, 사내 경로나 서버명을 올리지 마세요.
[GitHub private vulnerability reporting](https://github.com/geniuskey/AgentGuard/security/advisories/new)을
사용해 다음 내용을 보내 주세요.

- 영향받는 Agent Guard 버전과 설치 형태(NSIS/MSI/portable/dev)
- Windows 및 WebView2 버전
- 최소 재현 절차와 예상 영향
- 비밀 값과 개인/사내 식별자를 제거한 증거
- 알고 있다면 임시 완화책

접수 후 공개 시점과 수정 범위를 제보자와 조율합니다. 공개 전에 취약점 세부 정보를
일반 Issue나 토론에 게시하지 말아 주세요. 저장소에서 private reporting이 활성화되지 않아
링크를 사용할 수 없다면, 세부 정보 없이 “비공개 보안 연락 채널이 필요하다”는 Issue만 열어
maintainer의 안내를 기다려 주세요.

## 보안 경계

Agent Guard 데스크톱 앱은 로컬 설정 파일과 `%APPDATA%\AgentGuard` 아래의 SQLite/백업을
처리하며 텔레메트리나 프로젝트 업로드 기능을 두지 않습니다. 다만 별도의 개발자용
`test:claude-permissions` 명령은 사용자가 이미 인증한 Claude Code CLI를 명시적으로 실행하며
Claude API 사용과 네트워크 통신이 발생할 수 있습니다. 이 스크립트는 앱의 런타임 기능이
아닙니다.

Claude Code의 permission 규칙은 OS 샌드박스나 방화벽을 대체하지 않습니다. 특히 허용된
shell 명령과 하위 프로세스의 파일/네트워크 접근까지 강제 차단한다고 가정하면 안 됩니다.
자세한 위협 경계와 로컬 데이터 목록은
[`docs/security.md`](docs/security.md)와
[`docs/diagnostics-and-privacy.md`](docs/diagnostics-and-privacy.md)를 참고하세요.
