<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { buildDiff, saveSettings, type DiffView, type ScopeName } from '$lib/ipc';
  import { app, refreshEffective, setDefaultMode } from '$lib/state.svelte';
  import FileExplorer from '$lib/components/FileExplorer.svelte';
  import PolicyEditor from '$lib/components/PolicyEditor.svelte';
  import EffectivePreview from '$lib/components/EffectivePreview.svelte';
  import DiffViewer from '$lib/components/DiffViewer.svelte';

  onMount(() => {
    if (!app.loaded) goto('/');
  });

  let saving = $state(false);
  let error = $state<string | null>(null);
  let diff = $state<DiffView | null>(null);
  let saveScope = $state<ScopeName>('project');

  const dontAsk = $derived(app.scoped[app.activeScope].defaultMode === 'dontAsk');

  function toggleDefaultDeny() {
    setDefaultMode(app.activeScope, dontAsk ? null : 'dontAsk');
    refreshEffective();
  }

  async function openSaveDialog() {
    error = null;
    saveScope = app.activeScope;
    try {
      diff = await buildDiff(app.projectRoot, saveScope, app.scoped[saveScope]);
    } catch (e) {
      error = String(e);
    }
  }

  async function confirmSave() {
    if (!diff) return;
    saving = true;
    error = null;
    try {
      await saveSettings({
        projectRoot: app.projectRoot,
        projectId: app.projectId,
        scope: saveScope,
        scopeRules: app.scoped[saveScope],
        projectName: app.projectName
      });
      app.dirty = false;
      diff = null;
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<div class="top">
  <button class="back" onclick={() => goto('/')}>← Home</button>
  <div class="title">
    {app.projectName}
    {#if app.view?.risk}
      <span class="risk risk-{app.view.risk.level.toLowerCase()}">
        Risk {app.view.risk.level} ({app.view.risk.score})
      </span>
    {/if}
  </div>
  <label class="dd">
    <input type="checkbox" checked={dontAsk} onchange={toggleDefaultDeny} />
    Default Deny ({app.activeScope})
  </label>
  <button class="save" onclick={openSaveDialog} disabled={!app.dirty}>
    {app.dirty ? '● Save…' : 'Saved'}
  </button>
</div>

{#if error}<div class="err">{error}</div>{/if}

<div class="cols">
  <section class="left"><FileExplorer /></section>
  <section class="mid"><PolicyEditor /></section>
  <section class="right"><EffectivePreview /></section>
</div>

{#if diff}
  <div class="modal-bg" role="presentation" onclick={() => (diff = null)}>
    <div class="modal" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
      <h3>저장 전 변경 확인 — {saveScope} scope</h3>
      <p class="fp">{diff.path}</p>
      {#if diff.changed}
        <DiffViewer {diff} />
      {:else}
        <p class="nochg">변경 사항이 없습니다.</p>
      {/if}
      <div class="modal-actions">
        <button onclick={() => (diff = null)}>취소</button>
        <button class="primary" onclick={confirmSave} disabled={saving || !diff.changed}>
          {saving ? '저장 중…' : '백업 후 저장'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .top {
    display: flex; align-items: center; gap: 0.75rem; padding: 0.5rem 0.8rem;
    border-bottom: 1px solid #1e293b; background: #0b1220;
  }
  .back { background: none; border: 1px solid #334155; color: #94a3b8; border-radius: 6px; padding: 0.3rem 0.6rem; cursor: pointer; }
  .title { font-weight: 600; display: flex; align-items: center; gap: 0.5rem; }
  .risk { font-size: 0.7rem; padding: 0.1rem 0.5rem; border-radius: 999px; }
  .risk-high { background: #7f1d1d; color: #fecaca; }
  .risk-medium { background: #78350f; color: #fde68a; }
  .risk-low { background: #14532d; color: #bbf7d0; }
  .dd { margin-left: auto; font-size: 0.78rem; color: #94a3b8; display: flex; align-items: center; gap: 0.3rem; }
  .save { background: #2563eb; border: none; color: white; border-radius: 6px; padding: 0.4rem 0.9rem; cursor: pointer; }
  .save:disabled { background: #1e293b; color: #64748b; cursor: default; }
  .err { background: #7f1d1d; color: #fecaca; padding: 0.4rem 0.8rem; font-size: 0.8rem; }
  .cols { display: grid; grid-template-columns: 1fr 1fr 1fr; height: calc(100vh - 46px); }
  .left { border-right: 1px solid #1e293b; }
  .mid { border-right: 1px solid #1e293b; overflow: auto; }
  .cols section { min-height: 0; }
  .modal-bg { position: fixed; inset: 0; background: #000a; display: flex; align-items: center; justify-content: center; padding: 2rem; }
  .modal { background: #0f172a; border: 1px solid #334155; border-radius: 10px; padding: 1rem; width: min(900px, 95vw); }
  .modal h3 { margin: 0 0 0.25rem; font-size: 1rem; }
  .fp { color: #64748b; font-size: 0.75rem; margin: 0 0 0.6rem; }
  .nochg { color: #94a3b8; }
  .modal-actions { display: flex; justify-content: flex-end; gap: 0.5rem; margin-top: 0.8rem; }
  .modal-actions button { padding: 0.45rem 0.9rem; border-radius: 6px; border: 1px solid #334155; background: #1e293b; color: #e2e8f0; cursor: pointer; }
  .modal-actions .primary { background: #2563eb; border-color: #2563eb; }
  .modal-actions .primary:disabled { opacity: 0.5; cursor: default; }
</style>
