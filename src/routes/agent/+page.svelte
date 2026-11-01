<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { getAgentGlobal, inTauri, type AgentGlobal } from '$lib/ipc';
  import AgentConfigEditor from '$lib/components/AgentConfigEditor.svelte';
  import HelpButton from '$lib/components/HelpButton.svelte';

  let agent = $state<AgentGlobal | null>(null);
  let error = $state<string | null>(null);

  const id = $derived($page.url.searchParams.get('id') ?? '');

  $effect(() => {
    const currentId = id;
    agent = null;
    error = null;
    if (!currentId) {
      error = '에이전트 id가 지정되지 않았습니다.';
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
    <button class="btn btn-ghost btn-icon" onclick={() => goto('/')} aria-label="홈으로" title="홈으로">←</button>
    <div class="title">
      {#if agent}{agent.name}{:else}에이전트 설정{/if}
    </div>
    {#if agent}<code class="path">{agent.path}</code>{/if}
    <span class="help-slot"><HelpButton section="agent" compact /></span>
  </div>

  {#if agent}
    <p class="tagline">{agent.description} · 전역 설정. 저장 시 자동으로 백업됩니다.</p>
  {/if}

  {#if !inTauri()}
    <p class="callout callout-warn notice">데스크톱 앱에서만 편집할 수 있습니다 (npm run tauri dev).</p>
  {/if}
  {#if error}<div class="callout callout-error notice" role="alert">{error}</div>{/if}

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
  .help-slot {
    margin-left: auto;
  }
  .title {
    font-weight: 600;
    font-size: 1.05rem;
    letter-spacing: -0.01em;
    white-space: nowrap;
  }
  .path {
    background: var(--bg-1);
    border: 1px solid var(--border);
    padding: 0.15rem 0.55rem;
    border-radius: 4px;
    font-size: var(--fs-sm);
    color: var(--text-2);
    word-break: break-all;
    min-width: 0;
  }
  .tagline {
    color: var(--text-2);
    font-size: var(--fs-md);
    margin: 0.6rem 0;
  }
  .notice {
    margin: 0.6rem 0 0.4rem;
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
