<script lang="ts">
  import type { DirEntry, Policy, Tool } from '$lib/ipc';
  import { tick } from 'svelte';
  import { app, clearPolicy, refreshEffective, setPolicy } from '$lib/state.svelte';
  import TreeNode from './TreeNode.svelte';

  let pathTool = $state<Tool>('Read');
  let menu = $state<{ entry: DirEntry; x: number; y: number } | null>(null);
  let menuEl = $state<HTMLDivElement | null>(null);
  let menuAnchor: HTMLElement | null = null;

  // The explicit rule on this exact path in the active scope, if any.
  function ruleFor(entry: DirEntry) {
    return app.scoped[app.activeScope].rules.find((r) => r.path === entry.path);
  }

  function closeMenu(restoreFocus = false) {
    if (!menu) return;
    menu = null;
    if (restoreFocus) menuAnchor?.focus();
    menuAnchor = null;
  }

  async function applyTo(entry: DirEntry, policy: Policy) {
    closeMenu(true);
    setPolicy(entry.path, policy, entry.isDir ? 'folder-and-children' : 'file');
    await refreshEffective();
  }

  async function clearFor(entry: DirEntry) {
    closeMenu(true);
    clearPolicy(entry.path);
    await refreshEffective();
  }

  // Right-click on a tree row: open the policy context menu, clamped to viewport.
  async function openMenu(entry: DirEntry, x: number, y: number, anchor: HTMLElement) {
    const mw = 210;
    const mh = 200;
    if (x === 0 && y === 0) {
      const r = anchor.getBoundingClientRect();
      x = r.left + 24;
      y = r.bottom;
    }
    menuAnchor = anchor;
    menu = {
      entry,
      x: Math.max(8, Math.min(x, window.innerWidth - mw - 8)),
      y: Math.max(8, Math.min(y, window.innerHeight - mh - 8))
    };
    await tick();
    menuEl?.querySelector<HTMLElement>('[role="menuitem"]:not(:disabled)')?.focus();
  }

  function onMenuKey(e: KeyboardEvent) {
    if (!menuEl) return;
    const items = Array.from(menuEl.querySelectorAll<HTMLElement>('[role="menuitem"]:not(:disabled)'));
    const i = items.indexOf(document.activeElement as HTMLElement);
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      const next = e.key === 'ArrowDown' ? (i + 1) % items.length : (i - 1 + items.length) % items.length;
      items[next]?.focus();
    } else if (e.key === 'Tab') {
      e.preventDefault();
      closeMenu(true);
    }
  }

  function onWindowKey(e: KeyboardEvent) {
    if (e.key === 'Escape' && menu) {
      e.preventDefault();
      e.stopPropagation();
      closeMenu(true);
    }
  }
</script>

<div class="panel">
  <div class="head">
    <span class="ptitle truncate">{app.projectName || '파일'}</span>
    <label class="tool-pick">규칙 색상 <select class="input" bind:value={pathTool}><option value="Read">Read</option><option value="Edit">Edit</option></select></label>
    <button
      class="btn btn-sm root"
      class:active={app.selectedPath === ''}
      aria-pressed={app.selectedPath === ''}
      aria-label="/ root 선택"
      onclick={() => (app.selectedPath = '')}
    >
      / root
    </button>
  </div>
  <p class="note">색상은 선택한 도구의 명시 규칙 비교입니다. 실행 결과는 보장하지 않습니다.</p>
  <div class="tree" role="tree" aria-label="프로젝트 파일">
    {#if app.view}
      {#each app.view.tree as entry (entry.path)}
        <TreeNode {entry} {pathTool} oncontext={openMenu} />
      {/each}
    {:else}
      <p class="muted">프로젝트가 열려 있지 않습니다.</p>
    {/if}
  </div>
</div>

<svelte:window
  onclick={() => closeMenu()}
  oncontextmenu={() => closeMenu()}
  onkeydown={onWindowKey}
/>

{#if menu}
  {@const m = menu}
  {@const mRule = ruleFor(m.entry)}
  <div
    class="ctx"
    style="left: {m.x}px; top: {m.y}px"
    role="menu"
    tabindex="-1"
    aria-label="정책 지정"
    bind:this={menuEl}
    onkeydown={onMenuKey}
  >
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
    align-items: center;
    gap: 0.5rem;
    min-height: 2.6rem;
    box-sizing: border-box;
    padding: 0.4rem 0.7rem;
    border-bottom: 1px solid var(--border);
    font-size: var(--fs-md);
    flex-shrink: 0;
  }
  .ptitle {
    flex: 1;
    font-weight: 600;
    color: var(--text-2);
  }
  .tool-pick {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    color: var(--text-3);
    font-size: var(--fs-sm);
    white-space: nowrap;
    flex-shrink: 0;
  }
  .tool-pick .input {
    width: auto;
    padding: 0.18rem 0.4rem;
    font-size: var(--fs-sm);
  }
  .root {
    font-family: var(--font-mono);
    flex-shrink: 0;
  }
  .root.active {
    border-color: var(--accent-line);
    background: var(--accent-soft);
    color: var(--accent-text);
  }
  .note {
    margin: 0;
    padding: 0.35rem 0.7rem;
    color: var(--text-3);
    font-size: var(--fs-xs);
    line-height: 1.45;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .tree {
    overflow-y: auto;
    overflow-x: hidden;
    padding: 0.4rem;
    flex: 1;
    min-height: 0;
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
    font-size: var(--fs-md);
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
    font-size: var(--fs-xs);
    color: var(--text-3);
    word-break: break-all;
    margin-top: 0.1rem;
  }
  .ctx-scope {
    margin-left: auto;
    flex-shrink: 0;
    font-size: var(--fs-xs);
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
    font-size: var(--fs-md);
    transition: background-color var(--t-fast);
  }
  .ctx-item:hover:not(:disabled),
  .ctx-item:focus-visible {
    background: var(--bg-3);
    outline: none;
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
    font-size: var(--fs-xs);
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
