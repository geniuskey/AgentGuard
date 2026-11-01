<script lang="ts">
  import { effectiveFor, listDir, type DirEntry, type Tool } from '$lib/ipc';
  import { app } from '$lib/state.svelte';
  import type { EffectiveDisplay } from '$lib/match';
  import Self from './TreeNode.svelte';

  let {
    entry,
    pathTool = 'Read',
    oncontext
  }: {
    entry: DirEntry;
    pathTool?: Tool;
    oncontext?: (entry: DirEntry, x: number, y: number, anchor: HTMLElement) => void;
  } = $props();

  // Right-click a row: select it and open the policy context menu (like the
  // system explorer used for user settings).
  function onContextMenu(e: MouseEvent) {
    if (!oncontext || entry.excluded) return;
    e.preventDefault();
    e.stopPropagation();
    app.selectedPath = entry.path;
    oncontext(entry, e.clientX, e.clientY, e.currentTarget as HTMLElement);
  }

  let expanded = $state(false);
  let children = $state<DirEntry[] | null>(null);
  let loading = $state(false);
  let loadError = $state<string | null>(null);
  const expandable = $derived(entry.isDir && !entry.excluded);

  async function load() {
    loading = true;
    loadError = null;
    try {
      children = await listDir(app.projectRoot, entry.path);
    } catch (e) {
      loadError = String(e);
    } finally {
      loading = false;
    }
  }

  async function toggle() {
    if (!expandable) return;
    expanded = !expanded;
    if (expanded && children === null) await load();
  }

  function onLabelKey(e: KeyboardEvent) {
    if (!expandable) return;
    if (e.key === 'ArrowRight' && !expanded) {
      e.preventDefault();
      toggle();
    } else if (e.key === 'ArrowLeft' && expanded) {
      e.preventDefault();
      toggle();
    }
  }

  // Badge from an explicit rule on this exact path (deny > ask > allow across scopes),
  // else a scanner recommendation, else untracked.
  const badge = $derived.by(() => {
    const scopes = app.scoped.managed.enforceManagedOnly ? [app.scoped.managed] : [app.scoped.managed, app.scoped.local, app.scoped.project, app.scoped.user];
    let found: string | null = null;
    for (const p of ['deny', 'ask', 'allow'] as const) {
      for (const s of scopes) {
        if (s.rules.some((r) => r.path === entry.path && r.policy === p && (!r.tools || r.tools.includes(pathTool) || (pathTool === 'Edit' && p === 'deny' && r.tools.includes('Read'))))) {
          found = p;
          break;
        }
      }
      if (found) break;
    }
    if (found) return { kind: found, label: `${pathTool}: ${found[0].toUpperCase()}${found.slice(1)}` };
    const scan = app.view?.scan;
    if (scan?.denyCandidates.includes(entry.path)) return { kind: 'rec-deny', label: '검토 후보' };
    if (scan?.allowCandidates.includes(entry.path)) return { kind: 'rec-allow', label: '작업 경로 후보' };
    return { kind: 'untracked', label: '' };
  });

  const selected = $derived(app.selectedPath === entry.path);

  let eff = $state<EffectiveDisplay>({ state: 'none', source: null });
  $effect(() => {
    const scoped = $state.snapshot(app.scoped);
    const path = entry.path;
    const tool = pathTool;
    const root = app.projectRoot;
    let cancelled = false;
    eff = { state: 'none', source: null };
    if (!entry.excluded) effectiveFor(scoped, path, root, tool).then((result) => {
      if (!cancelled) eff = { state: result.effective ?? 'none', source: result.sourceScope };
    }).catch(() => {});
    return () => { cancelled = true; };
  });
</script>

<div
  class="node"
  class:selected
  role="treeitem"
  tabindex="-1"
  aria-selected={selected}
  aria-expanded={expandable ? expanded : undefined}
>
  <div class="row">
    <button
      class="twist"
      class:hidden={!expandable}
      class:open={expanded}
      tabindex="-1"
      aria-label={`${entry.name} ${expanded ? '접기' : '펼치기'}`}
      onclick={toggle}
    >
      <svg viewBox="0 0 16 16" width="12" height="12" fill="none" aria-hidden="true">
        <path d="m6 4 4 4-4 4" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </button>
    <button class="label" onclick={() => (app.selectedPath = entry.path)} ondblclick={toggle} onkeydown={onLabelKey} oncontextmenu={onContextMenu}>
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
        <span class="chip badge b-{badge.kind}">{badge.label}</span>
      {/if}
    </button>
  </div>

  {#if expanded}
    <div class="children" role="group">
      {#if loading}
        <div class="loading" role="status">불러오는 중…</div>
      {:else if loadError}
        <div class="load-error" role="alert">
          <span class="truncate" title={loadError}>폴더를 열지 못했습니다</span>
          <button class="btn btn-ghost btn-sm" onclick={load}>다시 시도</button>
        </div>
      {:else if children}
        {#each children as child (child.path)}
          <Self entry={child} {pathTool} {oncontext} />
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
  .node:focus {
    outline: none;
  }
  .row:hover {
    background: var(--hover);
  }
  .selected > .row {
    background: var(--accent-soft);
    box-shadow: inset 2px 0 0 var(--accent);
  }
  .twist {
    display: grid;
    place-items: center;
    width: 1.5rem;
    height: 1.6rem;
    flex-shrink: 0;
    margin: -0.1rem 0 -0.1rem -0.2rem;
    border-radius: var(--r-sm);
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
    font-size: var(--fs-lg);
    padding: 0.14rem 0;
  }
  .icon {
    display: grid;
    place-items: center;
    color: var(--text-3);
    flex-shrink: 0;
  }
  .icon.folder {
    color: var(--accent-text);
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
  .children {
    margin-left: 0.95rem;
    border-left: 1px solid var(--border);
    padding-left: 0.3rem;
  }
  .loading {
    color: var(--text-3);
    font-size: var(--fs-sm);
    padding: 0.15rem 0 0.15rem 1rem;
  }
  .load-error {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    min-width: 0;
    color: var(--deny);
    font-size: var(--fs-sm);
    padding: 0.15rem 0 0.15rem 1rem;
  }
  /* Neutral gray on purpose: git-ignored means "invisible to search", NOT
     "blocked" — red is reserved for Deny. */
  .gitig {
    margin-left: auto;
    font-size: var(--fs-xs);
    color: var(--text-3);
    border: 1px dashed var(--border-strong);
    border-radius: 999px;
    padding: 0.05rem 0.4rem;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .gitig + .badge {
    margin-left: 0.25rem;
  }
  .badge {
    margin-left: auto;
    flex-shrink: 0;
  }
  .b-allow {
    background: var(--allow-soft);
    color: var(--allow);
    border-color: var(--allow-line);
  }
  .b-deny {
    background: var(--deny-soft);
    color: var(--deny);
    border-color: var(--deny-line);
  }
  .b-ask {
    background: var(--ask-soft);
    color: var(--ask);
    border-color: var(--ask-line);
  }
  .b-rec-deny {
    background: none;
    color: var(--deny);
    border: 1px dashed var(--deny-line);
  }
  .b-rec-allow {
    background: none;
    color: var(--allow);
    border: 1px dashed var(--allow-line);
  }
</style>
