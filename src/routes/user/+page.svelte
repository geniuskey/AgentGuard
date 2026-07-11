<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { buildDiff, inTauri, loadSettings, saveSettings, type DiffView } from '$lib/ipc';
  import { app, refreshEffective, reset } from '$lib/state.svelte';
  import RuleListEditor from '$lib/components/RuleListEditor.svelte';
  import RawJsonEditor from '$lib/components/RawJsonEditor.svelte';
  import DiffViewer from '$lib/components/DiffViewer.svelte';
  import SystemExplorer from '$lib/components/SystemExplorer.svelte';

  // User settings (`~/.claude/settings.json`) are global — they apply before any
  // project is selected. Editing them needs no open project, so we clear project
  // state and edit the user scope directly.
  let mode = $state<'rules' | 'raw'>('rules');
  let error = $state<string | null>(null);
  let diff = $state<DiffView | null>(null);
  let saving = $state(false);

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
      app.dirty = false;
      await refreshEffective();
    } catch (e) {
      error = String(e);
    }
  }

  // Re-sync structured rules from disk when returning to Rules mode with no unsaved
  // edits, so a Raw-JSON save made in between is reflected (and never overwritten).
  async function setMode(m: 'rules' | 'raw') {
    if (m === 'rules' && mode !== 'rules' && !app.dirty) await loadUserRules();
    mode = m;
  }

  async function openSaveDialog() {
    error = null;
    try {
      diff = await buildDiff('', 'user', app.scoped.user);
    } catch (e) {
      error = String(e);
    }
  }

  async function confirmSave() {
    if (!diff) return;
    saving = true;
    try {
      await saveSettings({
        projectRoot: '',
        projectId: '',
        scope: 'user',
        scopeRules: app.scoped.user,
        projectName: ''
      });
      app.dirty = false;
      diff = null;
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  function onKey(e: KeyboardEvent) {
    const t = e.target as HTMLElement | null;
    if (t && ['INPUT', 'TEXTAREA', 'SELECT'].includes(t.tagName)) return;
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
      e.preventDefault();
      if (mode === 'rules' && app.dirty) openSaveDialog();
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<main>
  <div class="top">
    <button class="back" onclick={() => goto('/')} aria-label="홈으로">←</button>
    <div class="title">사용자 설정</div>
    <code class="path">~/.claude/settings.json</code>

    <div class="mode">
      <button class:active={mode === 'rules'} onclick={() => setMode('rules')}>규칙</button>
      <button class:active={mode === 'raw'} onclick={() => setMode('raw')}>Raw JSON</button>
    </div>

    {#if mode === 'rules'}
      <button class="save" class:dirty={app.dirty} onclick={openSaveDialog} disabled={!app.dirty}>
        {#if app.dirty}<span class="save-dot" aria-hidden="true"></span>저장…{:else}저장됨{/if}
      </button>
    {/if}
  </div>

  <p class="tagline">
    프로젝트와 무관하게 모든 프로젝트에 공통 적용되는 전역 설정입니다. 저장 시 자동으로 백업됩니다.
  </p>

  {#if !inTauri()}
    <p class="hint">데스크톱 앱에서만 편집할 수 있습니다 (npm run tauri dev).</p>
  {/if}
  {#if error}<div class="err" role="alert">{error}</div>{/if}

  <div class="body">
    {#if mode === 'rules'}
      <aside class="explorer">
        <SystemExplorer scope="user" />
      </aside>
    {/if}
    <div class="editor">
      {#if mode === 'rules'}
        <RuleListEditor scope="user" />
      {:else}
        <RawJsonEditor lockScope="user" />
      {/if}
    </div>
  </div>
</main>

{#if diff}
  <div class="modal-bg" role="presentation" onclick={() => (diff = null)}>
    <div class="modal" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
      <h3>저장 전 변경 확인 — user scope</h3>
      <p class="fp">{diff.path}</p>
      {#if diff.changed}<DiffViewer {diff} />{:else}<p class="nochg">변경 사항이 없습니다.</p>{/if}
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
  .back {
    background: none;
    border: 1px solid var(--border-strong);
    color: var(--text-2);
    border-radius: var(--r-sm);
    padding: 0.28rem 0.6rem;
    cursor: pointer;
    transition: color var(--t-fast), background-color var(--t-fast);
  }
  .back:hover {
    color: var(--text-1);
    background: var(--bg-2);
  }
  .title {
    font-weight: 600;
    font-size: 1.05rem;
    letter-spacing: -0.01em;
  }
  .path {
    background: var(--bg-1);
    border: 1px solid var(--border);
    padding: 0.15rem 0.55rem;
    border-radius: 4px;
    font-size: 0.76rem;
    color: var(--text-2);
  }
  .mode {
    display: flex;
    background: var(--bg-1);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    padding: 0.15rem;
    gap: 0.15rem;
    margin-left: auto;
  }
  .mode button {
    padding: 0.22rem 0.7rem;
    background: transparent;
    border: none;
    color: var(--text-2);
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.78rem;
    transition: background-color var(--t-fast), color var(--t-fast);
  }
  .mode button:hover {
    color: var(--text-1);
  }
  .mode button.active {
    background: var(--bg-3);
    color: var(--accent-text);
  }
  .save {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    background: var(--bg-2);
    border: 1px solid var(--border);
    color: var(--text-3);
    border-radius: var(--r-sm);
    padding: 0.35rem 0.95rem;
    font-weight: 600;
    font-size: 0.84rem;
    cursor: default;
    transition: background-color var(--t-fast), box-shadow var(--t-fast), color var(--t-fast);
  }
  .save.dirty {
    background: linear-gradient(180deg, #3b82f6, #2563eb);
    border-color: rgba(147, 197, 253, 0.25);
    color: white;
    cursor: pointer;
    box-shadow: 0 2px 14px rgba(37, 99, 235, 0.35);
  }
  .save.dirty:hover {
    filter: brightness(1.08);
  }
  .save-dot {
    width: 7px;
    height: 7px;
    border-radius: 999px;
    background: #fff;
    box-shadow: 0 0 8px rgba(255, 255, 255, 0.9);
  }
  .tagline {
    color: var(--text-2);
    font-size: 0.85rem;
    margin: 0.6rem 0;
  }
  .hint {
    color: var(--ask);
    background: var(--ask-soft);
    border-radius: var(--r-sm);
    padding: 0.35rem 0.6rem;
    font-size: 0.8rem;
    margin: 0 0 0.4rem;
  }
  .err {
    background: var(--deny-soft);
    border: 1px solid rgba(248, 113, 113, 0.3);
    color: var(--deny);
    padding: 0.4rem 0.8rem;
    font-size: 0.8rem;
    border-radius: var(--r-sm);
    margin-bottom: 0.4rem;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 0.6rem;
  }
  .explorer {
    width: 320px;
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
