<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import {
    addLocalToGitignore,
    applyProfile,
    buildDiff,
    exportTemplate,
    gitignoreStatus,
    importTemplate,
    listBackups,
    loadSettings,
    policyReport,
    previewBackup,
    restoreBackup,
    saveReportFile,
    saveSettings,
    scanRecommendationRules,
    type BackupRecord,
    type DiffView,
    type GitignoreStatus,
    type Policy,
    type ScopeName
  } from '$lib/ipc';
  import { app, mergeRules, refreshEffective, setDefaultMode, setPolicy } from '$lib/state.svelte';
  import FileExplorer from '$lib/components/FileExplorer.svelte';
  import PolicyEditor from '$lib/components/PolicyEditor.svelte';
  import EffectivePreview from '$lib/components/EffectivePreview.svelte';
  import RawJsonEditor from '$lib/components/RawJsonEditor.svelte';
  import DiffViewer from '$lib/components/DiffViewer.svelte';

  let rightMode = $state<'effective' | 'raw'>('effective');
  let saving = $state(false);
  let error = $state<string | null>(null);
  let diff = $state<DiffView | null>(null);
  let saveScope = $state<ScopeName>('project');
  let profile = $state('');
  let gitignore = $state<GitignoreStatus | null>(null);
  let backups = $state<BackupRecord[] | null>(null);
  let backupPreview = $state<{ rec: BackupRecord; text: string } | null>(null);
  let report = $state<string | null>(null);

  onMount(async () => {
    if (!app.loaded) {
      goto('/');
      return;
    }
    try {
      gitignore = await gitignoreStatus(app.projectRoot);
    } catch {
      /* non-fatal */
    }
  });

  const dontAsk = $derived(app.scoped[app.activeScope].defaultMode === 'dontAsk');

  function toggleDefaultDeny() {
    setDefaultMode(app.activeScope, dontAsk ? null : 'dontAsk');
    refreshEffective();
  }

  async function onProfileChange() {
    if (!profile) return;
    try {
      const plan = await applyProfile(app.projectRoot, profile);
      mergeRules(app.activeScope, plan.rules, plan.defaultMode);
      await refreshEffective();
    } catch (e) {
      error = String(e);
    }
  }

  async function applyScanRecommendations() {
    try {
      const rules = await scanRecommendationRules(app.projectRoot);
      mergeRules(app.activeScope, rules);
      await refreshEffective();
    } catch (e) {
      error = String(e);
    }
  }

  async function fixGitignore() {
    try {
      await addLocalToGitignore(app.projectRoot);
      gitignore = await gitignoreStatus(app.projectRoot);
    } catch (e) {
      error = String(e);
    }
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

  async function openBackups() {
    try {
      backups = await listBackups(app.projectId);
    } catch (e) {
      error = String(e);
    }
  }

  async function showBackup(rec: BackupRecord) {
    backupPreview = { rec, text: await previewBackup(rec.backupPath) };
  }

  async function doRestore(rec: BackupRecord) {
    try {
      await restoreBackup(rec.backupPath, rec.originalPath);
      backupPreview = null;
      backups = null;
      // Reload rules to reflect restored file.
      app.scoped = await loadSettings(app.projectRoot);
      await refreshEffective();
    } catch (e) {
      error = String(e);
    }
  }

  async function makeReport() {
    try {
      report = await policyReport({
        projectName: app.projectName,
        profile: profile || app.view?.project.riskProfile || null,
        scoped: app.scoped,
        riskScore: app.view?.risk.score ?? 0,
        riskLevel: app.view?.risk.level ?? 'Low'
      });
    } catch (e) {
      error = String(e);
    }
  }

  async function copyReport() {
    if (report) await navigator.clipboard.writeText(report);
  }

  let status = $state<string | null>(null);

  async function doExport() {
    try {
      const p = await exportTemplate(app.scoped, app.projectName);
      if (p) status = `템플릿 저장됨 → ${p}`;
    } catch (e) {
      error = String(e);
    }
  }

  async function doImport() {
    try {
      const s = await importTemplate();
      if (s) {
        app.scoped = s;
        app.dirty = true;
        await refreshEffective();
        status = '템플릿을 불러왔습니다. 저장하면 적용됩니다.';
      }
    } catch (e) {
      error = String(e);
    }
  }

  async function saveReport() {
    if (!report) return;
    try {
      const p = await saveReportFile(report, app.projectName);
      if (p) status = `리포트 저장됨 → ${p}`;
    } catch (e) {
      error = String(e);
    }
  }

  // Keyboard shortcuts: Ctrl/Cmd+S save; a/d/k set policy on the selected path.
  async function applyQuick(policy: Policy) {
    if (!app.selectedPath) return;
    setPolicy(app.selectedPath, policy, 'folder-and-children');
    await refreshEffective();
  }

  function onKey(e: KeyboardEvent) {
    const t = e.target as HTMLElement | null;
    if (t && ['INPUT', 'TEXTAREA', 'SELECT'].includes(t.tagName)) return;
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
      e.preventDefault();
      if (app.dirty) openSaveDialog();
      return;
    }
    if (!app.selectedPath) return;
    if (e.key === 'a') applyQuick('allow');
    else if (e.key === 'd') applyQuick('deny');
    else if (e.key === 'k') applyQuick('ask');
  }
