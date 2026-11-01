<script lang="ts">
  // All-drives explorer for global (user) settings: pick any folder/file on the
  // PC and mark it Allow (work) / Ask / Deny (sensitive), like the project tree
  // but rooted at the machine. Selection acts on the entry's Claude pattern.
  import { onMount, tick } from 'svelte';
  import type { Policy, ScopeName, SystemEntry, Tool } from '$lib/ipc';
  import { inTauri, listDrives } from '$lib/ipc';
  import { app, refreshEffective, removeRule, upsertRule } from '$lib/state.svelte';
  import { describePattern } from '$lib/describe';
  import { tooltip } from '$lib/tooltip';
  import SystemNode from './SystemNode.svelte';

  let { scope }: { scope: ScopeName } = $props();

  let pathTool = $state<Tool>('Read');
  let drives = $state<SystemEntry[] | null>(null);
  let selected = $state<SystemEntry | null>(null);
  let error = $state<string | null>(null);
  let menu = $state<{ entry: SystemEntry; x: number; y: number } | null>(null);
  let menuEl = $state<HTMLDivElement | null>(null);
  let menuReturnFocus: HTMLElement | null = null;

  onMount(async () => {
    try {
      drives = await listDrives();
    } catch (e) {
      error = String(e);
      drives = [];
    }
  });

  // The pattern a rule on an entry uses (folders recurse, files exact).
  function rulePattern(entry: SystemEntry): string {
    return entry.isDir ? entry.pattern + '/**' : entry.pattern;
  }

  // Explicit rule on this entry, if any (folder rule `base/**` or exact `base`).
  function ruleFor(entry: SystemEntry) {
    return app.scoped[scope].rules.find(
      (r) => r.path === entry.pattern || r.path === entry.pattern + '/**'
    );
  }

  const selPattern = $derived(selected ? rulePattern(selected) : '');
  const selRule = $derived(selected ? ruleFor(selected) : undefined);

  async function applyTo(entry: SystemEntry, policy: Policy) {
    menu = null;
    upsertRule(scope, { path: rulePattern(entry), policy, appliesTo: 'pattern' });
    await refreshEffective();
  }

  async function clearFor(entry: SystemEntry) {
    menu = null;
    removeRule(scope, entry.pattern);
    removeRule(scope, entry.pattern + '/**');
    await refreshEffective();
  }

  async function openMenu(entry: SystemEntry, x: number, y: number) {
    menuReturnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    menu = { entry, x, y };
    await tick();
    if (!menuEl || !menu) return;
    const r = menuEl.getBoundingClientRect();
    menu.x = Math.max(8, Math.min(x, window.innerWidth - r.width - 8));
    menu.y = Math.max(8, Math.min(y, window.innerHeight - r.height - 8));
    menuItems()[0]?.focus();
  }

  function menuItems(): HTMLButtonElement[] {
    return menuEl ? [...menuEl.querySelectorAll<HTMLButtonElement>('[role="menuitem"]:not(:disabled)')] : [];
  }

  function closeMenu(restoreFocus = false) {
    menu = null;
    if (restoreFocus) menuReturnFocus?.focus();
    menuReturnFocus = null;
  }

  function onMenuKeydown(e: KeyboardEvent) {
    const items = menuItems();
    const i = items.indexOf(document.activeElement as HTMLButtonElement);
    let next: number | null = null;
    if (e.key === 'ArrowDown') next = (i + 1) % items.length;
    else if (e.key === 'ArrowUp') next = (i - 1 + items.length) % items.length;
    else if (e.key === 'Home') next = 0;
    else if (e.key === 'End') next = items.length - 1;
    else if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      closeMenu(true);
      return;
    } else if (e.key === 'Tab') {
      closeMenu();
      return;
    }
    if (next !== null && items.length) {
      e.preventDefault();
      items[next].focus();
    }
  }
</script>

<div class="panel">
  <div class="head">
    <span>내 PC</span>
    <label class="tool-pick">
      규칙 색상
      <select class="input" bind:value={pathTool}>
        <option value="Read">Read</option>
        <option value="Edit">Edit</option>
      </select>
    </label>
  </div>

  <div class="selbar">
    {#if selected}
      <div class="selname" use:tooltip={describePattern(selPattern)}>
        {selected.name}
      </div>
      <code class="selpat">{selPattern}</code>
      <div class="selbtns">
        <button
          class="allow"
          class:active={selRule?.policy === 'allow'}
          aria-pressed={selRule?.policy === 'allow'}
          onclick={() => selected && applyTo(selected, 'allow')}
        >
          작업 Allow
        </button>
        <button
          class="ask"
          class:active={selRule?.policy === 'ask'}
          aria-pressed={selRule?.policy === 'ask'}
          onclick={() => selected && applyTo(selected, 'ask')}
        >
          Ask
        </button>
        <button
          class="deny"
          class:active={selRule?.policy === 'deny'}
          aria-pressed={selRule?.policy === 'deny'}
          onclick={() => selected && applyTo(selected, 'deny')}
        >
          민감 Deny
        </button>
        <button class="clear" onclick={() => selected && clearFor(selected)} disabled={!selRule}>
          해제
        </button>
      </div>
    {:else}
      <p class="hint">폴더/파일을 선택하거나 우클릭해 작업(Allow)·민감(Deny) 정책을 지정하세요.</p>
    {/if}
  </div>

  <div class="tree" role="tree" aria-label="내 PC 폴더 트리">
    {#if drives === null}
      <p class="muted">드라이브를 찾는 중…</p>
    {:else if error}
      <p class="callout callout-error" role="alert">{error}</p>
    {:else if drives.length === 0}
      <p class="muted">
        {inTauri() ? '드라이브가 없습니다.' : '데스크톱 앱에서 실행하면 드라이브가 표시됩니다.'}
      </p>
    {:else}
      {#each drives as d (d.path)}
        <SystemNode
          entry={d}
          {scope}
          {pathTool}
          selectedPath={selected?.path ?? ''}
          onselect={(e) => (selected = e)}
          oncontext={openMenu}
        />
      {/each}
    {/if}
  </div>

  <div class="legend" aria-label="색상 범례">
    <span><i class="lg lg-allow"></i>Allow 규칙</span>
    <span><i class="lg lg-ask"></i>Ask 규칙</span>
    <span><i class="lg lg-deny"></i>Deny 규칙</span>
    <span
      use:tooltip={'명시 규칙 비교입니다. 실제 승인 결과는 실행 환경에 따라 달라집니다. 회색은 규칙 없음 또는 판정 불가입니다.'}
    >
      <i class="lg lg-none"></i>없음 / 판정 불가
    </span>
  </div>
</div>

<svelte:window
  onclick={() => menu && closeMenu()}
  oncontextmenu={() => menu && closeMenu()}
  onkeydown={(e) => e.key === 'Escape' && menu && closeMenu(true)}
  onresize={() => menu && closeMenu()}
/>

{#if menu}
  {@const m = menu}
  {@const mRule = ruleFor(m.entry)}
  <div
    class="ctx"
    bind:this={menuEl}
    style="left: {m.x}px; top: {m.y}px"
    role="menu"
    tabindex="-1"
    aria-label="{m.entry.name} 정책 지정"
    onkeydown={onMenuKeydown}
  >
    <div class="ctx-head">
      <span class="ctx-name">{m.entry.name}</span>
      <code class="ctx-pat">{rulePattern(m.entry)}</code>
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
      <span class="ctx-label">Deny<span class="ctx-sub">민감 폴더 — 접근 Deny 규칙</span></span>
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
    padding: 0.55rem 0.7rem;
    border-bottom: 1px solid var(--border);
    font-size: var(--fs-md);
    font-weight: 600;
    letter-spacing: 0.02em;
    color: var(--text-2);
    flex-shrink: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.3rem 0.6rem;
  }
  .tool-pick {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    font-weight: 500;
    color: var(--text-3);
    white-space: nowrap;
  }
  .tool-pick select {
    width: auto;
    padding: 0.15rem 0.4rem;
    font-size: var(--fs-sm);
  }
  .selbar {
    padding: 0.5rem 0.6rem;
    border-bottom: 1px solid var(--border);
    background: var(--bg-1);
    flex-shrink: 0;
  }
  .selname {
    font-size: 0.8rem;
    font-weight: 600;
    width: fit-content;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .selpat {
    display: block;
    font-size: 0.7rem;
    color: var(--text-3);
    margin: 0.15rem 0 0.4rem;
    word-break: break-all;
  }
  .selbtns {
    display: flex;
    gap: 0.3rem;
    flex-wrap: wrap;
  }
  .selbtns button {
    padding: 0.28rem 0.55rem;
    border-radius: var(--r-sm);
    cursor: pointer;
    font-size: 0.72rem;
    font-weight: 600;
    white-space: nowrap;
    background: var(--bg-2);
    border: 1px solid var(--border-strong);
    color: var(--text-2);
    transition: background-color var(--t-fast), border-color var(--t-fast), color var(--t-fast);
  }
  .selbtns .allow {
    border-color: var(--allow-line);
    color: var(--allow);
  }
  .selbtns .allow:hover,
  .selbtns .allow.active {
    background: var(--allow-soft);
  }
  .selbtns .ask {
    border-color: var(--ask-line);
    color: var(--ask);
  }
  .selbtns .ask:hover,
  .selbtns .ask.active {
    background: var(--ask-soft);
  }
  .selbtns .deny {
    border-color: var(--deny-line);
    color: var(--deny);
  }
  .selbtns .deny:hover,
  .selbtns .deny.active {
    background: var(--deny-soft);
  }
  .selbtns .clear:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .selbtns .clear:hover:not(:disabled) {
    background: var(--bg-3);
  }
  .hint {
    color: var(--text-3);
    font-size: 0.74rem;
    margin: 0;
    line-height: 1.5;
  }
  .tree {
    overflow: auto;
    padding: 0.4rem;
    flex: 1;
  }
  .muted {
    color: var(--text-3);
    font-size: 0.78rem;
    padding: 0 0.3rem;
  }
  .legend {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    flex-wrap: wrap;
    padding: 0.4rem 0.7rem;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }
  .legend span {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.68rem;
    color: var(--text-3);
    white-space: nowrap;
  }
  .lg {
    width: 7px;
    height: 7px;
    border-radius: 999px;
    flex-shrink: 0;
  }
  .lg-allow {
    background: var(--allow);
  }
  .lg-ask {
    background: var(--ask);
  }
  .lg-deny {
    background: var(--deny);
  }
  .lg-none {
    border: 1px solid var(--border-strong);
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
    display: block;
    font-size: 0.66rem;
    color: var(--text-3);
    word-break: break-all;
    margin-top: 0.1rem;
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
  .ctx-item:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
  .ctx-item:hover:not(:disabled),
  .ctx-item:focus:not(:disabled) {
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
