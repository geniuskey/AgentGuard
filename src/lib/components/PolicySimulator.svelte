<script lang="ts">
  import { simulateAccess, type SimResult } from '$lib/ipc';
  import { app } from '$lib/state.svelte';

  let kind = $state<'path' | 'command'>('path');
  let shellTool = $state<'Bash' | 'PowerShell'>('PowerShell');
  let query = $state('');
  let result = $state<SimResult | null>(null);
  let error = $state<string | null>(null);
  let running = $state(false);

  const placeholder = $derived(
    kind === 'path' ? 'src/app.ts 또는 secrets/key.pem' : 'npm install 또는 curl https://…'
  );

  async function run() {
    const q = query.trim();
    if (!q) return;
    running = true;
    error = null;
    try {
      result = await simulateAccess(app.projectRoot, app.scoped, q, kind, shellTool);
    } catch (e) {
      error = String(e);
      result = null;
    } finally {
      running = false;
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter') run();
  }

  const decisionText: Record<string, string> = {
    allow: '허용됨',
    ask: '확인 요청 (Ask)',
    deny: '차단됨'
  };
</script>

<div class="sim">
  <div class="controls">
    <div class="kinds">
      <button class:active={kind === 'path'} onclick={() => (kind = 'path')}>경로</button>
      <button class:active={kind === 'command'} onclick={() => (kind = 'command')}>셸 명령</button>
    </div>
    {#if kind === 'command'}
      <select class="shell" bind:value={shellTool} aria-label="셸 도구">
        <option value="PowerShell">PowerShell</option>
        <option value="Bash">Bash</option>
      </select>
    {/if}
    <input
      type="text"
      bind:value={query}
      onkeydown={onKey}
      {placeholder}
      spellcheck="false"
      autocomplete="off"
    />
    <button class="run" onclick={run} disabled={running || !query.trim()}>
      {running ? '…' : '테스트'}
    </button>
  </div>

  <p class="basis">
    {#if kind === 'path'}
      현재 편집 중인 규칙 기준 (저장 전 변경 포함)
    {:else}
      저장된 설정 파일의 <b>{shellTool}</b> 규칙 기준{#if app.dirty}
        — 저장하지 않은 경로 규칙 변경은 반영되지 않음{/if}
    {/if}
  </p>

  {#if error}<p class="err" role="alert">{error}</p>{/if}

  {#if result}
    <div class="verdict v-{result.decision}">
      <span class="v-badge">{result.decision.toUpperCase()}</span>
      <span class="v-text">
        <code>{result.query}</code> → {decisionText[result.decision]}
      </span>
    </div>

    {#if result.fallback}
      <p class="note">
        일치하는 규칙 없음 — 기본 동작에 따라 실행 시 확인을 요청합니다.
      </p>
    {:else}
      <div class="matches">
        <b>일치한 규칙 {result.matches.length}개</b>
        {#each result.matches as m (m.scope + m.list + m.rule)}
          <div class="mrow" class:decisive={m.decisive}>
            <span class="pletter l-{m.list}">{m.list[0].toUpperCase()}</span>
            <code>{m.rule}</code>
            <span class="mscope">{m.scope}</span>
            {#if m.decisive}<span class="win">결정</span>{/if}
          </div>
        {/each}
        {#if result.matches.some((m) => !m.decisive)}
          <p class="note">deny &gt; ask &gt; allow 우선순위로 병합된 결과입니다.</p>
        {/if}
      </div>
    {/if}
  {:else if !error}
    <p class="muted">
      경로나 셸 명령을 입력하면 어떤 규칙이 적용되어 허용/확인/차단되는지 보여줍니다.
    </p>
  {/if}
</div>

<style>
  .sim {
    padding: 0.65rem;
    overflow: auto;
    flex: 1;
  }
  .controls {
    display: flex;
    gap: 0.4rem;
    align-items: center;
  }
  .kinds {
    display: flex;
    border: 1px solid var(--border-strong);
    border-radius: var(--r-sm);
    overflow: hidden;
    flex-shrink: 0;
  }
  .kinds button {
    background: transparent;
    border: none;
    color: var(--text-2);
    padding: 0.32rem 0.6rem;
    cursor: pointer;
    font-size: 0.74rem;
    white-space: nowrap;
    transition: background-color var(--t-fast), color var(--t-fast);
  }
  .kinds button.active {
    background: var(--accent-soft);
    color: var(--accent-text);
  }
  input {
    flex: 1;
    min-width: 0;
    background: var(--bg-1);
    border: 1px solid var(--border-strong);
    color: var(--text-1);
    border-radius: var(--r-sm);
    padding: 0.34rem 0.55rem;
    font-size: 0.78rem;
    font-family: var(--font-mono);
  }
  .shell {
    background: var(--bg-1);
    border: 1px solid var(--border-strong);
    color: var(--text-1);
    border-radius: var(--r-sm);
    padding: 0.34rem 0.45rem;
    font-size: 0.74rem;
  }
  input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .run {
    background: var(--accent-strong);
    border: 1px solid var(--accent-strong);
    color: white;
    border-radius: var(--r-sm);
    padding: 0.34rem 0.75rem;
    cursor: pointer;
    font-size: 0.76rem;
    font-weight: 600;
    flex-shrink: 0;
  }
  .run:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .basis {
    color: var(--text-3);
    font-size: 0.7rem;
    margin: 0.4rem 0 0.8rem;
  }
  .err {
    color: var(--deny);
    font-size: 0.78rem;
  }
  .verdict {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    border-radius: var(--r-sm);
    padding: 0.55rem 0.7rem;
    margin-bottom: 0.7rem;
    border: 1px solid var(--border);
  }
  .v-allow {
    background: var(--allow-soft);
    border-color: rgba(52, 211, 153, 0.35);
  }
  .v-ask {
    background: var(--ask-soft);
    border-color: rgba(251, 191, 36, 0.35);
  }
  .v-deny {
    background: var(--deny-soft);
    border-color: rgba(248, 113, 113, 0.35);
  }
  .v-badge {
    font-size: 0.68rem;
    font-weight: 800;
    letter-spacing: 0.06em;
    padding: 0.14rem 0.5rem;
    border-radius: 999px;
    flex-shrink: 0;
  }
  .v-allow .v-badge {
    background: var(--allow);
    color: #052e1b;
  }
  .v-ask .v-badge {
    background: var(--ask);
    color: #3b2a02;
  }
  .v-deny .v-badge {
    background: var(--deny);
    color: #3d0606;
  }
  .v-text {
    font-size: 0.8rem;
    color: var(--text-1);
    min-width: 0;
    word-break: break-all;
  }
  .v-text code {
    font-family: var(--font-mono);
    background: rgba(0, 0, 0, 0.2);
    padding: 0.05rem 0.35rem;
    border-radius: 4px;
  }
  .matches b {
    display: block;
    color: var(--text-2);
    font-size: 0.72rem;
    margin-bottom: 0.3rem;
  }
  .mrow {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    padding: 0.24rem 0.35rem;
    border-radius: 4px;
    font-size: 0.78rem;
  }
  .mrow.decisive {
    background: var(--bg-2);
    outline: 1px solid var(--border-strong);
  }
  .mrow code {
    font-family: var(--font-mono);
    font-size: 0.74rem;
    word-break: break-all;
    min-width: 0;
  }
  .mscope {
    margin-left: auto;
    color: var(--text-3);
    font-size: 0.66rem;
    text-transform: capitalize;
    flex-shrink: 0;
  }
  .win {
    color: var(--accent-text);
    background: var(--accent-soft);
    border-radius: 999px;
    padding: 0.03rem 0.45rem;
    font-size: 0.64rem;
    font-weight: 700;
    flex-shrink: 0;
  }
  .pletter {
    display: inline-grid;
    place-items: center;
    width: 1.15rem;
    height: 1.15rem;
    border-radius: 4px;
    font-size: 0.62rem;
    font-weight: 700;
    flex-shrink: 0;
  }
  .l-allow {
    background: var(--allow-soft);
    color: var(--allow);
  }
  .l-deny {
    background: var(--deny-soft);
    color: var(--deny);
  }
  .l-ask {
    background: var(--ask-soft);
    color: var(--ask);
  }
  .note {
    color: var(--text-3);
    font-size: 0.72rem;
    margin-top: 0.5rem;
  }
  .note code {
    font-family: var(--font-mono);
  }
  .muted {
    color: var(--text-3);
    font-size: 0.8rem;
  }
</style>
