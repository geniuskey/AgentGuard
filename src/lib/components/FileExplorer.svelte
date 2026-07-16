<script lang="ts">
  import type { DirEntry, Policy } from '$lib/ipc';
  import { app, clearPolicy, refreshEffective, setPolicy } from '$lib/state.svelte';
  import TreeNode from './TreeNode.svelte';

  let menu = $state<{ entry: DirEntry; x: number; y: number } | null>(null);

  // The explicit rule on this exact path in the active scope, if any.
  function ruleFor(entry: DirEntry) {
    return app.scoped[app.activeScope].rules.find((r) => r.path === entry.path);
  }

  async function applyTo(entry: DirEntry, policy: Policy) {
    menu = null;
    setPolicy(entry.path, policy, entry.isDir ? 'folder-and-children' : 'file');
    await refreshEffective();
  }

  async function clearFor(entry: DirEntry) {
    menu = null;
    clearPolicy(entry.path);
    await refreshEffective();
  }

  // Right-click on a tree row: open the policy context menu, clamped to viewport.
  function openMenu(entry: DirEntry, x: number, y: number) {
    const mw = 210;
    const mh = 200;
    menu = {
      entry,
      x: Math.max(8, Math.min(x, window.innerWidth - mw - 8)),
      y: Math.max(8, Math.min(y, window.innerHeight - mh - 8))
    };
  }
</script>

<div class="panel">
  <div class="head">
    <span class="ptitle">{app.projectName || 'Files'}</span>
    <button class="root" class:active={app.selectedPath === ''} onclick={() => (app.selectedPath = '')}>
      / root
    </button>
  </div>
  <div class="tree">
    {#if app.view}
      {#each app.view.tree as entry (entry.path)}
        <TreeNode {entry} oncontext={openMenu} />
      {/each}
    {:else}
      <p class="muted">프로젝트가 열려 있지 않습니다.</p>
    {/if}
  </div>
</div>

<svelte:window
  onclick={() => (menu = null)}
  oncontextmenu={() => (menu = null)}
  onkeydown={(e) => e.key === 'Escape' && (menu = null)}
/>

{#if menu}
  {@const m = menu}
  {@const mRule = ruleFor(m.entry)}
  <div class="ctx" style="left: {m.x}px; top: {m.y}px" role="menu" aria-label="정책 지정">
    <div class="ctx-head">
      <span class="ctx-name">{m.entry.name || '(project root)'}</span>
      <code class="ctx-pat">{m.entry.path}<span class="ctx-scope">{app.activeScope}</span></code>
    </div>
    <button role="menuitem" class="ctx-item" onclick={() => applyTo(m.entry, 'allow')}>
      <span class="cdot c-allow" aria-hidden="true"></span>
      <span class="ctx-label">Allow<span class="ctx-sub">작업 폴더 — 접근 허용</span></span>
      {#if mRule?.policy === 'allow'}<span class="ctx-check">✓</span>{/if}
    </button>
    <button role="menuitem" class="ctx-item" onclick={() => applyTo(m.entry, 'ask')}>
      <span class="cdot c-ask" aria-hidden="true"></span>
      <span class="ctx-label">Ask<span class="ctx-sub">접근할 때마다 확인</span></span>
      {#if mRule?.policy === 'ask'}<span class="ctx-check">✓</span>{/if}
    </button>
    <button role="menuitem" class="ctx-item" onclick={() => applyTo(m.entry, 'deny')}>
      <span class="cdot c-deny" aria-hidden="true"></span>
      <span class="ctx-label">Deny<span class="ctx-sub">민감 폴더 — 접근 차단</span></span>
      {#if mRule?.policy === 'deny'}<span class="ctx-check">✓</span>{/if}
    </button>
    <div class="ctx-sep" role="separator"></div>
    <button role="menuitem" class="ctx-item" disabled={!mRule} onclick={() => clearFor(m.entry)}>
      <span class="cdot" aria-hidden="true"></span>
      <span class="ctx-label">규칙 해제</span>
    </button>
  </div>
{/if}

<style>
  .panel {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
    padding: 0.55rem 0.7rem;
    border-bottom: 1px solid var(--border);
    font-size: 0.8rem;
  }
  .ptitle {
    font-weight: 600;
    letter-spacing: 0.02em;
    color: var(--text-2);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .root {
    font-size: 0.7rem;
    font-family: var(--font-mono);
    background: none;
    border: 1px solid var(--border-strong);
    color: var(--text-2);
    border-radius: 4px;
    padding: 0.12rem 0.45rem;
    cursor: pointer;
    flex-shrink: 0;
    transition: color var(--t-fast), border-color var(--t-fast), background-color var(--t-fast);
  }
  .root:hover {
    color: var(--text-1);
    background: var(--bg-2);
  }
  .root.active {
    border-color: rgba(79, 142, 247, 0.45);
    background: var(--accent-soft);
    color: var(--accent-text);
  }
  .tree {
    overflow: auto;
    padding: 0.4rem;
    flex: 1;
  }
  .muted {
    color: var(--text-3);
    font-size: 0.85rem;
  }

  /* Right-click policy menu (fixed — escapes panel clipping). */
  .ctx {
    position: fixed;
    z-index: 90;
    min-width: 200px;
    background: var(--bg-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--r-md);
    box-shadow: var(--shadow-modal);
    padding: 0.3rem;
    animation: ag-rise-in 120ms cubic-bezier(0.2, 0, 0, 1);
  }
  .ctx-head {
    padding: 0.3rem 0.5rem 0.4rem;
    border-bottom: 1px solid var(--border);
    margin-bottom: 0.25rem;
  }
  .ctx-name {
    display: block;
    font-size: 0.78rem;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 230px;
  }
  .ctx-pat {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.66rem;
    color: var(--text-3);
    word-break: break-all;
    margin-top: 0.1rem;
  }
  .ctx-scope {
    margin-left: auto;
    flex-shrink: 0;
    font-size: 0.62rem;
    color: var(--accent-text);
    background: var(--accent-soft);
    border-radius: 999px;
    padding: 0.02rem 0.4rem;
    text-transform: capitalize;
  }
  .ctx-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    background: none;
    border: none;
    border-radius: var(--r-sm);
    padding: 0.35rem 0.5rem;
    cursor: pointer;
    text-align: left;
    color: var(--text-1);
    font-size: 0.8rem;
    transition: background-color var(--t-fast);
  }
  .ctx-item:hover:not(:disabled) {
    background: var(--bg-3);
  }
  .ctx-item:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .cdot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    border: 1px solid var(--border-strong);
    flex-shrink: 0;
  }
  .c-allow {
    background: var(--allow);
    border-color: var(--allow);
  }
  .c-ask {
    background: var(--ask);
    border-color: var(--ask);
  }
  .c-deny {
    background: var(--deny);
    border-color: var(--deny);
  }
  .ctx-label {
    display: flex;
    flex-direction: column;
    min-width: 0;
    font-weight: 600;
    line-height: 1.25;
  }
  .ctx-sub {
    font-size: 0.68rem;
    font-weight: 400;
    color: var(--text-3);
  }
  .ctx-check {
    margin-left: auto;
    color: var(--accent-text);
    font-weight: 700;
  }
  .ctx-sep {
    height: 1px;
    background: var(--border);
    margin: 0.25rem 0.2rem;
  }
</style>
