<script lang="ts">
  import type { AppliesTo, Policy, ScopeName } from '$lib/ipc';
  import { app, clearPolicy, refreshEffective, setPolicy } from '$lib/state.svelte';

  let appliesTo = $state<AppliesTo>('folder-and-children');

  const target = $derived(app.selectedPath || '(project root)');

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
    <code>{target}</code>
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
    <code>{current ? current.policy.toUpperCase() : '— (untracked)'}</code>
  </div>

  <div class="buttons">
    <button class="allow" onclick={() => apply('allow')} disabled={!app.selectedPath}>Allow</button>
    <button class="ask" onclick={() => apply('ask')} disabled={!app.selectedPath}>Ask</button>
    <button class="deny" onclick={() => apply('deny')} disabled={!app.selectedPath}>Deny</button>
    <button class="clear" onclick={clear} disabled={!current}>Clear rule</button>
  </div>

  {#if !app.selectedPath}
    <p class="hint">왼쪽 트리에서 파일/폴더를 선택하세요.</p>
  {/if}
</div>

<style>
  .panel { padding: 0.75rem; }
  h3 { margin: 0 0 0.75rem; font-size: 0.95rem; }
  .field { display: block; margin-bottom: 0.75rem; }
  .field > span { display: block; font-size: 0.72rem; color: #94a3b8; margin-bottom: 0.25rem; }
  code { background: #0b1220; padding: 0.2rem 0.4rem; border-radius: 4px; font-size: 0.8rem; display: inline-block; }
  select { width: 100%; padding: 0.4rem; background: #0b1220; color: #e2e8f0; border: 1px solid #334155; border-radius: 6px; }
  .segmented { display: flex; gap: 0.3rem; }
  .segmented button {
    flex: 1; padding: 0.35rem; background: #0b1220; border: 1px solid #334155; color: #94a3b8;
    border-radius: 6px; cursor: pointer; text-transform: capitalize;
  }
  .segmented button.active { border-color: #2563eb; color: #93c5fd; }
  .buttons { display: grid; grid-template-columns: 1fr 1fr; gap: 0.4rem; margin-top: 0.5rem; }
  .buttons button { padding: 0.5rem; border-radius: 6px; border: 1px solid #334155; cursor: pointer; color: #e2e8f0; background: #1e293b; }
  .buttons button:disabled { opacity: 0.45; cursor: default; }
  .allow { border-color: #14532d !important; }
  .ask { border-color: #78350f !important; }
  .deny { border-color: #7f1d1d !important; }
  .hint { color: #64748b; font-size: 0.8rem; }
</style>
