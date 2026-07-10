<script lang="ts">
  import { toSettingsPreview, type Permissions } from '$lib/ipc';
  import { app } from '$lib/state.svelte';

  type Tab = 'allowed' | 'denied' | 'ask' | 'conflicts' | 'byscope' | 'raw';
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
    { id: 'raw', label: 'Raw Rules' }
  ];
</script>

<div class="panel">
  <div class="tabs">
    {#each tabs as t (t.id)}
      <button class:active={tab === t.id} onclick={() => (tab = t.id)}>
        {t.label}
        {#if t.id === 'conflicts' && conflicts.length}<span class="dot">{conflicts.length}</span>{/if}
      </button>
    {/each}
  </div>

  <div class="body">
    {#if tab === 'allowed'}
      {#each allowed as e (e.path)}<div class="row"><span class="i">✅</span>{e.path}</div>{:else}<p class="muted">없음</p>{/each}
    {:else if tab === 'denied'}
      {#each denied as e (e.path)}<div class="row"><span class="i">⛔</span>{e.path}<span class="src">{e.sourceScope ?? ''}</span></div>{:else}<p class="muted">없음</p>{/each}
    {:else if tab === 'ask'}
      {#each asked as e (e.path)}<div class="row"><span class="i">❓</span>{e.path}</div>{:else}<p class="muted">없음</p>{/each}
    {:else if tab === 'conflicts'}
      {#each conflicts as e (e.path)}<div class="row conflict"><span class="i">⚠️</span>{e.path} → {e.effective.toUpperCase()} <span class="src">deny 우선</span></div>{:else}<p class="muted">충돌 없음</p>{/each}
    {:else if tab === 'byscope'}
      {#each byScope as g (g.scope)}
        <div class="scope-group">
          <b>{g.scope}{#if g.defaultMode} · defaultMode={g.defaultMode}{/if}</b>
          {#each g.rules as r (r.path + r.policy)}
            <div class="row"><span class="i i-{r.policy}">{r.policy[0].toUpperCase()}</span>{r.path}</div>
          {:else}
            <p class="muted">규칙 없음</p>
          {/each}
        </div>
      {/each}
    {:else if tab === 'raw'}
      {#if raw}
        <div class="raw-group"><b>allow</b>{#each raw.allow as r}<code>{r}</code>{/each}</div>
        <div class="raw-group"><b>ask</b>{#each raw.ask as r}<code>{r}</code>{/each}</div>
        <div class="raw-group"><b>deny</b>{#each raw.deny as r}<code>{r}</code>{/each}</div>
        <p class="note">= {app.activeScope} scope에 저장될 실제 규칙</p>
      {:else}
        <p class="muted">계산 중…</p>
      {/if}
    {/if}
  </div>
</div>

<style>
  .panel { display: flex; flex-direction: column; height: 100%; }
  .tabs { display: flex; border-bottom: 1px solid #1e293b; overflow-x: auto; }
  .tabs button {
    padding: 0.5rem 0.7rem; background: none; border: none; border-bottom: 2px solid transparent;
    color: #94a3b8; cursor: pointer; font-size: 0.8rem; white-space: nowrap;
  }
  .tabs button.active { color: #e2e8f0; border-bottom-color: #2563eb; }
  .dot { background: #7f1d1d; color: #fecaca; border-radius: 999px; padding: 0 0.35rem; font-size: 0.65rem; margin-left: 0.25rem; }
  .body { overflow: auto; padding: 0.5rem; flex: 1; }
  .row { display: flex; align-items: center; gap: 0.4rem; font-size: 0.82rem; padding: 0.15rem 0.2rem; }
  .row .i { width: 1.1rem; }
  .row .src { margin-left: auto; color: #64748b; font-size: 0.7rem; }
  .row.conflict { color: #fca5a5; }
  .muted { color: #64748b; font-size: 0.85rem; }
  .raw-group { margin-bottom: 0.6rem; display: flex; flex-direction: column; gap: 0.15rem; }
  .raw-group b { color: #94a3b8; font-size: 0.75rem; }
  code { background: #0b1220; padding: 0.15rem 0.4rem; border-radius: 4px; font-size: 0.75rem; }
  .note { color: #475569; font-size: 0.72rem; }
  .scope-group { margin-bottom: 0.7rem; }
  .scope-group b { color: #94a3b8; font-size: 0.75rem; text-transform: capitalize; }
  .i-allow { color: #4ade80; } .i-deny { color: #f87171; } .i-ask { color: #fbbf24; }
</style>
