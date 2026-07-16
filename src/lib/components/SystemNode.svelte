<script lang="ts">
  // One row of the all-drives explorer. Lazy-loads children on expand and shows
  // the explicit rule badge for this path's pattern in the given scope.
  import { listSystemDir, type ScopeName, type SystemEntry } from '$lib/ipc';
  import { app } from '$lib/state.svelte';
  import { knownDescription } from '$lib/describe';
  import { effectiveDisplay, type EffectiveDisplay } from '$lib/match';
  import { tooltip } from '$lib/tooltip';
  import Self from './SystemNode.svelte';

  let {
    entry,
    scope,
    selectedPath,
    onselect,
    oncontext
  }: {
    entry: SystemEntry;
    scope: ScopeName;
    selectedPath: string;
    onselect: (e: SystemEntry) => void;
    oncontext: (e: SystemEntry, x: number, y: number) => void;
  } = $props();

  function onContextMenu(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    onselect(entry);
    oncontext(entry, e.clientX, e.clientY);
  }

  let expanded = $state(false);
  let children = $state<SystemEntry[] | null>(null);
  let loading = $state(false);
  let loadError = $state<string | null>(null);

  async function toggle() {
    if (!entry.isDir) return;
    expanded = !expanded;
    if (expanded && children === null && !loading) {
      loading = true;
      loadError = null;
      try {
        children = await listSystemDir(entry.path);
      } catch (e) {
        loadError = String(e);
      } finally {
        loading = false;
      }
    }
  }

  // Explicit rule on this exact path (folder rule `base/**` or exact `base`).
  const rule = $derived(
    app.scoped[scope].rules.find(
      (r) => r.path === entry.pattern || r.path === entry.pattern + '/**'
    )
  );
  const selected = $derived(selectedPath === entry.path);

  // Effective access for this path — explicit rule, ancestor-inherited rule,
  // or the scope default. Colors the name so inheritance is visible.
  const eff = $derived(effectiveDisplay(app.scoped[scope].rules, entry.pattern));
  const inherited = $derived(
    !!eff.source && eff.source !== entry.pattern && eff.source !== entry.pattern + '/**'
  );

  function stateLine(e: EffectiveDisplay, inh: boolean): string {
    const from = inh ? ` — 상위 규칙 ${e.source} 상속` : '';
    switch (e.state) {
      case 'allow':
        return `지금 상태: 접근 가능${from}`;
      case 'ask':
        return `지금 상태: 접근할 때마다 확인${from}`;
      case 'deny':
        return `지금 상태: 차단됨${from}`;
      default:
        return '지금 상태: 규칙 없음 — 기본 동작 (필요할 때 확인)';
    }
  }

  const tip = $derived.by(() => {
    const lines: string[] = [];
    const known = knownDescription(entry.pattern);
    if (known) lines.push(known);
    lines.push(stateLine(eff, inherited));
    return lines.join('\n');
  });
</script>

<div class="node">
  <div class="row" class:selected>
    <button
      class="twist"
      class:hidden={!entry.isDir}
      class:open={expanded}
      aria-label={expanded ? '접기' : '펼치기'}
      onclick={toggle}
    >
      <svg viewBox="0 0 16 16" width="11" height="11" fill="none" aria-hidden="true">
        <path d="m6 4 4 4-4 4" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </button>
    <button
      class="label"
      use:tooltip={tip}
      onclick={() => onselect(entry)}
      ondblclick={toggle}
      oncontextmenu={onContextMenu}
    >
      <span class="icon e-{eff.state}" class:folder={entry.isDir} aria-hidden="true">
        {#if entry.isDir}
          <svg viewBox="0 0 16 16" width="13" height="13" fill="none">
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
          <svg viewBox="0 0 16 16" width="13" height="13" fill="none">
            <path d="M4 1.8h5.2L12.5 5v9.2H4V1.8Z" stroke="currentColor" stroke-width="1.1" stroke-linejoin="round" />
            <path d="M9 2v3.3h3.3" stroke="currentColor" stroke-width="1.1" stroke-linejoin="round" />
          </svg>
        {/if}
      </span>
      <span class="name e-{eff.state}">{entry.name}</span>
      {#if rule}
        <span class="badge b-{rule.policy}">{rule.policy.toUpperCase()}</span>
      {:else if inherited}
        <span class="inh i-{eff.state}" aria-hidden="true"></span>
      {/if}
    </button>
  </div>

  {#if expanded}
    <div class="children">
      {#if loading}
        <div class="note">불러오는 중…</div>
      {:else if loadError}
        <div class="note err">접근할 수 없습니다</div>
      {:else if children}
        {#if children.length === 0}
          <div class="note">비어 있음</div>
        {:else}
          {#each children as child (child.path)}
            <Self entry={child} {scope} {selectedPath} {onselect} {oncontext} />
          {/each}
        {/if}
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
    padding: 0.06rem 0.3rem;
    border-radius: var(--r-sm);
    transition: background-color var(--t-fast);
  }
  .row:hover {
    background: var(--bg-2);
  }
  .row.selected {
    background: var(--accent-soft);
    box-shadow: inset 2px 0 0 var(--accent);
  }
  .twist {
    display: grid;
    place-items: center;
    width: 1.05rem;
    height: 1.05rem;
    color: var(--text-3);
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    flex-shrink: 0;
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
    gap: 0.38rem;
    flex: 1;
    min-width: 0;
    background: none;
    border: none;
    color: var(--text-1);
    cursor: pointer;
    text-align: left;
    font-size: 0.8rem;
    padding: 0.12rem 0;
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
  /* Effective access coloring (explicit or inherited from ancestors). */
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
  /* Inherited state marker: small hollow dot (explicit rules get the badge). */
  .inh {
    margin-left: auto;
    width: 6px;
    height: 6px;
    border-radius: 999px;
    flex-shrink: 0;
  }
  .inh.i-allow {
    background: var(--allow);
    opacity: 0.55;
  }
  .inh.i-ask {
    background: var(--ask);
    opacity: 0.55;
  }
  .inh.i-deny {
    background: var(--deny);
    opacity: 0.55;
  }
  .badge {
    margin-left: auto;
    font-size: 0.58rem;
    padding: 0.06rem 0.4rem;
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
  .children {
    margin-left: 0.85rem;
    border-left: 1px solid var(--border);
    padding-left: 0.25rem;
  }
  .note {
    color: var(--text-3);
    font-size: 0.72rem;
    padding: 0.1rem 0 0.1rem 1rem;
  }
  .note.err {
    color: var(--deny);
  }
</style>
