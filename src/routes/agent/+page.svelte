<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { getAgentGlobal, inTauri, type AgentGlobal } from '$lib/ipc';
  import AgentConfigEditor from '$lib/components/AgentConfigEditor.svelte';

  let agent = $state<AgentGlobal | null>(null);
  let error = $state<string | null>(null);

  const id = $derived($page.url.searchParams.get('id') ?? '');

  $effect(() => {
    const currentId = id;
    agent = null;
    error = null;
    if (!currentId) {
      error = 'agent id가 지정되지 않았습니다.';
      return;
    }
    if (!inTauri()) return;
    getAgentGlobal(currentId)
      .then((a) => {
        // Agents with a dedicated route (Claude Code → /user) go there instead.
        if (!a.route.startsWith('/agent')) {
          goto(a.route);
          return;
        }
        agent = a;
      })
      .catch((e) => (error = String(e)));
  });
</script>

<main>
  <div class="top">
    <button class="back" onclick={() => goto('/')} aria-label="홈으로">←</button>
    <div class="title">
      {#if agent}{agent.name}{:else}에이전트 설정{/if}
    </div>
    {#if agent}<code class="path">{agent.path}</code>{/if}
  </div>

  {#if agent}
    <p class="tagline">{agent.description} · 전역 설정. 저장 시 자동으로 백업됩니다.</p>
  {/if}

  {#if !inTauri()}
    <p class="hint">데스크톱 앱에서만 편집할 수 있습니다 (npm run tauri dev).</p>
  {/if}
  {#if error}<div class="err" role="alert">{error}</div>{/if}

  {#if agent}
    <div class="editor">
      <AgentConfigEditor {agent} />
    </div>
  {/if}
</main>

<style>
  main {
    height: 100vh;
    display: flex;
    flex-direction: column;
    padding: 0.8rem 1rem;
    box-sizing: border-box;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    flex-wrap: wrap;
  }
  .back {
    background: none;
    border: 1px solid var(--border-strong);
    color: var(--text-2);
    border-radius: var(--r-sm);
    padding: 0.28rem 0.6rem;
    cursor: pointer;
    transition: color var(--t-fast), background-color var(--t-fast);
  }
  .back:hover {
    color: var(--text-1);
    background: var(--bg-2);
  }
  .title {
    font-weight: 600;
    font-size: 1.05rem;
    letter-spacing: -0.01em;
  }
  .path {
    background: var(--bg-1);
    border: 1px solid var(--border);
    padding: 0.15rem 0.55rem;
    border-radius: 4px;
    font-size: 0.76rem;
    color: var(--text-2);
    word-break: break-all;
  }
  .tagline {
    color: var(--text-2);
    font-size: 0.85rem;
    margin: 0.6rem 0;
  }
  .hint {
    color: var(--ask);
    background: var(--ask-soft);
    border-radius: var(--r-sm);
    padding: 0.35rem 0.6rem;
    font-size: 0.8rem;
    margin: 0.6rem 0 0.4rem;
  }
  .err {
    background: var(--deny-soft);
    border: 1px solid rgba(248, 113, 113, 0.3);
    color: var(--deny);
    padding: 0.4rem 0.8rem;
    font-size: 0.8rem;
    border-radius: var(--r-sm);
    margin-bottom: 0.4rem;
  }
  .editor {
    flex: 1;
    min-height: 0;
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    overflow: hidden;
    background: var(--bg-0);
  }
</style>
