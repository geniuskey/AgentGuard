<script lang="ts">
  function back() {
    history.back();
  }
</script>

<main>
  <button class="back" onclick={back}>← Back</button>
  <h1>📖 Agent Guard 가이드</h1>

  <section>
    <h2>핵심 개념</h2>
    <ul>
      <li><b>Allow / Ask / Deny</b> — 에이전트가 경로에 접근할 때 각각 허용 / 확인 후 허용 / 차단합니다.</li>
      <li><b>Default Deny</b> — 켜면(<code>dontAsk</code>) 명시적으로 Allow한 경로 외에는 모두 차단됩니다. 사내 보안 환경 권장.</li>
      <li><b>Scope</b> — Local &gt; Project &gt; User 순으로 우선하며, <b>Deny는 어느 Scope에서든 최우선</b>입니다.</li>
    </ul>
  </section>

  <section>
    <h2>기본 흐름</h2>
    <ol>
      <li>Home에서 프로젝트를 엽니다. 위험 경로가 자동 스캔되고 리스크 점수가 계산됩니다.</li>
      <li>왼쪽 트리에서 경로를 고르고, 가운데 Policy Editor에서 정책을 지정합니다.</li>
      <li>오른쪽 Effective Preview에서 3개 설정이 병합된 최종 접근 결과를 확인합니다.</li>
      <li>Save를 누르면 <b>변경 Diff</b>를 확인하고, 기존 파일은 <b>자동 백업</b> 후 저장됩니다.</li>
    </ol>
  </section>

  <section>
    <h2>키보드 단축키 (Explorer)</h2>
    <ul>
      <li><kbd>Ctrl/⌘</kbd>+<kbd>S</kbd> — 저장 다이얼로그</li>
      <li><kbd>A</kbd> / <kbd>K</kbd> / <kbd>D</kbd> — 선택 경로를 Allow / Ask / Deny (this folder and children)</li>
    </ul>
  </section>

  <section>
    <h2>안전장치</h2>
    <ul>
      <li>모든 처리는 로컬에서만 이루어지며, 앱은 Secret 값을 저장·전송하지 않습니다.</li>
      <li>Project 설정에 개인 경로/Secret이 들어가면 경고합니다.</li>
      <li><code>settings.local.json</code>은 <code>.gitignore</code> 등록을 권장합니다.</li>
      <li>settings.json의 알 수 없는 필드와 수기 규칙은 저장 시 그대로 보존됩니다.</li>
    </ul>
  </section>
</main>

<style>
  main { max-width: 760px; margin: 0 auto; padding: 2rem 1.5rem; }
  .back { background: none; border: 1px solid #334155; color: #94a3b8; border-radius: 6px; padding: 0.3rem 0.6rem; cursor: pointer; }
  h1 { font-size: 1.5rem; margin: 1rem 0; }
  h2 { font-size: 1.05rem; border-bottom: 1px solid #1e293b; padding-bottom: 0.3rem; margin-top: 1.5rem; }
  li { margin: 0.3rem 0; line-height: 1.5; }
  code { background: #0b1220; padding: 0.1rem 0.35rem; border-radius: 4px; font-size: 0.85em; }
  kbd { background: #1e293b; border: 1px solid #334155; border-radius: 4px; padding: 0.05rem 0.35rem; font-size: 0.8em; }
</style>
