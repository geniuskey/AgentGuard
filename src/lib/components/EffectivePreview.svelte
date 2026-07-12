<script lang="ts">
  import { toSettingsPreview, type Permissions } from '$lib/ipc';
  import { app } from '$lib/state.svelte';
  import PolicySimulator from './PolicySimulator.svelte';
  import AgentSurface from './AgentSurface.svelte';

  type Tab = 'allowed' | 'denied' | 'ask' | 'conflicts' | 'byscope' | 'raw' | 'simulate' | 'surface';
  let tab = $state<Tab>('allowed');

  const allowed = $derived(app.effective.filter((e) => e.effective === 'allow'));
  const denied = $derived(app.effective.filter((e) => e.effective === 'deny'));
  const asked = $derived(app.effective.filter((e) => e.effective === 'ask'));
  const conflicts = $derived(app.effective.filter((e) => e.conflict));
  const byScope = $derived(
    (['user', 'project', 'local'] as const).map((s) => ({
      scope: s,
      rules: app.scoped[s].rules,
      defaultMode: app.scoped[s].defaultMode
    }))
  );

  let raw = $state<Permissions | null>(null);
  // Recompute raw rules for the active scope whenever we switch to the Raw tab.
  $effect(() => {
    if (tab === 'raw') {
      toSettingsPreview(app.scoped[app.activeScope].rules).then((p) => (raw = p));
    }
  });

  const tabs: { id: Tab; label: string }[] = [
    { id: 'allowed', label: 'Allowed' },
    { id: 'denied', label: 'Denied' },
    { id: 'ask', label: 'Ask' },
    { id: 'conflicts', label: 'Conflicts' },
    { id: 'byscope', label: 'By Scope' },
    { id: 'raw', label: 'Raw Rules' },
    { id: 'simulate', label: 'Simulator' },
    { id: 'surface', label: 'MCP/Hooks' }
  ];
</script>

