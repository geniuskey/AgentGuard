<script lang="ts">
  // Raw JSON editor for one scope (req §8.8). Validate / Format / Save.
  // NOTE: a lightweight textarea editor with live JSON validation. Monaco (the
  // documented choice in docs/tech-stack.md) is deferred as a visual polish item —
  // it cannot be verified in this headless environment and needs CSP worker-src
  // tuning; the functional contract (validate/format/save/diff/restore) is complete.
  import type { ScopeName } from '$lib/ipc';
  import { readRawSettings, saveRawSettings, validateJson } from '$lib/ipc';
  import { app } from '$lib/state.svelte';
  import UnsavedMarker from '$lib/components/UnsavedMarker.svelte';
  import { confirmDiscardChanges } from '$lib/unsaved';

  // `lockScope` pins the editor to one scope (used by the project-less User settings
  // page) and hides the scope switcher.
  let { lockScope }: { lockScope?: ScopeName } = $props();

  let picked = $state<ScopeName>('project');
  const scope = $derived(lockScope ?? picked);
  let text = $state('');
  let onDisk = $state('');
  let error = $state<string | null>(null);
  let status = $state<string | null>(null);
  let loading = $state(false);
  let loadSequence = 0;

  const dirty = $derived(!loading && text !== onDisk);
  const unsavedId = $derived(`raw-settings-${lockScope ?? 'project'}`);

  const scopes: ScopeName[] = ['user', 'project', 'local'];

  async function load(target: ScopeName) {
    const sequence = ++loadSequence;
    loading = true;
    status = null;
    error = null;
    try {
      const loaded = await readRawSettings(app.projectRoot, target);
      if (sequence !== loadSequence) return;
      text = loaded;
      onDisk = loaded;
    } catch (e) {
      error = String(e);
    } finally {
      if (sequence === loadSequence) loading = false;
    }
  }

  $effect(() => {
    // Reload whenever the scope changes (or on first mount).
    const target = scope;
    load(target);
  });

  function selectScope(next: ScopeName) {
    if (next === scope) return;
    if (!confirmDiscardChanges(dirty, window.confirm.bind(window))) return;
    picked = next;
  }

  async function validate() {
    error = await validateJson(text);
    status = error ? null : '유효한 JSON입니다.';
  }

  function format() {
    try {
      text = JSON.stringify(JSON.parse(text || '{}'), null, 2) + '\n';
      error = null;
      status = '포맷 완료';
    } catch (e) {
      error = String(e);
    }
  }

  async function save() {
    const v = await validateJson(text);
    if (v) {
      error = v;
      return;
    }
    try {
      const res = await saveRawSettings({
        projectRoot: app.projectRoot,
        projectId: app.projectId,
        scope,
        text,
        projectName: app.projectName
      });
      status = `저장됨 → ${res.written}`;
      error = null;
      onDisk = text;
    } catch (e) {
      error = String(e);
    }
  }
</script>

<UnsavedMarker id={unsavedId} when={dirty} />

<div class="panel">
  <div class="bar">
    {#if lockScope}
      <div class="scope-label">{lockScope} scope</div>
    {:else}
      <div class="segmented">
        {#each scopes as s (s)}
          <button class:active={scope === s} onclick={() => selectScope(s)}>{s}</button>
        {/each}
      </div>
    {/if}
    <div class="tools">
      <button onclick={validate}>Validate</button>
      <button onclick={format}>Format</button>
      <button class="primary" class:dirty onclick={save} disabled={!dirty}>
        {dirty ? 'Save' : 'Saved'}
      </button>
    </div>
  </div>

  {#if loading}
    <p class="muted">불러오는 중…</p>
  {:else}
    <textarea bind:value={text} spellcheck="false" placeholder="{'{}'}"></textarea>
  {/if}

  {#if error}<p class="err" role="alert">JSON 오류: {error}</p>{/if}
  {#if status}<p class="ok" role="status">{status}</p>{/if}
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 0.5rem;
    box-sizing: border-box;
  }
  .bar {
    display: flex;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }
  .segmented {
    display: flex;
    background: var(--bg-1);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    padding: 0.15rem;
    gap: 0.15rem;
  }
  .segmented button {
    padding: 0.2rem 0.55rem;
    background: transparent;
    border: none;
    color: var(--text-2);
    border-radius: 4px;
    cursor: pointer;
    text-transform: capitalize;
    font-size: 0.75rem;
    transition: background-color var(--t-fast), color var(--t-fast);
  }
  .segmented button:hover {
    color: var(--text-1);
  }
  .segmented button.active {
    background: var(--bg-3);
    color: var(--accent-text);
  }
  .scope-label {
    font-size: 0.72rem;
    font-weight: 600;
    color: var(--accent-text);
    background: var(--accent-soft);
    border: 1px solid rgba(79, 142, 247, 0.3);
    border-radius: 999px;
    padding: 0.1rem 0.6rem;
    text-transform: capitalize;
    align-self: center;
  }
  .tools {
    display: flex;
    gap: 0.35rem;
    min-width: 0;
    max-width: 100%;
    overflow-x: auto;
  }
  .tools button {
    flex-shrink: 0;
    padding: 0.28rem 0.65rem;
    background: var(--bg-2);
    border: 1px solid var(--border-strong);
    color: var(--text-1);
    border-radius: var(--r-sm);
    cursor: pointer;
    font-size: 0.75rem;
    transition: background-color var(--t-fast), border-color var(--t-fast);
  }
  .tools button:hover {
    background: var(--bg-3);
  }
  .tools .primary {
    background: var(--accent-strong);
    border-color: var(--accent-strong);
    color: white;
    font-weight: 600;
  }
  .tools .primary:hover {
    background: #3b76ee;
  }
  .tools .primary:not(.dirty) {
    background: var(--bg-2);
    border-color: var(--border);
    color: var(--text-3);
    cursor: default;
  }
  textarea {
    flex: 1;
    width: 100%;
    box-sizing: border-box;
    resize: none;
    background: var(--bg-1);
    color: var(--text-1);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    padding: 0.6rem;
    font-family: var(--font-mono);
    font-size: 0.78rem;
    line-height: 1.5;
    transition: border-color var(--t-fast);
  }
  textarea:focus {
    border-color: var(--accent);
    outline: none;
  }
  .muted {
    color: var(--text-3);
  }
  .err {
    color: var(--deny);
    font-size: 0.78rem;
    margin: 0.4rem 0 0;
  }
  .ok {
    color: var(--allow);
    font-size: 0.78rem;
    margin: 0.4rem 0 0;
  }
</style>
