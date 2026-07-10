<script lang="ts">
  import { listDir, type DirEntry } from '$lib/ipc';
  import { app } from '$lib/state.svelte';
  import Self from './TreeNode.svelte';

  let { entry }: { entry: DirEntry } = $props();

  let expanded = $state(false);
  let children = $state<DirEntry[] | null>(null);
  let loading = $state(false);

  async function toggle() {
    if (!entry.isDir || entry.excluded) return;
    expanded = !expanded;
    if (expanded && children === null) {
      loading = true;
      try {
        children = await listDir(app.projectRoot, entry.path);
      } finally {
        loading = false;
      }
    }
  }

  // Badge from an explicit rule on this exact path (deny > ask > allow across scopes),
  // else a scanner recommendation, else untracked.
  const badge = $derived.by(() => {
    const scopes = [app.scoped.local, app.scoped.project, app.scoped.user];
    let found: string | null = null;
    for (const p of ['deny', 'ask', 'allow'] as const) {
      for (const s of scopes) {
        if (s.rules.some((r) => r.path === entry.path && r.policy === p)) {
          found = p;
          break;
        }
      }
      if (found) break;
    }
    if (found) return { kind: found, label: found.toUpperCase() };
    const scan = app.view?.scan;
    if (scan?.denyCandidates.includes(entry.path)) return { kind: 'rec-deny', label: 'DENY?' };
    if (scan?.allowCandidates.includes(entry.path)) return { kind: 'rec-allow', label: 'ALLOW?' };
    return { kind: 'untracked', label: '' };
  });

  const selected = $derived(app.selectedPath === entry.path);
</script>

<div class="node" class:selected>
  <div class="row">
    <button
      class="twist"
      class:hidden={!entry.isDir || entry.excluded}
      aria-label="expand"
      onclick={toggle}
    >
      {expanded ? '▾' : '▸'}
    </button>
    <button class="label" onclick={() => (app.selectedPath = entry.path)} ondblclick={toggle}>
      <span class="icon">{entry.isDir ? '📁' : '📄'}</span>
      <span class="name" class:dim={entry.excluded}>{entry.name}</span>
      {#if badge.label}
        <span class="badge b-{badge.kind}">{badge.label}</span>
      {/if}
    </button>
  </div>

  {#if expanded}
    <div class="children">
      {#if loading}
        <div class="loading">…</div>
      {:else if children}
        {#each children as child (child.path)}
          <Self entry={child} />
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .row {
    display: flex; align-items: center; gap: 0.2rem; width: 100%;
    padding: 0.05rem 0.3rem; border-radius: 4px;
  }
  .row:hover { background: #1e293b; }
  .selected > .row { background: #1e3a5f; }
  .twist {
    width: 1.1rem; color: #64748b; background: none; border: none; cursor: pointer;
    padding: 0; font-size: 0.8rem;
  }
  .twist.hidden { visibility: hidden; }
  .label {
    display: flex; align-items: center; gap: 0.35rem; flex: 1; min-width: 0;
    background: none; border: none; color: #cbd5e1; cursor: pointer;
    text-align: left; font-size: 0.85rem; padding: 0.1rem 0;
  }
  .name.dim { color: #475569; font-style: italic; }
  .children { margin-left: 1rem; border-left: 1px solid #1e293b; padding-left: 0.25rem; }
  .loading { color: #475569; font-size: 0.8rem; padding-left: 1rem; }
  .badge { margin-left: auto; font-size: 0.62rem; padding: 0.05rem 0.4rem; border-radius: 999px; font-weight: 700; }
  .b-allow { background: #14532d; color: #bbf7d0; }
  .b-deny { background: #7f1d1d; color: #fecaca; }
  .b-ask { background: #78350f; color: #fde68a; }
  .b-rec-deny { background: none; color: #f87171; border: 1px dashed #7f1d1d; }
  .b-rec-allow { background: none; color: #4ade80; border: 1px dashed #14532d; }
</style>
