<script lang="ts">
  import { onMount } from 'svelte';
  import { buildDiff, inTauri, loadSettings, saveSettings, type DiffView, type ScopeRules } from '$lib/ipc';
  import { app, refreshEffective, reset, markScopeSaved } from '$lib/state.svelte';
  import RuleListEditor from '$lib/components/RuleListEditor.svelte';
  import RawJsonEditor from '$lib/components/RawJsonEditor.svelte';
  import DiffViewer from '$lib/components/DiffViewer.svelte';
  import SystemExplorer from '$lib/components/SystemExplorer.svelte';
  import ClaudeSettingsPanel from '$lib/components/ClaudeSettingsPanel.svelte';
  import HelpButton from '$lib/components/HelpButton.svelte';
  import UnsavedMarker from '$lib/components/UnsavedMarker.svelte';
  import { modalFocus } from '$lib/modal';
  import { confirmDiscardChanges, isUnsavedSource } from '$lib/unsaved';
  import { toast } from '$lib/toast.svelte';

  type Mode = 'rules' | 'general' | 'raw';

  // User settings (`~/.claude/settings.json`) are global — they apply before any
  // project is selected. Editing them needs no open project, so we clear project
  // state and edit the user scope directly. `rules` edits the permissions block,
  // `general` everything else (model, env, …), `raw` the full JSON.
  let mode = $state<Mode>('rules');
  let saveSnapshot = $state<ScopeRules | null>(null);
  let diff = $state<DiffView | null>(null);
  let saving = $state(false);
  let generalDirty = $state(false);
  let generalPanel = $state<ReturnType<typeof ClaudeSettingsPanel>>();

  const modes: { id: Mode; label: string }[] = [
    { id: 'rules', label: '접근 권한' },
    { id: 'general', label: '일반 설정' },
    { id: 'raw', label: 'Raw JSON' }
  ];

  const dirty = $derived(mode === 'rules' ? app.dirty : mode === 'general' ? generalDirty : false);

  onMount(async () => {
    reset();
    await loadUserRules();
  });

  // Fold the on-disk user settings into structured rules. No project root needed:
  // the user path is resolved from the home dir regardless of the passed root.
  async function loadUserRules() {
    if (!inTauri()) return;
    try {
      const scoped = await loadSettings('');
      app.scoped.user = scoped.user;
      app.scoped.managed = scoped.managed;
      markScopeSaved('user');
      await refreshEffective();
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  // Re-sync structured rules from disk when returning to Rules mode with no unsaved
  // edits, so a Raw-JSON or general-settings save made in between is reflected
  // (and never overwritten).
  async function setMode(m: Mode) {
    if (m === mode) return;
    const localSource = mode === 'general' ? 'claude-general' : mode === 'raw' ? 'raw-settings-user' : null;
    if (
      localSource &&
      !confirmDiscardChanges(isUnsavedSource(localSource), window.confirm.bind(window))
    ) {
      return;
    }
    if (m === 'rules' && mode !== 'rules' && !app.dirty) await loadUserRules();
    if (m !== 'general') generalDirty = false;
    mode = m;
  }

  function save() {
    if (mode === 'rules') openSaveDialog();
    else if (mode === 'general') generalPanel?.openSaveDialog();
  }

  async function openSaveDialog() {
    try {
      saveSnapshot = $state.snapshot(app.scoped.user);
      diff = await buildDiff('', 'user', saveSnapshot);
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  async function confirmSave() {
    if (!diff || !saveSnapshot) return;
    saving = true;
    try {
      const snapshot = saveSnapshot;
      await saveSettings({
        projectRoot: '',
        projectId: '',
        scope: 'user',
        scopeRules: snapshot,
        expectedBefore: diff.before,
        projectName: ''
      });
      markScopeSaved('user', snapshot);
      diff = null;
      toast('사용자 설정을 저장했습니다 (백업 생성됨)');
    } catch (e) {
      toast(String(e), 'error');
    } finally {
      saving = false;
    }
  }

  // Ctrl+S works from inside inputs too; ignored while any modal is open.
  function onKey(e: KeyboardEvent) {
    if (!(e.ctrlKey || e.metaKey) || e.key.toLowerCase() !== 's') return;
    e.preventDefault();
    if (diff || document.querySelector('.modal')) return;
    if (dirty || mode === 'general') save();
  }
</script>

<svelte:window onkeydown={onKey} />
<UnsavedMarker id="user-rules" when={app.dirty} />

<main>
  <div class="top">
    <div class="title">사용자 설정</div>
    <code class="path">~/.claude/settings.json</code>

    <div class="segmented mode" role="tablist" aria-label="편집 모드">
      {#each modes as m (m.id)}
        <button
          role="tab"
          id="user-tab-{m.id}"
          aria-selected={mode === m.id}
          aria-controls="user-panel"
          onclick={() => setMode(m.id)}
        >
          {m.label}
        </button>
      {/each}
    </div>

    <HelpButton section="user" compact />

    <span class="save-slot">
      {#if mode !== 'raw'}
        <button
          class="btn"
          class:btn-primary={dirty}
          onclick={save}
          disabled={!dirty}
          title="저장 (Ctrl+S)"
        >
          {dirty ? '저장…' : '저장됨'}
        </button>
      {/if}
    </span>
  </div>

  <p class="tagline">모든 프로젝트에 공통 적용되는 전역 설정입니다. 저장 시 자동으로 백업됩니다.</p>

  {#if !inTauri()}
    <p class="callout callout-warn notice">데스크톱 앱에서만 편집할 수 있습니다 (npm run tauri dev).</p>
  {/if}

  <div class="body">
    {#if mode === 'rules'}
      <aside class="explorer">
        <SystemExplorer scope="user" />
      </aside>
    {/if}
    <div class="editor" id="user-panel" role="tabpanel" aria-labelledby="user-tab-{mode}">
      {#if mode === 'rules'}
        <RuleListEditor scope="user" />
      {:else if mode === 'general'}
        <ClaudeSettingsPanel
          bind:this={generalPanel}
          bind:dirty={generalDirty}
          onsaved={() => { if (!app.dirty) loadUserRules(); }}
        />
      {:else}
        <RawJsonEditor lockScope="user" />
      {/if}
    </div>
  </div>
</main>

{#if diff}
  <div class="modal-bg" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) diff = null; }}>
    <div
      class="modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="user-save-title"
      tabindex="-1"
      use:modalFocus={() => (diff = null)}
    >
      <h3 id="user-save-title">저장 전 변경 확인 — 사용자 설정</h3>
      <p class="fp">{diff.path}</p>
      {#if diff.changed}<DiffViewer {diff} />{:else}<p class="nochg">변경 사항이 없습니다.</p>{/if}
      <div class="modal-actions">
        <button data-modal-initial onclick={() => (diff = null)}>취소</button>
        <button class="primary" onclick={confirmSave} disabled={saving || !diff.changed}>
          {saving ? '저장 중…' : '백업 후 저장'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  main {
    height: 100vh;
    display: flex;
    flex-direction: column;
    padding: 0.8rem 1rem;
    box-sizing: border-box;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    flex-wrap: wrap;
  }
  .title {
    font-weight: 600;
    font-size: 1.05rem;
    letter-spacing: -0.01em;
    white-space: nowrap;
  }
  .path {
    min-width: 0;
    background: var(--bg-1);
    border: 1px solid var(--border);
    padding: 0.15rem 0.55rem;
    border-radius: 4px;
    font-size: var(--fs-sm);
    color: var(--text-2);
    word-break: break-all;
  }
  .mode {
    margin-left: auto;
    flex-shrink: 0;
  }
  .save-slot {
    display: inline-flex;
    justify-content: flex-end;
    min-width: 5.4rem;
    flex-shrink: 0;
  }
  .save-slot .btn {
    min-width: 5.4rem;
    font-weight: 600;
  }
  .tagline {
    color: var(--text-2);
    font-size: var(--fs-md);
    margin: 0.6rem 0;
  }
  .notice {
    margin-bottom: 0.4rem;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 0.6rem;
  }
  .explorer {
    width: clamp(220px, 30%, 320px);
    flex-shrink: 0;
    min-height: 0;
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    overflow: hidden;
    background: var(--bg-0);
  }
  .editor {
    flex: 1;
    min-width: 0;
    min-height: 0;
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    overflow: hidden;
    background: var(--bg-0);
  }
  .fp {
    color: var(--text-3);
    font-size: 0.75rem;
    font-family: var(--font-mono);
    margin: 0 0 0.6rem;
    word-break: break-all;
  }
  .nochg {
    color: var(--text-2);
  }
</style>
