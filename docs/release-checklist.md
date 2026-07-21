# Windows 릴리스 체크리스트

릴리스마다 새 Windows VM snapshot에서 수행하고, 결과를 릴리스 PR 또는 첨부된 테스트 기록에
남긴다. 체크하지 못한 항목은 “통과”가 아니라 blocker 또는 알려진 제한으로 기록한다.

## 1. 빌드 출처와 자동 검사

- [ ] release commit이 보호된 `main`에 있고 worktree가 깨끗하다.
- [ ] `package.json`, lockfile, Cargo workspace, `tauri.conf.json` 버전이 일치한다.
- [ ] Frontend test/check/build, Rust fmt/clippy/test, docs build, npm audit, RustSec audit가 통과한다.
- [ ] release workflow가 동일 commit에서 NSIS, MSI, portable x64 artifact를 만들었다.
- [ ] 각 artifact의 SHA-256, 크기, commit SHA를 기록하고 release notes/CHANGELOG를 검토했다.
- [ ] production이면 Authenticode signer와 timestamp를 세 artifact 모두에서 검증했다.
- [ ] 서명되지 않은 빌드는 prerelease로만 표시되고 SmartScreen 경고 가능성을 명시한다.
- [ ] GitHub private vulnerability reporting이 활성화되어 `SECURITY.md` 링크가 실제로 열리고,
  Dependabot/RustSec 알림을 받을 maintainer가 지정되어 있다.

## 2. 깨끗한 VM 조합

지원 대상 Windows 버전별로 snapshot을 되돌려 아래 조합을 각각 검사한다.

| 조합 | 확인 사항 |
|---|---|
| Standard user + NSIS | 관리자 권한 없이 설치/실행 가능한지, elevation이 필요하면 정확히 문서화 |
| Standard user + MSI | 설치 범위와 elevation 동작, repair/uninstall |
| Administrator + NSIS/MSI | 설치 위치, Start menu, Apps & Features, 중복 설치 방지 |
| Portable | 쓰기 가능한 폴더와 읽기 전용 폴더에서 실행 결과, AppData 위치 |
| WebView2 최신 | 첫 실행, 기본 화면, devtools 없이 정상 렌더링 |
| WebView2 없음/오래됨 | installer의 bootstrap/offline 동작과 사용자 오류 안내; 강제 설치 정책 기록 |
| 네트워크 차단 | 앱 핵심 편집 기능이 네트워크 없이 동작하는지 확인 |

## 3. 설치·업그레이드·제거·롤백

- [ ] 새 VM에 NSIS 설치 → 실행 → 종료 → 제거. 프로그램 파일/shortcut은 제거되고
  `%APPDATA%\AgentGuard`와 프로젝트 설정은 정책대로 보존된다.
- [ ] snapshot을 되돌려 MSI 설치 → repair → 실행 → 제거를 같은 기준으로 확인한다.
- [ ] `N-1`에서 최근 프로젝트, 프로필, 메타데이터, dismissed finding, 백업을 만든 뒤 `N`으로
  in-place 업그레이드하고 모두 다시 로드되는지 확인한다.
- [ ] 업그레이드 도중 앱 실행, 파일 잠금, 디스크 부족을 각각 재현해 원본과 이전 실행본이
  손상되지 않는지 확인한다.
- [ ] [`release-policy.md`](release-policy.md)의 AppData 복사본으로 `N-1` 롤백을 수행하고,
  DB migration이 있었다면 migration 전 복사본 복원이 실제로 동작하는지 확인한다.

## 4. 경로·파일 시스템 매트릭스

각 위치에서 프로젝트 열기 → 스캔 → Allow/Deny 편집 → Diff → 저장 → 앱 재실행 → 복원까지
수행한다. 경로가 지원되지 않으면 조용히 오동작하지 않고 변경 없이 명확한 오류를 보여야 한다.

| 경로/조건 | 예시 또는 준비 | 합격 기준 |
|---|---|---|
| 한글/공백 | `C:\테스트 사용자\내 프로젝트` | 표시·저장·백업·재로드에서 경로 손실 없음 |
| 긴 경로 | 전체 경로 260자 초과, OS long paths on/off | 지원 여부가 일관되고 실패 시 원본 유지 |
| OneDrive | 동기화 폴더, online-only/로컬 고정 파일 | placeholder·동기화 충돌 시 원본 유지와 오류 표시 |
| UNC | `\\server\share\프로젝트` 및 연결 끊김 | 지원 범위를 기록하고 연결 손실 시 hang/부분 저장 없음 |
| 읽기 전용 | 파일 ACL read-only와 폴더 쓰기 거부 | 권한 오류를 “파일 없음”으로 오인하지 않고 원본 유지 |
| 외부 변경 | 앱이 dirty인 동안 editor로 JSON 변경 | 덮어쓰기 전에 충돌을 알리고 사용자 선택을 요구 |
| 손상 JSON | 구문 오류가 있는 각 scope 설정 | 저장 차단, 원문/백업 접근 가능 |

WSL path 변환은 현재 지원 범위 밖이다. Windows에서 `\\wsl$` 경로를 UNC와 별개로 시험하고,
지원한다고 공표하려면 별도의 변환·왕복 테스트가 필요하다.

## 5. 핵심 사용자 흐름

- [ ] User/Project/Local 및 managed file scope를 불러오고 effective 결과에서 Deny 우선순위를 확인한다.
- [ ] 규칙의 reason/risk level/notes, 선택 프로필, scanner dismiss가 재실행 후 유지된다.
- [ ] Raw JSON → 시각 편집 → Raw JSON 왕복에서 알 수 없는 키가 보존되고 잘못된 JSON 저장은 차단된다.
- [ ] 백업 목록은 프로젝트 경계를 벗어난 파일을 열거나 복원할 수 없다.
- [ ] 홈/창 닫기/탭 전환에서 dirty 변경을 버릴 때 확인하며 Cancel이 동작한다.
- [ ] 키보드만으로 주요 동작, modal focus trap/Escape/초점 복귀가 동작한다.
- [ ] 800px 최소 창과 100%, 125%, 200% 배율에서 주요 control이 잘리거나 겹치지 않는다.
- [ ] `npm run test:claude-permissions`는 별도 승인된 개발 머신에서만 실행하고 앱 smoke test와
  혼동하지 않는다.

## 6. 승인 기록

- Release/tag/commit:
- VM image 및 Windows build:
- WebView2 version:
- Installer별 결과와 로그 링크:
- Path matrix 증거:
- Signer/timestamp/checksum:
- 발견된 제한과 release notes 링크:
- 검증자 / 날짜:
- 최종 승인자 / 날짜:
