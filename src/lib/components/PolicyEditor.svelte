<script lang="ts">
  import { inTauri, noteIgnoredPath, pathIgnored, type AppliesTo, type Policy, type ScopeName } from '$lib/ipc';
  import { app, clearPolicy, refreshEffective, setPolicy } from '$lib/state.svelte';

  let appliesTo = $state<AppliesTo>('folder-and-children');

  const target = $derived(app.selectedPath || '(project root)');

  // Git-ignored paths need a heads-up: an Allow rule opens access, but the
  // agent's Grep search still skips them.
  let ignored = $state(false);
  let noteStatus = $state<string | null>(null);

  $effect(() => {
    const p = app.selectedPath;
    ignored = false;
    noteStatus = null;
    if (!p || !inTauri()) return;
    pathIgnored(app.projectRoot, p)
      .then((v) => (ignored = v))
      .catch(() => {});
  });

  async function noteInClaudeMd() {
    try {
      const added = await noteIgnoredPath(app.projectRoot, app.selectedPath);
      noteStatus = added
        ? 'CLAUDE.md에 안내를 추가했습니다 — 에이전트가 이 경로를 인지합니다.'
        : 'CLAUDE.md에 이미 안내가 있습니다.';
    } catch (e) {
      noteStatus = String(e);
    }
  }

  // The explicit rule currently on this path in the active scope, if any.
  const current = $derived(
    app.scoped[app.activeScope].rules.find((r) => r.path === app.selectedPath)
  );

  async function apply(policy: Policy) {
    if (!app.selectedPath) return;
    setPolicy(app.selectedPath, policy, appliesTo);
    await refreshEffective();
  }

  async function clear() {
    if (!app.selectedPath) return;
    clearPolicy(app.selectedPath);
    await refreshEffective();
  }

  const scopes: ScopeName[] = ['project', 'local'];
</script>

<div class="panel">
  <h3>Policy Editor</h3>

  <div class="field">
    <span>Path</span>
    <code class="target">{target}</code>
  </div>

  <div class="field">
    <span>Scope</span>
    <div class="segmented">
      {#each scopes as s (s)}
        <button class:active={app.activeScope === s} onclick={() => (app.activeScope = s)}>{s}</button>
      {/each}
    </div>
  </div>

  <label class="field">
    <span>Applies to</span>
    <select bind:value={appliesTo} disabled={!app.selectedPath}>
      <option value="file">This file only</option>
      <option value="folder">This folder only</option>
      <option value="folder-and-children">This folder and children</option>
      <option value="pattern">Matching pattern</option>
    </select>
  </label>

  <div class="field">
    <span>Current (this scope)</span>
    {#if current}
      <span class="cur cur-{current.policy}">{current.policy.toUpperCase()}</span>
    {:else}
      <span class="cur cur-none">— untracked</span>
    {/if}
  </div>

  <div class="buttons">
    <button class="allow" onclick={() => apply('allow')} disabled={!app.selectedPath}>
      Allow <kbd>A</kbd>
    </button>
    <button class="ask" onclick={() => apply('ask')} disabled={!app.selectedPath}>
      Ask <kbd>K</kbd>
    </button>
    <button class="deny" onclick={() => apply('deny')} disabled={!app.selectedPath}>
      Deny <kbd>D</kbd>
    </button>
    <button class="clear" onclick={clear} disabled={!current}>Clear rule</button>
  </div>

  {#if ignored}
    <div class="ginote">
      <b>.gitignore에 포함된 경로입니다.</b> 차단된 것은 아닙니다 —
      Grep 검색으로는 <b>발견 못 함</b> · 경로를 알면 Read <b>가능</b> · 접근 권한은
      Allow/Ask/Deny 규칙이 결정. CLAUDE.md에 경로를 알려주면 에이전트가 직접 읽거나
      <code>rg --no-ignore</code>로 검색할 수 있습니다.
      <button class="gbtn" onclick={noteInClaudeMd}>CLAUDE.md에 알리기</button>
      {#if noteStatus}<span class="gstat">{noteStatus}</span>{/if}
    </div>
  {/if}

  {#if !app.selectedPath}
    <p class="hint">왼쪽 트리에서 파일/폴더를 선택하세요.</p>
  {/if}
</div>

<style>
  .panel {
    padding: 0.85rem;
  }
  h3 {
    margin: 0 0 0.9rem;
    font-size: 0.8rem;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--text-2);
  }
  .field {
    display: block;
    margin-bottom: 0.85rem;
  }
  .field > span {
    display: block;
    font-size: 0.68rem;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-3);
    margin-bottom: 0.3rem;
  }
  .target {
    background: var(--bg-1);
    border: 1px solid var(--border);
    padding: 0.3rem 0.5rem;
    border-radius: var(--r-sm);
    font-size: 0.79rem;
    display: block;
    word-break: break-all;
    color: var(--text-1);
  }
  select {
    width: 100%;
    padding: 0.45rem;
    background: var(--bg-1);
    color: var(--text-1);
    border: 1px solid var(--border-strong);
    border-radius: var(--r-sm);
    font-size: 0.84rem;
  }
  select:disabled {
    opacity: 0.5;
  }
  .segmented {
    display: flex;
    background: var(--bg-1);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    padding: 0.18rem;
    gap: 0.18rem;
  }
  .segmented button {
    flex: 1;
    padding: 0.32rem;
    background: transparent;
    border: none;
    color: var(--text-2);
    border-radius: 4px;
    cursor: pointer;
    text-transform: capitalize;
    font-size: 0.8rem;
    transition: background-color var(--t-fast), color var(--t-fast);
  }
  .segmented button:hover {
    color: var(--text-1);
  }
  .segmented button.active {
    background: var(--bg-3);
    color: var(--accent-text);
    box-shadow: var(--shadow-1);
  }
  .cur {
    display: inline-block;
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    padding: 0.18rem 0.6rem;
    border-radius: 999px;
  }
  .cur-allow {
    background: var(--allow-soft);
    color: var(--allow);
    border: 1px solid rgba(52, 211, 153, 0.3);
  }
  .cur-ask {
    background: var(--ask-soft);
    color: var(--ask);
    border: 1px solid rgba(251, 191, 36, 0.3);
  }
  .cur-deny {
    background: var(--deny-soft);
    color: var(--deny);
    border: 1px solid rgba(248, 113, 113, 0.3);
  }
  .cur-none {
    color: var(--text-3);
    border: 1px dashed var(--border-strong);
    font-weight: 500;
  }
  .buttons {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.45rem;
    margin-top: 0.6rem;
  }
  .buttons button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    padding: 0.55rem;
    border-radius: var(--r-sm);
    border: 1px solid var(--border-strong);
    cursor: pointer;
    color: var(--text-1);
    background: var(--bg-2);
    font-size: 0.85rem;
    font-weight: 600;
    transition: background-color var(--t-fast), border-color var(--t-fast), box-shadow var(--t-fast);
  }
  .buttons button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .buttons kbd {
    font-size: 0.62rem;
    padding: 0 0.3rem;
    opacity: 0.7;
  }
  .allow {
    border-color: rgba(52, 211, 153, 0.35);
    color: var(--allow);
    background: var(--allow-soft);
  }
  .allow:hover:not(:disabled) {
    background: rgba(52, 211, 153, 0.22);
    box-shadow: 0 0 14px rgba(52, 211, 153, 0.15);
  }
  .ask {
    border-color: rgba(251, 191, 36, 0.35);
    color: var(--ask);
    background: var(--ask-soft);
  }
  .ask:hover:not(:disabled) {
    background: rgba(251, 191, 36, 0.22);
    box-shadow: 0 0 14px rgba(251, 191, 36, 0.15);
  }
  .deny {
    border-color: rgba(248, 113, 113, 0.35);
    color: var(--deny);
    background: var(--deny-soft);
  }
  .deny:hover:not(:disabled) {
    background: rgba(248, 113, 113, 0.22);
    box-shadow: 0 0 14px rgba(248, 113, 113, 0.15);
  }
  .clear:hover:not(:disabled) {
    background: var(--bg-3);
  }
  .hint {
    color: var(--text-3);
    font-size: 0.8rem;
    margin-top: 0.8rem;
  }
  .ginote {
    margin-top: 0.9rem;
    background: var(--ask-soft);
    border: 1px solid rgba(251, 191, 36, 0.3);
    border-radius: var(--r-sm);
    padding: 0.55rem 0.7rem;
    font-size: 0.76rem;
    color: var(--text-2);
    line-height: 1.55;
  }
  .ginote b {
    color: var(--ask);
  }
  .ginote code {
    background: var(--bg-1);
    border: 1px solid var(--border);
    padding: 0 0.3rem;
    border-radius: 3px;
    font-size: 0.72rem;
  }
  .gbtn {
    display: block;
    margin-top: 0.5rem;
    background: rgba(251, 191, 36, 0.15);
    border: 1px solid rgba(251, 191, 36, 0.35);
    color: var(--ask);
    border-radius: var(--r-sm);
    padding: 0.3rem 0.7rem;
    cursor: pointer;
    font-size: 0.74rem;
    font-weight: 600;
    transition: background-color var(--t-fast);
  }
  .gbtn:hover {
    background: rgba(251, 191, 36, 0.25);
  }
  .gstat {
    display: block;
    margin-top: 0.4rem;
    color: var(--accent-text);
    font-size: 0.72rem;
  }
</style>
