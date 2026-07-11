// Human explanations for permission patterns, shown as tooltips. Ordinary users
// can't read `~/.ssh/**` — say what the folder/file is and what the glob covers.

const KNOWN: [RegExp, string][] = [
  // Credential / secret locations.
  [/(^|\/)\.ssh(\/|$)/, 'SSH 원격 접속 개인키 폴더 — 유출되면 내 서버에 그대로 로그인할 수 있습니다'],
  [/(^|\/)\.aws(\/|$)/, 'AWS 클라우드 자격증명(액세스 키) 폴더'],
  [/(^|\/)gcloud(\/|$)/, 'Google Cloud 자격증명·설정 폴더'],
  [/(^|\/)\.azure(\/|$)/, 'Azure 클라우드 자격증명 폴더'],
  [/(^|\/)\.kube(\/|$)/, 'Kubernetes 클러스터 접속 설정(kubeconfig) 폴더'],
  [/(^|\/)\.gnupg(\/|$)/, 'GPG 암호화·서명 개인키 폴더'],
  [/(^|\/)\.docker(\/|$)/, 'Docker 로그인 토큰이 저장되는 폴더'],
  [/(^|\/)\.config\/gh(\/|$)/, 'GitHub CLI 로그인 토큰 폴더'],
  [/(^|\/)\.password-store(\/|$)/, 'pass 비밀번호 저장소'],
  [/(^|\/)\.netrc$/, '원격 서버 로그인 계정·암호가 평문으로 저장되는 파일'],
  [/(^|\/)\.npmrc$/, 'npm 패키지 저장소 인증 토큰 파일'],
  [/(^|\/)\.pypirc$/, 'PyPI(파이썬 패키지) 배포 인증 파일'],
  [/(^|\/)\.git-credentials$/, 'Git 원격 저장소 암호·토큰이 평문으로 저장되는 파일'],
  [/(^|\/)\.env(\.[\w.-]+)?$/, '환경 변수 파일 — API 키·DB 암호가 흔히 저장됩니다'],
  [/(\*|\.)pem$/, 'PEM 인증서/개인키 파일'],
  [/(\*|\.)key$/, '암호화 개인키 파일'],
  [/(\*|\.)(pfx|p12)$/, '인증서와 개인키 묶음 파일(PKCS#12)'],
  [/(^|\/)id_(rsa|ed25519|ecdsa|dsa)(\.pub)?$/, 'SSH 접속용 개인키 파일'],
  [/(^|\/)(secrets?|credentials?)(\/|\.|$)/i, '이름상 비밀정보·자격증명으로 추정되는 항목'],
  // Agent / tool config.
  [/(^|\/)\.claude(\/|$)/, 'Claude Code 전역 설정 폴더'],
  [/(^|\/)\.codex(\/|$)/, 'Codex CLI 전역 설정 폴더'],
  [/(^|\/)\.gemini(\/|$)/, 'Gemini CLI 전역 설정 폴더'],
  // Common system / project folders (helps non-developers in the explorer).
  [/(^|\/)Windows(\/|$)/, 'Windows 시스템 폴더 — 수정하면 시스템이 손상될 수 있습니다'],
  [/(^|\/)Program Files( \(x86\))?(\/|$)/, '설치된 프로그램 폴더'],
  [/(^|\/)AppData(\/|$)/, '프로그램별 설정·데이터가 저장되는 숨김 폴더'],
  [/(^|\/)node_modules(\/|$)/, '설치된 패키지(의존성) 폴더 — 직접 수정하지 않는 곳'],
  [/(^|\/)\.git(\/|$)/, 'Git 저장소 내부 데이터 폴더'],
  [/(^|\/)Documents(\/|$)/, '내 문서 폴더'],
  [/(^|\/)Desktop(\/|$)/, '바탕 화면 폴더'],
  [/(^|\/)Downloads(\/|$)/, '다운로드 폴더']
];

/** What this folder/file is, if it's a well-known location; null otherwise. */
export function knownDescription(pattern: string): string | null {
  for (const [re, desc] of KNOWN) if (re.test(pattern)) return desc;
  return null;
}

/**
 * Full tooltip: known meaning (if any) + what the glob syntax covers.
 * `group: true` describes a path prefix (tree group row) — location only,
 * no rule-coverage line. Returns '' when there is nothing useful to say.
 */
export function describePattern(pattern: string, opts?: { group?: boolean }): string {
  const lines: string[] = [];
  const known = knownDescription(pattern);
  if (known) lines.push(known);

  if (pattern === '~' || pattern.startsWith('~/')) lines.push('~ = 내 홈 폴더');
  else if (pattern.startsWith('//**')) lines.push('//** = PC의 모든 드라이브');
  else {
    const m = pattern.match(/^\/\/([a-z])(\/|$)/i);
    if (m) lines.push(`//${m[1]} = ${m[1].toUpperCase()}: 드라이브`);
  }

  if (!opts?.group) {
    if (pattern.endsWith('/**')) lines.push('끝의 ** = 이 위치의 모든 하위 폴더·파일 포함');
    else if (/[*?]/.test(pattern)) lines.push('* = 이 모양과 이름이 일치하는 모든 항목');
    else lines.push('이 경로 하나에만 적용');
  }

  return lines.join('\n');
}
