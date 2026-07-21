<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import {
    addLocalToGitignore,
    applyProfile,
    buildDiff,
    claudeProjectTrustStatus,
    exportTemplate,
    gitignoreStatus,
    importTemplate,
    listBackups,
    listSensitivePaths,
    loadSettings,
    onSettingsFileChanged,
    policyReport,
    previewBackup,
    restoreBackup,
    saveReportFile,
    saveSettings,
    scanRecommendationRules,
    setSensitivePathDismissed,
    unwatchProject,
    watchProject,
    type BackupRecord,
    type ClaudeProjectTrustStatus,
    type DiffView,
    type GitignoreStatus,
    type Policy,
    type ScopeName,
    type SensitivePathRecord
  } from '$lib/ipc';
  import { app, mergeRules, refreshEffective, setPolicy } from '$lib/state.svelte';
  import FileExplorer from '$lib/components/FileExplorer.svelte';
  import PolicyEditor from '$lib/components/PolicyEditor.svelte';
  import EffectivePreview from '$lib/components/EffectivePreview.svelte';
  import RawJsonEditor from '$lib/components/RawJsonEditor.svelte';
  import DiffViewer from '$lib/components/DiffViewer.svelte';
  import UnsavedMarker from '$lib/components/UnsavedMarker.svelte';
  import { modalFocus } from '$lib/modal';
  import { confirmDiscardChanges, isUnsavedSource } from '$lib/unsaved';

  let rightMode = $state<'effective' | 'raw'>('effective');
  let workspaceMode = $state<'files' | 'policy' | 'preview'>('policy');
  let saving = $state(false);
  let error = $state<string | null>(null);
  let diff = $state<DiffView | null>(null);
  let saveScope = $state<ScopeName>('project');
  let profile = $state('');
  let gitignore = $state<GitignoreStatus | null>(null);
  let backups = $state<BackupRecord[] | null>(null);
  let sensitivePaths = $state<SensitivePathRecord[]>([]);
  let showSensitivePaths = $state(false);
  let backupPreview = $state<{ rec: BackupRecord; text: string } | null>(null);
  let report = $state<string | null>(null);
  let claudeTrust = $state<ClaudeProjectTrustStatus | null>(null);
  const hasSharedAllow = $derived(
    (claudeTrust?.sharedAllowRules ?? 0) > 0 ||
      app.scoped.project.rules.some((rule) => rule.policy === 'allow')
  );
  const showTrustWarning = $derived(
    claudeTrust !== null && !claudeTrust.accepted && hasSharedAllow
  );
  const activeSensitiveCount = $derived(sensitivePaths.filter((item) => !item.dismissed).length);

  async function refreshClaudeTrust() {
    try {
      claudeTrust = await claudeProjectTrustStatus(app.projectRoot);
    } catch {
      claudeTrust = null;
    }
  }

  // External-change detection: watch the settings files while the page is open.
  let externalChange = $state<string | null>(null);
  let lastSaveAt = 0;
  let unlisten: (() => void) | null = null;
  let changeTimer: ReturnType<typeof setTimeout> | undefined;

  function onExternalChange(path: string) {
    if (Date.now() - lastSaveAt < 2500) return; // our own write
    clearTimeout(changeTimer);
    // Editors fire several fs events per save — debounce into one reaction.
    changeTimer = setTimeout(async () => {
      if (app.dirty) {
        externalChange = path;
        return;
      }
      try {
        app.scoped = await loadSettings(app.projectRoot);
        await refreshClaudeTrust();
        await refreshEffective();
        status = '설정 파일이 외부에서 변경되어 다시 불러왔습니다.';
      } catch (e) {
        error = String(e);
      }
    }, 400);
  }

  async function reloadFromDisk() {
    try {
      app.scoped = await loadSettings(app.projectRoot);
      await refreshClaudeTrust();
      app.dirty = false;
      await refreshEffective();
      externalChange = null;
      status = '디스크의 설정을 다시 불러왔습니다.';
    } catch (e) {
      error = String(e);
    }
  }

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
    sensitivePaths = app.view?.sensitivePaths ?? [];
    profile = app.view?.project.riskProfile ?? '';
    await refreshClaudeTrust();
    try {
      await watchProject(app.projectRoot);
      unlisten = await onSettingsFileChanged(onExternalChange);
    } catch {
      /* watcher is best-effort */
    }
  });

  onDestroy(() => {
    clearTimeout(changeTimer);
    unlisten?.();
    unwatchProject().catch(() => {});
  });

  async function onProfileChange() {
    if (!profile) return;
    try {
      const plan = await applyProfile(app.projectRoot, profile);
      mergeRules(app.activeScope, plan.rules);
      if (app.view) app.view.project.riskProfile = profile;
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
    lastSaveAt = Date.now();
    try {
      await saveSettings({
        projectRoot: app.projectRoot,
        projectId: app.projectId,
        scope: saveScope,
        scopeRules: app.scoped[saveScope],
        projectName: app.projectName
      });
      if (saveScope === 'project' && app.view) app.view.hasProjectSettings = true;
      await refreshClaudeTrust();
      lastSaveAt = Date.now();
      app.dirty = false;
      diff = null;
      // Re-watch: the save may have created .claude/ dirs that weren't watchable before.
      watchProject(app.projectRoot).catch(() => {});
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
    backupPreview = { rec, text: await previewBackup(rec.id) };
  }

  async function doRestore(rec: BackupRecord) {
    try {
      lastSaveAt = Date.now();
      await restoreBackup(rec.id);
      backupPreview = null;
      backups = null;
      // Reload rules to reflect restored file.
      app.scoped = await loadSettings(app.projectRoot);
      await refreshClaudeTrust();
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

  async function openSensitivePaths() {
    try {
      sensitivePaths = await listSensitivePaths(app.projectId);
      showSensitivePaths = true;
    } catch (e) {
      error = String(e);
    }
  }

  async function toggleSensitivePath(item: SensitivePathRecord) {
    try {
      await setSensitivePathDismissed(app.projectId, item.id, !item.dismissed);
      item.dismissed = !item.dismissed;
      if (app.view) app.view.sensitivePaths = sensitivePaths;
    } catch (e) {
      error = String(e);
    }
  }

  function setRightMode(next: 'effective' | 'raw') {
    if (next === rightMode) return;
    if (
      rightMode === 'raw' &&
      !confirmDiscardChanges(isUnsavedSource('raw-settings-project'), window.confirm.bind(window))
    ) {
      return;
    }
    rightMode = next;
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
<UnsavedMarker id="project-rules" when={app.dirty} />

<div class="page">
  <div class="top">
    <button class="back" onclick={() => goto('/')} aria-label="홈으로">←</button>
    <div class="title">
      {app.projectName}
      {#if app.view?.risk}
        <span class="risk risk-{app.view.risk.level.toLowerCase()}">
          Risk {app.view.risk.level} · {app.view.risk.score}
        </span>
      {/if}
    </div>

    <div class="tools">
      <select class="profile" bind:value={profile} onchange={onProfileChange} title="프로필 적용">
        <option value="">Profile…</option>
        <option value="conservative">Conservative</option>
        <option value="balanced">Balanced</option>
        <option value="fast-dev">Fast Dev</option>
        <option value="custom">Custom</option>
      </select>
      <button class="mini" onclick={applyScanRecommendations}>추천 적용</button>
      <button class="mini" onclick={openSensitivePaths}>민감 경로 {activeSensitiveCount}</button>
      <span class="sep" aria-hidden="true"></span>
      <button class="mini" onclick={() => goto('/env')}>Env</button>
      <button class="mini" onclick={openBackups}>Backups</button>
      <button class="mini" onclick={makeReport}>Report</button>
      <button class="mini" onclick={doExport}>Export</button>
      <button class="mini" onclick={doImport}>Import</button>
      <button class="mini" onclick={() => goto('/guide?s=project')} title="사용 가이드" aria-label="사용 가이드">?</button>
    </div>

    <button class="save save-push" class:dirty={app.dirty} onclick={openSaveDialog} disabled={!app.dirty}>
      {#if app.dirty}<span class="save-dot" aria-hidden="true"></span>Save…{:else}Saved{/if}
    </button>
  </div>

  {#if gitignore && gitignore.exists && !gitignore.ignored}
    <div class="banner">
      <span class="banner-icon" aria-hidden="true">⚠</span>
      <span>
        <code>.claude/settings.local.json</code> 이 <code>.gitignore</code>에 없습니다 (개인 설정 유출
        위험).
      </span>
      <button onclick={fixGitignore}>.gitignore에 추가</button>
    </div>
  {/if}

  {#if showTrustWarning}
    <div class="banner trust-banner" role="alert">
      <span class="banner-icon" aria-hidden="true">⚠</span>
      <span>
        이 프로젝트는 아직 Claude Code에서 신뢰되지 않았습니다. 공유
        <code>.claude/settings.json</code>의 Allow 규칙은 무시되고 Deny만 적용됩니다.
        프로젝트 폴더에서 대화형 <code>claude</code>를 실행해 trust 확인을 승인하세요.
      </span>
    </div>
  {/if}

  {#if externalChange}
    <div class="banner">
      <span class="banner-icon" aria-hidden="true">⚠</span>
      <span>
        설정 파일이 <b>외부에서 변경</b>되었습니다: <code>{externalChange}</code> — 저장하지 않은
        편집과 충돌할 수 있습니다.
      </span>
      <span class="banner-actions">
        <button onclick={reloadFromDisk}>다시 불러오기 (내 변경 폐기)</button>
        <button onclick={() => (externalChange = null)}>무시</button>
      </span>
    </div>
  {/if}

  {#if error}<div class="err" role="alert">{error}</div>{/if}
  {#if status}<div class="status" role="status">{status}</div>{/if}

  <nav class="workspace-tabs" aria-label="편집 영역">
    <button
      class:active={workspaceMode === 'files'}
      aria-pressed={workspaceMode === 'files'}
      onclick={() => (workspaceMode = 'files')}>Files</button
    >
    <button
      class:active={workspaceMode === 'policy'}
      aria-pressed={workspaceMode === 'policy'}
      onclick={() => (workspaceMode = 'policy')}>Policy</button
    >
    <button
      class:active={workspaceMode === 'preview'}
      aria-pressed={workspaceMode === 'preview'}
      onclick={() => (workspaceMode = 'preview')}>Preview</button
    >
  </nav>

  <div class="cols">
    <section class="left" class:active={workspaceMode === 'files'} aria-label="파일 탐색기"><FileExplorer /></section>
    <section class="mid" class:active={workspaceMode === 'policy'} aria-label="정책 편집기"><PolicyEditor /></section>
    <section class="right" class:active={workspaceMode === 'preview'} aria-label="적용 결과 미리보기">
      <div class="rmode">
        <button class:active={rightMode === 'effective'} onclick={() => setRightMode('effective')}>
          Effective
        </button>
        <button class:active={rightMode === 'raw'} onclick={() => setRightMode('raw')}>Raw JSON</button>
      </div>
      {#if rightMode === 'effective'}<EffectivePreview />{:else}<RawJsonEditor />{/if}
    </section>
  </div>
</div>

{#if diff}
  <div class="modal-bg" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) diff = null; }}>
    <div
      class="modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="project-save-title"
      tabindex="-1"
      use:modalFocus={() => (diff = null)}
    >
      <h3 id="project-save-title">저장 전 변경 확인 — {saveScope} scope</h3>
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

{#if backups}
  <div class="modal-bg" role="presentation" onclick={(e) => {
    if (e.target === e.currentTarget) {
      backups = null;
      backupPreview = null;
    }
  }}>
    <div
      class="modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="project-backups-title"
      tabindex="-1"
      use:modalFocus={() => {
        backups = null;
        backupPreview = null;
      }}
    >
      <h3 id="project-backups-title">백업 복원</h3>
      {#if backups.length === 0}
        <p class="nochg">백업이 없습니다.</p>
      {:else}
        <ul class="blist">
          {#each backups as b (b.id)}
            <li>
              <span class="binfo">{b.createdAt} · {b.scope}</span>
              <span class="bactions">
                <button onclick={() => showBackup(b)}>미리보기</button>
                <button class="bprimary" onclick={() => doRestore(b)}>복원</button>
              </span>
            </li>
          {/each}
        </ul>
      {/if}
      {#if backupPreview}
        <pre class="preview">{backupPreview.text}</pre>
      {/if}
      <div class="modal-actions">
        <button data-modal-initial onclick={() => { backups = null; backupPreview = null; }}>닫기</button>
      </div>
    </div>
  </div>
{/if}

{#if showSensitivePaths}
  <div class="modal-bg" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) showSensitivePaths = false; }}>
    <div
      class="modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="sensitive-paths-title"
      tabindex="-1"
      use:modalFocus={() => (showSensitivePaths = false)}
    >
      <h3 id="sensitive-paths-title">스캐너 민감 경로</h3>
      <p class="nochg">무시한 경로는 다음 스캔과 프로필 추천에서 제외됩니다.</p>
      {#if sensitivePaths.length === 0}
        <p class="nochg">발견된 민감 경로가 없습니다.</p>
      {:else}
        <ul class="blist">
          {#each sensitivePaths as item (item.id)}
            <li>
              <span class="binfo"><code>{item.path}</code> · {item.source}</span>
              <button onclick={() => toggleSensitivePath(item)}>
                {item.dismissed ? '다시 추천' : '무시'}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
      <div class="modal-actions">
        <button data-modal-initial onclick={() => (showSensitivePaths = false)}>닫기</button>
      </div>
    </div>
  </div>
{/if}

{#if report}
  <div class="modal-bg" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) report = null; }}>
    <div
      class="modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="project-report-title"
      tabindex="-1"
      use:modalFocus={() => (report = null)}
    >
      <h3 id="project-report-title">Policy Report (Markdown)</h3>
      <pre class="preview">{report}</pre>
      <div class="modal-actions">
        <button data-modal-initial onclick={() => (report = null)}>닫기</button>
        <button onclick={copyReport}>클립보드에 복사</button>
        <button class="primary" onclick={saveReport}>파일로 저장</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .page {
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.5rem 0.8rem;
    border-bottom: 1px solid var(--border);
    background: linear-gradient(180deg, var(--bg-1), var(--bg-0));
    flex-wrap: wrap;
    flex-shrink: 0;
  }
  .back {
    background: none;
    border: 1px solid var(--border-strong);
    color: var(--text-2);
    border-radius: var(--r-sm);
    padding: 0.28rem 0.6rem;
    cursor: pointer;
    font-size: 0.9rem;
    line-height: 1.2;
    transition: color var(--t-fast), border-color var(--t-fast), background-color var(--t-fast);
  }
  .back:hover {
    color: var(--text-1);
    background: var(--bg-2);
  }
  .title {
    font-weight: 600;
    letter-spacing: -0.01em;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
  }
  .risk {
    font-size: 0.68rem;
    font-weight: 700;
    padding: 0.12rem 0.55rem;
    border-radius: 999px;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
  .risk-high {
    background: var(--deny-soft);
    color: var(--deny);
    border: 1px solid rgba(248, 113, 113, 0.35);
  }
  .risk-medium {
    background: var(--ask-soft);
    color: var(--ask);
    border: 1px solid rgba(251, 191, 36, 0.35);
  }
  .risk-low {
    background: var(--allow-soft);
    color: var(--allow);
    border: 1px solid rgba(52, 211, 153, 0.35);
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    flex-wrap: wrap;
  }
  .sep {
    width: 1px;
    height: 1.1rem;
    background: var(--border);
    margin: 0 0.2rem;
  }
  .profile {
    background: var(--bg-1);
    color: var(--text-1);
    border: 1px solid var(--border-strong);
    border-radius: var(--r-sm);
    padding: 0.3rem 0.4rem;
    font-size: 0.78rem;
  }
  .mini {
    background: transparent;
    border: 1px solid transparent;
    color: var(--text-2);
    border-radius: var(--r-sm);
    padding: 0.3rem 0.6rem;
    cursor: pointer;
    font-size: 0.78rem;
    transition: color var(--t-fast), background-color var(--t-fast), border-color var(--t-fast);
  }
  .mini:hover {
    color: var(--text-1);
    background: var(--bg-2);
    border-color: var(--border);
  }
  .save-push {
    margin-left: auto;
  }
  .save {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    background: var(--bg-2);
    border: 1px solid var(--border);
    color: var(--text-3);
    border-radius: var(--r-sm);
    padding: 0.4rem 1rem;
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
  .banner {
    background: var(--ask-soft);
    border-bottom: 1px solid rgba(251, 191, 36, 0.25);
    color: var(--ask);
    padding: 0.45rem 0.8rem;
    font-size: 0.8rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-shrink: 0;
  }
  .banner-icon {
    color: var(--ask);
  }
  .banner code {
    background: rgba(0, 0, 0, 0.25);
    padding: 0 0.3rem;
    border-radius: 3px;
  }
  .banner button {
    margin-left: auto;
    background: rgba(251, 191, 36, 0.15);
    border: 1px solid rgba(251, 191, 36, 0.35);
    color: var(--ask);
    border-radius: var(--r-sm);
    padding: 0.25rem 0.65rem;
    cursor: pointer;
    font-size: 0.76rem;
    transition: background-color var(--t-fast);
  }
  .banner button:hover {
    background: rgba(251, 191, 36, 0.25);
  }
  .banner-actions {
    margin-left: auto;
    display: flex;
    gap: 0.35rem;
    flex-shrink: 0;
  }
  .banner-actions button {
    margin-left: 0;
    white-space: nowrap;
  }
  .err {
    background: var(--deny-soft);
    border-bottom: 1px solid rgba(248, 113, 113, 0.25);
    color: var(--deny);
    padding: 0.4rem 0.8rem;
    font-size: 0.8rem;
    flex-shrink: 0;
  }
  .status {
    background: var(--allow-soft);
    border-bottom: 1px solid rgba(52, 211, 153, 0.2);
    color: var(--allow);
    padding: 0.35rem 0.8rem;
    font-size: 0.78rem;
    flex-shrink: 0;
  }
  .cols {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    flex: 1;
    min-height: 0;
  }
  .workspace-tabs {
    display: none;
  }
  .left {
    border-right: 1px solid var(--border);
  }
  .mid {
    border-right: 1px solid var(--border);
    overflow: auto;
  }
  .cols section {
    min-height: 0;
    min-width: 0;
  }
  .rmode {
    display: flex;
    gap: 0.25rem;
    padding: 0.4rem 0.5rem;
    border-bottom: 1px solid var(--border);
  }
  .rmode button {
    background: transparent;
    border: 1px solid transparent;
    color: var(--text-2);
    border-radius: var(--r-sm);
    padding: 0.22rem 0.65rem;
    cursor: pointer;
    font-size: 0.75rem;
    transition: color var(--t-fast), background-color var(--t-fast), border-color var(--t-fast);
  }
  .rmode button:hover {
    color: var(--text-1);
    background: var(--bg-2);
  }
  .rmode button.active {
    border-color: rgba(79, 142, 247, 0.35);
    background: var(--accent-soft);
    color: var(--accent-text);
  }
  .right {
    display: flex;
    flex-direction: column;
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
  .blist {
    list-style: none;
    padding: 0;
    display: grid;
    gap: 0.35rem;
  }
  .blist li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.8rem;
    border: 1px solid var(--border);
    background: var(--bg-2);
    border-radius: var(--r-sm);
    padding: 0.35rem 0.6rem;
  }
  .binfo {
    font-variant-numeric: tabular-nums;
    color: var(--text-2);
  }
  .bactions {
    display: flex;
    gap: 0.35rem;
  }
  .bactions button {
    background: var(--bg-3);
    border: 1px solid var(--border-strong);
    color: var(--text-1);
    border-radius: 5px;
    padding: 0.2rem 0.55rem;
    cursor: pointer;
    font-size: 0.72rem;
    transition: background-color var(--t-fast), border-color var(--t-fast);
  }
  .bactions button:hover {
    border-color: var(--accent);
  }
  .bactions .bprimary {
    background: var(--accent-strong);
    border-color: var(--accent-strong);
    color: white;
  }
  .preview {
    background: var(--bg-0);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    padding: 0.6rem;
    font-size: 0.72rem;
    max-height: 40vh;
    overflow: auto;
    white-space: pre-wrap;
    margin-top: 0.6rem;
  }

  @media (max-width: 1100px) {
    .page {
      height: 100dvh;
      overflow: hidden;
    }
    .top .tools {
      order: 3;
      flex: 1 0 100%;
      flex-wrap: nowrap;
      max-width: 100%;
      overflow-x: auto;
      padding-bottom: 0.15rem;
    }
    .top .tools > * {
      flex-shrink: 0;
    }
    .workspace-tabs {
      display: flex;
      flex-shrink: 0;
      gap: 0.25rem;
      padding: 0.35rem 0.5rem;
      border-bottom: 1px solid var(--border);
      background: var(--bg-1);
      overflow-x: auto;
    }
    .workspace-tabs button {
      flex: 1 0 auto;
      min-width: 5.5rem;
      border: 1px solid transparent;
      border-radius: var(--r-sm);
      background: transparent;
      color: var(--text-2);
      padding: 0.32rem 0.7rem;
      cursor: pointer;
    }
    .workspace-tabs button.active {
      border-color: rgba(79, 142, 247, 0.35);
      background: var(--accent-soft);
      color: var(--accent-text);
    }
    .cols {
      display: block;
      overflow: hidden;
    }
    .cols section {
      display: none;
      width: 100%;
      height: 100%;
      border-right: 0;
    }
    .cols section.active {
      display: block;
    }
    .cols .right.active {
      display: flex;
    }
    .rmode {
      flex-shrink: 0;
      overflow-x: auto;
    }
    .rmode button {
      flex-shrink: 0;
      white-space: nowrap;
    }
  }

  @media (max-width: 700px) {
    .top {
      gap: 0.4rem;
      padding: 0.45rem 0.55rem;
    }
    .title {
      flex: 1;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    .risk {
      display: none;
    }
    .save {
      padding-inline: 0.7rem;
    }
    .banner {
      align-items: flex-start;
      flex-wrap: wrap;
    }
    .banner-actions {
      flex-basis: 100%;
    }
  }
</style>