</script>

<svelte:window onkeydown={onKey} />

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

  <select class="profile" bind:value={profile} onchange={onProfileChange} title="프로필 적용">
    <option value="">Profile…</option>
    <option value="conservative">Conservative</option>
    <option value="balanced">Balanced</option>
    <option value="fast-dev">Fast Dev</option>
    <option value="custom">Custom</option>
  </select>
  <button class="mini" onclick={applyScanRecommendations}>추천 적용</button>
  <button class="mini" onclick={() => goto('/env')}>Env</button>
  <button class="mini" onclick={openBackups}>Backups</button>
  <button class="mini" onclick={makeReport}>Report</button>
  <button class="mini" onclick={doExport}>Export</button>
  <button class="mini" onclick={doImport}>Import</button>
  <button class="mini" onclick={() => goto('/guide')} title="사용 가이드">?</button>

  <label class="dd">
    <input type="checkbox" checked={dontAsk} onchange={toggleDefaultDeny} />
    Default Deny ({app.activeScope})
  </label>
  <button class="save" onclick={openSaveDialog} disabled={!app.dirty}>
    {app.dirty ? '● Save…' : 'Saved'}
  </button>
</div>

{#if gitignore && gitignore.exists && !gitignore.ignored}
  <div class="banner">
    ⚠️ <code>.claude/settings.local.json</code> 이 <code>.gitignore</code>에 없습니다 (개인 설정 유출 위험).
    <button onclick={fixGitignore}>.gitignore에 추가</button>
  </div>
{/if}

{#if error}<div class="err">{error}</div>{/if}
{#if status}<div class="status" role="status">{status}</div>{/if}

<div class="cols">
  <section class="left"><FileExplorer /></section>
  <section class="mid"><PolicyEditor /></section>
  <section class="right">
    <div class="rmode">
      <button class:active={rightMode === 'effective'} onclick={() => (rightMode = 'effective')}>Effective</button>
      <button class:active={rightMode === 'raw'} onclick={() => (rightMode = 'raw')}>Raw JSON</button>
    </div>
    {#if rightMode === 'effective'}<EffectivePreview />{:else}<RawJsonEditor />{/if}
  </section>
</div>

{#if diff}
  <div class="modal-bg" role="presentation" onclick={() => (diff = null)}>
    <div class="modal" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
      <h3>저장 전 변경 확인 — {saveScope} scope</h3>
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

{#if backups}
  <div class="modal-bg" role="presentation" onclick={() => (backups = null)}>
    <div class="modal" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
      <h3>백업 복원</h3>
      {#if backups.length === 0}
        <p class="nochg">백업이 없습니다.</p>
      {:else}
        <ul class="blist">
          {#each backups as b (b.id)}
            <li>
              <span>{b.createdAt} · {b.scope}</span>
              <span class="bactions">
                <button onclick={() => showBackup(b)}>미리보기</button>
                <button class="primary" onclick={() => doRestore(b)}>복원</button>
              </span>
            </li>
          {/each}
        </ul>
      {/if}
      {#if backupPreview}
        <pre class="preview">{backupPreview.text}</pre>
      {/if}
      <div class="modal-actions">
        <button onclick={() => { backups = null; backupPreview = null; }}>닫기</button>
      </div>
    </div>
  </div>
{/if}

{#if report}
  <div class="modal-bg" role="presentation" onclick={() => (report = null)}>
    <div class="modal" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
      <h3>Policy Report (Markdown)</h3>
      <pre class="preview">{report}</pre>
      <div class="modal-actions">
        <button onclick={() => (report = null)}>닫기</button>
        <button onclick={copyReport}>클립보드에 복사</button>
        <button class="primary" onclick={saveReport}>파일로 저장</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .top { display: flex; align-items: center; gap: 0.5rem; padding: 0.5rem 0.8rem; border-bottom: 1px solid #1e293b; background: #0b1220; flex-wrap: wrap; }
  .back { background: none; border: 1px solid #334155; color: #94a3b8; border-radius: 6px; padding: 0.3rem 0.6rem; cursor: pointer; }
  .title { font-weight: 600; display: flex; align-items: center; gap: 0.5rem; }
  .risk { font-size: 0.7rem; padding: 0.1rem 0.5rem; border-radius: 999px; }
  .risk-high { background: #7f1d1d; color: #fecaca; }
  .risk-medium { background: #78350f; color: #fde68a; }
  .risk-low { background: #14532d; color: #bbf7d0; }
  .profile { background: #0b1220; color: #e2e8f0; border: 1px solid #334155; border-radius: 6px; padding: 0.3rem; font-size: 0.78rem; }
  .mini { background: #1e293b; border: 1px solid #334155; color: #cbd5e1; border-radius: 6px; padding: 0.3rem 0.6rem; cursor: pointer; font-size: 0.78rem; }
  .dd { margin-left: auto; font-size: 0.78rem; color: #94a3b8; display: flex; align-items: center; gap: 0.3rem; }
  .save { background: #2563eb; border: none; color: white; border-radius: 6px; padding: 0.4rem 0.9rem; cursor: pointer; }
  .save:disabled { background: #1e293b; color: #64748b; cursor: default; }
  .banner { background: #422006; color: #fde68a; padding: 0.4rem 0.8rem; font-size: 0.8rem; display: flex; align-items: center; gap: 0.5rem; }
  .banner button { margin-left: auto; background: #78350f; border: none; color: #fde68a; border-radius: 6px; padding: 0.25rem 0.6rem; cursor: pointer; }
  .banner code { background: #00000033; padding: 0 0.25rem; border-radius: 3px; }
  .err { background: #7f1d1d; color: #fecaca; padding: 0.4rem 0.8rem; font-size: 0.8rem; }
  .status { background: #0e2a1a; color: #bbf7d0; padding: 0.35rem 0.8rem; font-size: 0.78rem; }
  .cols { display: grid; grid-template-columns: 1fr 1fr 1fr; height: calc(100vh - 46px); }
  .left { border-right: 1px solid #1e293b; }
  .mid { border-right: 1px solid #1e293b; overflow: auto; }
  .cols section { min-height: 0; }
  .rmode { display: flex; gap: 0.25rem; padding: 0.35rem 0.5rem; border-bottom: 1px solid #1e293b; }
  .rmode button { background: #0b1220; border: 1px solid #334155; color: #94a3b8; border-radius: 6px; padding: 0.2rem 0.6rem; cursor: pointer; font-size: 0.75rem; }
  .rmode button.active { border-color: #2563eb; color: #93c5fd; }
  .right { display: flex; flex-direction: column; }
  .modal-bg { position: fixed; inset: 0; background: #000a; display: flex; align-items: center; justify-content: center; padding: 2rem; }
  .modal { background: #0f172a; border: 1px solid #334155; border-radius: 10px; padding: 1rem; width: min(900px, 95vw); max-height: 90vh; overflow: auto; }
  .modal h3 { margin: 0 0 0.25rem; font-size: 1rem; }
  .fp { color: #64748b; font-size: 0.75rem; margin: 0 0 0.6rem; }
  .nochg { color: #94a3b8; }
  .modal-actions { display: flex; justify-content: flex-end; gap: 0.5rem; margin-top: 0.8rem; }
  .modal-actions button { padding: 0.45rem 0.9rem; border-radius: 6px; border: 1px solid #334155; background: #1e293b; color: #e2e8f0; cursor: pointer; }
  .modal-actions .primary { background: #2563eb; border-color: #2563eb; }
  .modal-actions .primary:disabled { opacity: 0.5; cursor: default; }
  .blist { list-style: none; padding: 0; display: grid; gap: 0.3rem; }
  .blist li { display: flex; justify-content: space-between; align-items: center; font-size: 0.8rem; border: 1px solid #1e293b; border-radius: 6px; padding: 0.3rem 0.5rem; }
  .bactions { display: flex; gap: 0.35rem; }
  .bactions button { background: #1e293b; border: 1px solid #334155; color: #cbd5e1; border-radius: 5px; padding: 0.2rem 0.5rem; cursor: pointer; font-size: 0.72rem; }
  .bactions .primary { background: #2563eb; border-color: #2563eb; color: white; }
  .preview { background: #0b1220; border: 1px solid #1e293b; border-radius: 6px; padding: 0.5rem; font-size: 0.72rem; max-height: 40vh; overflow: auto; white-space: pre-wrap; margin-top: 0.6rem; }
</style>
