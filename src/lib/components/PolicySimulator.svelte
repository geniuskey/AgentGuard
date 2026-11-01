<script lang="ts">
  import { simulateAccess, type SimResult } from '$lib/ipc';
  import { app } from '$lib/state.svelte';

  let pathTool = $state<'Read' | 'Edit'>('Read');
  let kind = $state<'path' | 'command'>('path');
  let shellTool = $state<'Bash' | 'PowerShell'>('PowerShell');
  let query = $state('');
  let result = $state<SimResult | null>(null);
  let error = $state<string | null>(null);
  let running = $state(false);

  const placeholder = $derived(
    kind === 'path' ? 'src/app.ts 또는 secrets/key.pem' : 'npm install 또는 curl https://…'
  );

  let requestId = 0;
  $effect(() => {
    query; kind; shellTool; pathTool; app.projectRoot; JSON.stringify(app.scoped);
    result = null;
    error = null;
    running = false;
    requestId += 1;
  });

  async function run() {
    const q = query.trim();
    if (!q) return;
    running = true;
    error = null;
    const id = ++requestId;
    try {
      const next = await simulateAccess(app.projectRoot, app.scoped, q, kind, shellTool, pathTool);
      if (id === requestId) result = next;
    } catch (e) {
      if (id === requestId) { error = String(e); result = null; }
    } finally {
      if (id === requestId) running = false;
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter') run();
  }

  const decisionText: Record<string, string> = {
    allow: '일치한 Allow 규칙',
    ask: '일치한 Ask 규칙',
    deny: '일치한 Deny 규칙'
  };
</script>

<div class="sim">
  <div class="controls">
    <div class="segmented" role="tablist" aria-label="검사 대상">
      <button role="tab" aria-selected={kind === 'path'} onclick={() => (kind = 'path')}>경로</button>
      <button role="tab" aria-selected={kind === 'command'} onclick={() => (kind = 'command')}>셸 명령</button>
    </div>
    {#if kind === 'path'}
      <select class="input tool" bind:value={pathTool} aria-label="파일 도구"><option value="Read">Read</option><option value="Edit">Edit</option></select>
    {/if}
    {#if kind === 'command'}
      <select class="input tool" bind:value={shellTool} aria-label="셸 도구">
        <option value="PowerShell">PowerShell</option>
        <option value="Bash">Bash</option>
      </select>
    {/if}
    <input
      class="input query"
      type="text"
      bind:value={query}
      onkeydown={onKey}
      {placeholder}
      aria-label={kind === 'path' ? '검사할 경로' : '검사할 셸 명령'}
      spellcheck="false"
      autocomplete="off"
    />
    <button class="btn btn-primary btn-sm" onclick={run} disabled={running || !query.trim()}>
      {running ? '검사 중…' : '규칙 비교'}
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

  {#if error}<p class="callout callout-error" role="alert">{error}</p>{/if}

  {#if result}
    <div class="verdict v-{result.decision ?? 'unknown'}">
      <span class="v-badge">{result.decision?.toUpperCase() ?? 'UNKNOWN'}</span>
      <span class="v-text">
        <code>{result.query}</code> → {result.decision ? decisionText[result.decision] : '판정하지 않음'}
      </span>
    </div>

    {#each result.notes as note}<p class="note">{note}</p>{/each}
    {#if result.fallback}
      <p class="note">
        일치하는 명시 규칙이 없습니다. 실제 허용·확인·거부는 도구와 실행 환경에 따라 달라집니다.
      </p>
    {:else}
      <div class="matches">
        <b>일치한 규칙 {result.matches.length}개</b>
        {#each result.matches as m (m.scope + m.list + m.rule)}
          <div class="mrow" class:decisive={m.decisive}>
            <span class="pletter l-{m.list}">{m.list[0].toUpperCase()}</span>
            <code>{m.rule}</code>
            <span class="mscope">{m.scope}</span>
            {#if m.decisive}<span class="chip chip-accent">결정</span>{/if}
          </div>
        {/each}
        {#if result.matches.some((m) => !m.decisive)}
          <p class="note">deny &gt; ask &gt; allow 우선순위로 병합된 결과입니다.</p>
        {/if}
      </div>
    {/if}
  {:else if !error}
    <p class="muted">
      경로나 단순 셸 명령에 일치하는 명시 규칙을 비교합니다. 실제 실행 결과를 보장하지 않습니다.
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
    flex-wrap: wrap;
    gap: 0.4rem;
    align-items: center;
  }
  .controls .segmented,
  .controls .btn,
  .tool {
    flex-shrink: 0;
  }
  .segmented > button {
    font-size: var(--fs-sm);
  }
  .tool {
    width: auto;
    padding: 0.3rem 0.45rem;
    font-size: var(--fs-sm);
  }
  .query {
    flex: 1 1 12rem;
    min-width: 0;
    padding: 0.32rem 0.55rem;
    font-family: var(--font-mono);
  }
  .basis {
    color: var(--text-3);
    font-size: var(--fs-xs);
    margin: 0.4rem 0 0.8rem;
  }
  .callout-error {
    margin-bottom: 0.7rem;
    word-break: break-all;
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
    border-color: var(--allow-line);
  }
  .v-ask {
    background: var(--ask-soft);
    border-color: var(--ask-line);
  }
  .v-deny {
    background: var(--deny-soft);
    border-color: var(--deny-line);
  }
  .v-badge {
    font-size: var(--fs-xs);
    font-weight: 800;
    letter-spacing: 0.06em;
    padding: 0.14rem 0.5rem;
    border-radius: 999px;
    background: var(--bg-3);
    color: var(--text-2);
    white-space: nowrap;
    flex-shrink: 0;
  }
  .v-allow .v-badge {
    background: var(--allow);
    color: var(--on-policy);
  }
  .v-ask .v-badge {
    background: var(--ask);
    color: var(--on-policy);
  }
  .v-deny .v-badge {
    background: var(--deny);
    color: var(--on-policy);
  }
  .v-text {
    font-size: var(--fs-md);
    color: var(--text-1);
    min-width: 0;
    word-break: break-all;
  }
  .v-text code {
    font-family: var(--font-mono);
    background: var(--bg-3);
    padding: 0.05rem 0.35rem;
    border-radius: var(--r-sm);
  }
  .matches b {
    display: block;
    color: var(--text-2);
    font-size: var(--fs-sm);
    margin-bottom: 0.3rem;
  }
  .mrow {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    padding: 0.24rem 0.35rem;
    border-radius: var(--r-sm);
    font-size: var(--fs-md);
  }
  .mrow.decisive {
    background: var(--bg-2);
    outline: 1px solid var(--border-strong);
  }
  .mrow code {
    font-family: var(--font-mono);
    font-size: var(--fs-sm);
    word-break: break-all;
    min-width: 0;
  }
  .mscope {
    margin-left: auto;
    color: var(--text-3);
    font-size: var(--fs-xs);
    text-transform: capitalize;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .mrow .chip {
    flex-shrink: 0;
  }
  .pletter {
    display: inline-grid;
    place-items: center;
    width: 1.15rem;
    height: 1.15rem;
    border-radius: var(--r-sm);
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
    font-size: var(--fs-sm);
    margin-top: 0.5rem;
  }
</style>
