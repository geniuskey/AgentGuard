# 릴리스·업데이트·롤백 정책

## 1. 릴리스 채널

- `0.x` GitHub prerelease: 평가와 호환성 검증용. 데이터 형식과 설치 동작이 바뀔 수 있다.
- Stable: 아래 릴리스 체크리스트, 코드 서명, 업그레이드/롤백 검증을 모두 통과한 빌드만 사용한다.
- 산출물은 NSIS setup EXE, MSI, portable EXE이며 한 릴리스 안에서 버전과 커밋이 같아야 한다.

현재 `.github/workflows/release.yml`은 `main`에서 수동으로 실행하는 **서명되지 않은 prerelease**
workflow다. 자동 updater manifest를 만들지 않으며(`uploadUpdaterJson: false`), production 배포
workflow로 간주하지 않는다.

## 2. Windows 코드 서명 전제 조건

production 릴리스를 만들기 전에 다음이 필요하다.

1. 조직 이름과 일치하는 승인된 Authenticode 인증서(EV/OV 또는 조직의 managed signing)와
   인증서 소유자 승인.
2. 개인 키를 저장소나 일반 GitHub secret에 평문으로 넣지 않는 서명 방식. HSM/managed
   signing 또는 접근이 제한된 CI certificate store를 우선한다.
3. RFC 3161 timestamp 서비스와 조직이 승인한 digest 알고리즘(일반적으로 SHA-256).
4. fork/일반 PR에서는 서명 자격 증명에 접근할 수 없고, 보호된 tag/environment의 승인된
   release job에서만 접근하도록 한 GitHub Environment 규칙.
5. EXE, NSIS installer, MSI 각각의 서명 및 timestamp 검증. Windows SDK가 있는 깨끗한
   머신에서 `Get-AuthenticodeSignature`와 `signtool verify /pa /all /v`가 성공해야 한다.

Tauri updater의 artifact signature와 Windows Authenticode는 서로 다른 목적이다. updater를
도입할 경우의 Tauri private key가 Authenticode 인증서를 대신하지 않는다.

## 3. 현재 업데이트 정책: 수동

앱에는 자동 업데이트 확인이나 다운로드 기능이 없다. 사용자는 GitHub Releases에서 새 버전의
release notes와 checksum/signature를 확인하고, 현재 설치 채널과 같은 installer를 내려받아
직접 업그레이드한다. portable 사용자는 앱 종료 후 새 EXE로 교체한다.

업그레이드 전에 다음을 수행한다.

1. Agent Guard를 종료한다.
2. `%APPDATA%\AgentGuard`와 중요 프로젝트의 `.claude` 설정을 사용자만 접근 가능한 위치에
   복사한다.
3. release notes에서 DB migration, 최소 Windows/WebView2 버전, 알려진 문제를 확인한다.
4. 설치 후 기존 프로젝트·프로필·메타데이터·백업 목록과 Claude 설정 파일이 유지되는지 확인한다.

향후 자동 updater는 신뢰할 수 있는 HTTPS endpoint, Tauri updater signing key의 안전한 보관,
서명된 manifest/artifact, 채널 분리, 실패 시 원자적 교체가 준비된 뒤 별도 위협 모델과 함께
활성화한다.

## 4. 롤백

릴리스 이전 버전으로 돌아갈 수 있도록 최소 한 개의 직전 승인 artifact와 checksum/signature를
보존한다. 롤백은 앱 종료 → `%APPDATA%\AgentGuard` 복사 → 현재 버전 제거 → 직전 같은 채널의
버전 설치 순서다.

DB schema downgrade 호환성은 아직 보장하지 않는다. 새 버전이 DB migration을 수행했다면 이전
버전 실행 전에 **업그레이드 전 복사본 전체를 복원**해야 한다. 데이터 손실형 migration은
자동 롤백과 복원 절차가 릴리스에서 검증되기 전까지 허용하지 않는다. 설치 제거는 기본적으로
`%APPDATA%\AgentGuard`와 프로젝트의 `.claude` 파일을 보존해야 하며, 실제 동작을 릴리스마다
검증한다.

## 5. 릴리스 기록

각 릴리스는 `CHANGELOG.md`, commit SHA, 산출물 checksum, 서명 주체/timestamp, CI 링크,
[`release-checklist.md`](release-checklist.md)의 VM 테스트 증거와 알려진 제한을 남긴다.
