<script lang="ts">
  import { listDir, type DirEntry } from '$lib/ipc';
  import { app } from '$lib/state.svelte';
  import { effectiveDisplay } from '$lib/match';
  import Self from './TreeNode.svelte';

  let {
    entry,
    oncontext
  }: {
    entry: DirEntry;
    oncontext?: (entry: DirEntry, x: number, y: number) => void;
  } = $props();

  // Right-click a row: select it and open the policy context menu (like the
  // system explorer used for user settings).
  function onContextMenu(e: MouseEvent) {
    if (!oncontext || entry.excluded) return;
    e.preventDefault();
    e.stopPropagation();
    app.selectedPath = entry.path;
    oncontext(entry, e.clientX, e.clientY);
  }

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

  // Effective access coloring: all scopes' rules merged (deny > ask > allow),
  // with folder rules inherited by children — same convention as the system
  // explorer. Excluded dirs keep their dimmed style instead.
  const eff = $derived.by(() => {
    if (entry.excluded) return { state: 'none' as const, source: null };
    const rules = [...app.scoped.local.rules, ...app.scoped.project.rules, ...app.scoped.user.rules];
    const dm =
      app.scoped.local.defaultMode ?? app.scoped.project.defaultMode ?? app.scoped.user.defaultMode;
    return effectiveDisplay(rules, dm, entry.path);
  });
</script>

<div class="node" class:selected>
  <div class="row">
    <button
      class="twist"
      class:hidden={!entry.isDir || entry.excluded}
      class:open={expanded}
      aria-label={expanded ? '접기' : '펼치기'}
      onclick={toggle}
    >
      <svg viewBox="0 0 16 16" width="12" height="12" fill="none" aria-hidden="true">
        <path d="m6 4 4 4-4 4" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </button>
    <button class="label" onclick={() => (app.selectedPath = entry.path)} ondblclick={toggle} oncontextmenu={onContextMenu}>
      <span class="icon e-{eff.state}" class:folder={entry.isDir} aria-hidden="true">
        {#if entry.isDir}
          <svg viewBox="0 0 16 16" width="14" height="14" fill="none">
            <path
              d="M1.8 4.2A1.2 1.2 0 0 1 3 3h3l1.4 1.5H13a1.2 1.2 0 0 1 1.2 1.2v6.1A1.2 1.2 0 0 1 13 13H3a1.2 1.2 0 0 1-1.2-1.2V4.2Z"
              fill="currentColor"
              fill-opacity="0.25"
              stroke="currentColor"
              stroke-width="1.1"
              stroke-linejoin="round"
            />
          </svg>
        {:else}
          <svg viewBox="0 0 16 16" width="14" height="14" fill="none">
            <path
              d="M4 1.8h5.2L12.5 5v9.2H4V1.8Z"
              stroke="currentColor"
              stroke-width="1.1"
              stroke-linejoin="round"
            />
            <path d="M9 2v3.3h3.3" stroke="currentColor" stroke-width="1.1" stroke-linejoin="round" />
          </svg>
        {/if}
      </span>
      <span class="name e-{eff.state}" class:dim={entry.excluded}>{entry.name}</span>
      {#if entry.ignored && !entry.excluded}
        <span
          class="gitig"
          title=".gitignore 경로 — 에이전트가 Grep 검색으로는 발견하지 못하지만, 경로를 알면 읽을 수 있습니다. 접근 허용/차단은 Allow/Ask/Deny 규칙이 결정합니다."
        >검색 제외</span>
      {/if}
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
          <Self entry={child} {oncontext} />
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 0.15rem;
    width: 100%;
    padding: 0.08rem 0.35rem;
    border-radius: var(--r-sm);
    transition: background-color var(--t-fast);
  }
  .row:hover {
    background: var(--bg-2);
  }
  .selected > .row {
    background: var(--accent-soft);
    box-shadow: inset 2px 0 0 var(--accent);
  }
  .twist {
    display: grid;
    place-items: center;
    width: 1.15rem;
    height: 1.15rem;
    color: var(--text-3);
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    transition: transform var(--t-fast), color var(--t-fast);
  }
  .twist:hover {
    color: var(--text-1);
  }
  .twist.open {
    transform: rotate(90deg);
  }
  .twist.hidden {
    visibility: hidden;
  }
  .label {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex: 1;
    min-width: 0;
    background: none;
    border: none;
    color: var(--text-1);
    cursor: pointer;
    text-align: left;
    font-size: 0.84rem;
    padding: 0.14rem 0;
  }
  .icon {
    display: grid;
    place-items: center;
    color: var(--text-3);
    flex-shrink: 0;
  }
  .icon.folder {
    color: #8fb3f9;
  }
  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .name.dim {
    color: var(--text-3);
    font-style: italic;
  }
  /* Effective access coloring (explicit or inherited from an ancestor rule). */
  .name.e-allow,
  .icon.e-allow {
    color: var(--allow);
  }
  .name.e-ask,
  .icon.e-ask {
    color: var(--ask);
  }
  .name.e-deny,
  .icon.e-deny {
    color: var(--deny);
  }
  .name.e-deny-default,
  .icon.e-deny-default {
    color: var(--deny);
    opacity: 0.55;
  }
  .children {
    margin-left: 0.95rem;
    border-left: 1px solid var(--border);
    padding-left: 0.3rem;
  }
  .loading {
    color: var(--text-3);
    font-size: 0.8rem;
    padding-left: 1rem;
  }
  /* Neutral gray on purpose: git-ignored means "invisible to search", NOT
     "blocked" — red is reserved for Deny. */
  .gitig {
    margin-left: auto;
    font-size: 0.58rem;
    color: var(--text-3);
    border: 1px dashed var(--border-strong);
    border-radius: 999px;
    padding: 0.05rem 0.4rem;
    flex-shrink: 0;
  }
  .gitig + .badge {
    margin-left: 0.25rem;
  }
  .badge {
    margin-left: auto;
    font-size: 0.6rem;
    padding: 0.08rem 0.42rem;
    border-radius: 999px;
    font-weight: 700;
    letter-spacing: 0.03em;
    flex-shrink: 0;
  }
  .b-allow {
    background: var(--allow-soft);
    color: var(--allow);
    border: 1px solid rgba(52, 211, 153, 0.3);
  }
  .b-deny {
    background: var(--deny-soft);
    color: var(--deny);
    border: 1px solid rgba(248, 113, 113, 0.3);
  }
  .b-ask {
    background: var(--ask-soft);
    color: var(--ask);
    border: 1px solid rgba(251, 191, 36, 0.3);
  }
  .b-rec-deny {
    background: none;
    color: var(--deny);
    border: 1px dashed rgba(248, 113, 113, 0.45);
  }
  .b-rec-allow {
    background: none;
    color: var(--allow);
    border: 1px dashed rgba(52, 211, 153, 0.45);
  }
</style>