<div class="panel">
  <div class="tabs">
    {#each tabs as t (t.id)}
      <button class:active={tab === t.id} onclick={() => (tab = t.id)}>
        {t.label}
        {#if t.id === 'conflicts' && conflicts.length}<span class="cnt">{conflicts.length}</span>{/if}
      </button>
    {/each}
  </div>

  {#if tab === 'simulate'}
    <PolicySimulator />
  {:else if tab === 'surface'}
    <AgentSurface />
  {:else}
  <div class="body">
    {#if tab === 'allowed'}
      {#each allowed as e (e.path)}
        <div class="row"><span class="pd pd-allow" aria-hidden="true"></span>{e.path}</div>
      {:else}
        <p class="muted">없음</p>
      {/each}
    {:else if tab === 'denied'}
      {#each denied as e (e.path)}
        <div class="row">
          <span class="pd pd-deny" aria-hidden="true"></span>{e.path}
          <span class="src">{e.sourceScope ?? ''}</span>
        </div>
      {:else}
        <p class="muted">없음</p>
      {/each}
    {:else if tab === 'ask'}
      {#each asked as e (e.path)}
        <div class="row"><span class="pd pd-ask" aria-hidden="true"></span>{e.path}</div>
      {:else}
        <p class="muted">없음</p>
      {/each}
    {:else if tab === 'conflicts'}
      {#each conflicts as e (e.path)}
        <div class="row conflict">
          <span class="pd pd-conflict" aria-hidden="true"></span>{e.path} → {e.effective.toUpperCase()}
          <span class="src">deny 우선</span>
        </div>
      {:else}
        <p class="muted">충돌 없음</p>
      {/each}
    {:else if tab === 'byscope'}
      {#each byScope as g (g.scope)}
        <div class="scope-group">
          <b>{g.scope}{#if g.defaultMode} · defaultMode={g.defaultMode}{/if}</b>
          {#each g.rules as r (r.path + r.policy)}
            <div class="row">
              <span class="pletter l-{r.policy}">{r.policy[0].toUpperCase()}</span>{r.path}
            </div>
          {:else}
            <p class="muted">규칙 없음</p>
          {/each}
        </div>
      {/each}
    {:else if tab === 'raw'}
      {#if raw}
        <div class="raw-group"><b class="rk-allow">allow</b>{#each raw.allow as r}<code>{r}</code>{/each}</div>
        <div class="raw-group"><b class="rk-ask">ask</b>{#each raw.ask as r}<code>{r}</code>{/each}</div>
        <div class="raw-group"><b class="rk-deny">deny</b>{#each raw.deny as r}<code>{r}</code>{/each}</div>
        <p class="note">= {app.activeScope} scope에 저장될 실제 규칙</p>
      {:else}
        <p class="muted">계산 중…</p>
      {/if}
    {/if}
  </div>
  {/if}
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .tabs {
    display: flex;
    border-bottom: 1px solid var(--border);
    overflow-x: auto;
  }
  .tabs button {
    padding: 0.55rem 0.7rem;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--text-2);
    cursor: pointer;
    font-size: 0.78rem;
    white-space: nowrap;
    transition: color var(--t-fast), border-color var(--t-fast);
  }
  .tabs button:hover {
    color: var(--text-1);
  }
  .tabs button.active {
    color: var(--accent-text);
    border-bottom-color: var(--accent);
  }
  .cnt {
    background: var(--deny-soft);
    color: var(--deny);
    border: 1px solid rgba(248, 113, 113, 0.35);
    border-radius: 999px;
    padding: 0 0.38rem;
    font-size: 0.64rem;
    font-weight: 700;
    margin-left: 0.3rem;
    font-variant-numeric: tabular-nums;
  }
  .body {
    overflow: auto;
    padding: 0.55rem;
    flex: 1;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.8rem;
    font-family: var(--font-mono);
    padding: 0.22rem 0.3rem;
    border-radius: 4px;
    min-width: 0;
  }
  .row:hover {
    background: var(--bg-1);
  }
  .pd {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    flex-shrink: 0;
  }
  .pd-allow {
    background: var(--allow);
    box-shadow: 0 0 6px rgba(52, 211, 153, 0.6);
  }
  .pd-deny {
    background: var(--deny);
    box-shadow: 0 0 6px rgba(248, 113, 113, 0.6);
  }
  .pd-ask {
    background: var(--ask);
    box-shadow: 0 0 6px rgba(251, 191, 36, 0.6);
  }
  .pd-conflict {
    background: var(--deny);
    outline: 2px solid rgba(248, 113, 113, 0.3);
  }
  .row .src {
    margin-left: auto;
    color: var(--text-3);
    font-size: 0.68rem;
    font-family: var(--font-sans);
    flex-shrink: 0;
  }
  .row.conflict {
    color: var(--deny);
  }
  .muted {
    color: var(--text-3);
    font-size: 0.85rem;
  }
  .pletter {
    display: inline-grid;
    place-items: center;
    width: 1.15rem;
    height: 1.15rem;
    border-radius: 4px;
    font-size: 0.62rem;
    font-weight: 700;
    font-family: var(--font-sans);
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
  .raw-group {
    margin-bottom: 0.7rem;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .raw-group b {
    font-size: 0.66rem;
    font-weight: 700;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    width: fit-content;
    padding: 0.08rem 0.45rem;
    border-radius: 4px;
    margin-bottom: 0.15rem;
  }
  .rk-allow {
    background: var(--allow-soft);
    color: var(--allow);
  }
  .rk-ask {
    background: var(--ask-soft);
    color: var(--ask);
  }
  .rk-deny {
    background: var(--deny-soft);
    color: var(--deny);
  }
  code {
    background: var(--bg-1);
    border: 1px solid var(--border);
    padding: 0.18rem 0.45rem;
    border-radius: 4px;
    font-size: 0.74rem;
    word-break: break-all;
  }
  .note {
    color: var(--text-3);
    font-size: 0.72rem;
  }
  .scope-group {
    margin-bottom: 0.8rem;
  }
  .scope-group b {
    display: block;
    color: var(--text-2);
    font-size: 0.72rem;
    text-transform: capitalize;
    margin-bottom: 0.2rem;
  }
</style>
