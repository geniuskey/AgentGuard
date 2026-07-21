# Security Design

> 근거: 요구사항서 9.1(보안), 14장(보안 경고 규칙), 18장(보안 설계 원칙).
> AWS Bedrock 도우미는 요구사항서 8.13, `docs/ui-spec.md`(Bedrock 화면)와 연계.

---

## 1. 보안 설계 원칙 (요구사항서 18장 — 구현 규칙화)

1. 앱은 프로젝트 내용을 **외부로 전송하지 않는다**. 네트워크 코드 자체를 두지 않는다(Local First).
2. 앱은 **Secret 값을 수집·저장하지 않는다**. DB에는 정책 메타데이터만(D3, data-model.md).
3. settings.json 저장 전 **자동 백업**(backup.rs).
4. 저장 전 **Diff** 표시.
5. Project settings에는 **공유 가능한 정보만** 저장하도록 안내/경고.
6. Local settings는 개인 환경 설정용으로 유도 + `.gitignore` 등록 권장.
7. 위험한 Allow 규칙은 **저장 전 경고**.
8. **Deny 규칙은 UI에서 가장 강하게 표시**(빨강/굵게).
9. 앱 내부 DB에는 메타데이터만.
10. 원본 설정 파일의 **알 수 없는 필드 보존**(D5).

---

## 2. Project settings 저장 경고 트리거 (요구사항서 14장)

Project scope에 저장하려는 값에 아래가 포함되면 **저장 전 경고**(개인·기밀 정보의 저장소 공유 방지):

```
개인 경로:   C:\Users\<username>\ ,  D:\private\ ,  절대 홈 경로
Secret 키워드: AWS_SECRET_ACCESS_KEY, AWS_SESSION_TOKEN,
              password, passwd, token, secret, api_key, private_key
```
- 탐지 방식: 저장 직전 최종 JSON 문자열 + 규칙 경로를 정규식/키워드로 스캔.
- 경고는 차단이 아니라 **명시적 확인**(사용자가 의도적으로 진행 가능). 단 기본은 "취소" 강조.

---

## 3. 위험한 정책 경고 (요구사항서 14장)

아래 Allow 규칙은 저장 전 강조 경고:
```
Allow 프로젝트 루트 /**
Allow raw/**    Allow data/**    Allow secrets/**
Allow .env      Allow *.pem      Allow *.key
```

경고 문구(요구사항서 예시):
```
이 설정은 프로젝트 전체를 에이전트에게 허용합니다.
사내 보안 환경에서는 필요한 폴더만 Allow하는 것을 권장합니다.
```

> "루트 전체 Allow"는 리스크 점수 +50과도 연동(risk-scanner.md).

---

## 4. Secret 감지 경고 (AWS Bedrock, 요구사항서 8.13)

MVP는 **읽기/경고 중심**, Secret 저장 기능 없음.

- 감지 대상 환경변수(존재 여부만 표시, 값은 마스킹):
  `AWS_REGION`, `AWS_DEFAULT_REGION`, `AWS_PROFILE`, `AWS_ACCESS_KEY_ID`,
  `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN`, `HTTPS_PROXY`, `HTTP_PROXY`,
  `NO_PROXY`, `REQUESTS_CA_BUNDLE`, `SSL_CERT_FILE`.
- `AWS_PROFILE` 사용 권장.
- `AWS_SECRET_ACCESS_KEY`를 settings.json에 직접 저장하려 하면 **강하게 경고**:
```
AWS_SECRET_ACCESS_KEY를 settings.json에 직접 저장하지 않는 것을 권장합니다.
가능하면 AWS_PROFILE 또는 사내 인증 체계를 사용하세요.
```

---

## 5. `.gitignore` 권장 (요구사항서 7.3, 9.1)

- Local settings(`.claude/settings.local.json`)는 개인 전용 → `.gitignore` 등록 권장.
- 프로젝트를 열 때 `.gitignore`에 해당 항목이 없으면 "추가 추천"을 제안(Should-have).

---

## 6. 안정성 보증 (요구사항서 9.4)

- 잘못된 JSON 저장 방지(validate 후에만 쓰기).
- 파일 쓰기 실패 시 원본 보존(임시 파일 → 원자적 rename).
- 백업 실패 시 저장 중단 + 경고.
- 설정 파일 손상 시 백업에서 복구 가능.

---

## 7. Claude Code 권한의 보안 경계

- `allow`는 일치한 도구 호출의 **사전 승인**이다. 미등록 호출을 제거하는 화이트리스트가
  아니며, 기본 모드에서는 일치 규칙이 없으면 사용자에게 확인을 요청한다.
- `Read(...)`·`Edit(...)` Deny는 Claude Code 내장 파일 도구에 적용된다. 허용된
  `Bash(...)`·`PowerShell(...)` 명령과 그 하위 프로세스의 OS 파일 접근은 차단하지 않는다.
- 민감 파일을 프로세스 수준에서도 강제 격리해야 하는 환경은 셸 allow를 최소화하고
  Claude Code sandbox 또는 별도의 OS sandbox/ACL을 함께 사용해야 한다.
- 웹 접근 제한 토글은 Claude 웹 도구와 Bash/PowerShell의 대표 HTTP 클라이언트를
  차단하는 방어 계층이다. 임의 Python/Node 프로세스나 새 네트워크 도구까지 포괄하는
  방화벽으로 표현하지 않으며, 일부 규칙만 존재하면 UI에서 `PARTIAL`로 표시한다.
- Claude가 아직 신뢰하지 않은 저장소에서는 공유 `.claude/settings.json`의 Allow가
  무시될 수 있다. Agent Guard는 `~/.claude.json`에서 해당 프로젝트의 trust boolean만
  읽어 경고하며, 신뢰를 대신 승인하거나 다른 Claude 상태를 노출하지 않는다. Deny는
  미신뢰 상태에서도 보수적으로 적용된다.
