<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import { goto } from '$app/navigation';
  import {
    addLocalToGitignore,
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
    type PolicyRule,
    type ScopeRules,
    type ScopeName,
    type SensitivePathRecord
  } from '$lib/ipc';
  import {
    app,
    mergeRules,
    refreshEffective,
    setPolicy,
    markAllScopesSaved,
    markScopeSaved,
    isScopeDirty,
    upsertRule,
    removeRule
  } from '$lib/state.svelte';
  import CandidateReview from '$lib/components/CandidateReview.svelte';
  import FileExplorer from '$lib/components/FileExplorer.svelte';
  import PolicyEditor from '$lib/components/PolicyEditor.svelte';
  import EffectivePreview from '$lib/components/EffectivePreview.svelte';
  import RawJsonEditor from '$lib/components/RawJsonEditor.svelte';
  import DiffViewer from '$lib/components/DiffViewer.svelte';
  import UnsavedMarker from '$lib/components/UnsavedMarker.svelte';
  import HelpButton from '$lib/components/HelpButton.svelte';
  import { modalFocus } from '$lib/modal';
  import { tooltip } from '$lib/tooltip';
  import { toast } from '$lib/toast.svelte';
  import { confirmDiscardChanges, isUnsavedSource } from '$lib/unsaved';

  let rightMode = $state<'effective' | 'raw'>('effective');
  let workspaceMode = $state<'files' | 'policy' | 'preview'>('policy');
  let saving = $state(false);
  let busy = $state<'scan' | 'backups' | 'report' | null>(null);
  let moreOpen = $state(false);
  let moreButton = $state<HTMLButtonElement | null>(null);
  let moreMenu = $state<HTMLDivElement | null>(null);
  let diff = $state<DiffView | null>(null);
  let saveScope = $state<ScopeName>('project');
  let candidates = $state<PolicyRule[] | null>(null);
  let candidateScope = $state<ScopeName>('project');
  let saveSnapshot = $state<ScopeRules | null>(null);
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
  const activeDirty = $derived(isScopeDirty(app.activeScope));
  const scanSummary = $derived(
    app.view ? `루트 이름 기반 후보 ${app.view.scan.candidates?.length ?? 0}개 · 하위 폴더 미검사` : ''
  );

  function fail(e: unknown) {
    toast(String(e), 'error');
  }

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
        markAllScopesSaved();
        await refreshClaudeTrust();
        await refreshEffective();
        toast('설정 파일이 외부에서 변경되어 다시 불러왔습니다.', 'info');
      } catch (e) {
        fail(e);
      }
    }, 400);
  }

  async function reloadFromDisk() {
    try {
      app.scoped = await loadSettings(app.projectRoot);
      await refreshClaudeTrust();
      markAllScopesSaved();
      await refreshEffective();
      externalChange = null;
      toast('디스크의 설정을 다시 불러왔습니다.');
    } catch (e) {
      fail(e);
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

  async function applyScanRecommendations() {
    busy = 'scan';
    try {
      candidateScope = app.activeScope;
      candidates = await scanRecommendationRules(app.projectRoot);
    } catch (e) {
      fail(e);
    } finally {
      busy = null;
    }
  }

  async function acceptCandidates(rules: PolicyRule[], replacements: string[]) {
    try {
      mergeRules(candidateScope, rules, replacements);
      candidates = null;
      await refreshEffective();
      toast(`${rules.length}개 규칙을 편집에 반영했습니다. 저장하면 적용됩니다.`);
    } catch (e) {
      fail(e);
    }
  }

  async function fixGitignore() {
    try {
      await addLocalToGitignore(app.projectRoot);
      gitignore = await gitignoreStatus(app.projectRoot);
      toast('.gitignore에 settings.local.json을 추가했습니다.');
    } catch (e) {
      fail(e);
    }
  }

  async function openSaveDialog() {
    saveScope = app.activeScope;
    try {
      saveSnapshot = $state.snapshot(app.scoped[saveScope]);
      diff = await buildDiff(app.projectRoot, saveScope, saveSnapshot);
    } catch (e) {
      fail(e);
    }
  }

  async function confirmSave() {
    if (!diff || !saveSnapshot) return;
    saving = true;
    lastSaveAt = Date.now();
    try {
      await saveSettings({
        projectRoot: app.projectRoot,
        projectId: app.projectId,
        scope: saveScope,
        scopeRules: saveSnapshot,
        expectedBefore: diff.before,
        projectName: app.projectName
      });
      if (saveScope === 'project' && app.view) app.view.hasProjectSettings = true;
      await refreshClaudeTrust();
      lastSaveAt = Date.now();
      markScopeSaved(saveScope, saveSnapshot);
      diff = null;
      toast(`${saveScope} 설정을 저장했습니다.`);
      // Re-watch: the save may have created .claude/ dirs that weren't watchable before.
      watchProject(app.projectRoot).catch(() => {});
    } catch (e) {
      fail(e);
    } finally {
      saving = false;
    }
  }

  async function openBackups() {
    busy = 'backups';
    try {
      backups = await listBackups(app.projectId);
    } catch (e) {
      fail(e);
    } finally {
      busy = null;
    }
  }

  async function showBackup(rec: BackupRecord) {
    try {
      backupPreview = { rec, text: await previewBackup(rec.id) };
    } catch (e) {
      fail(e);
    }
  }

  async function doRestore(rec: BackupRecord) {
    const message = `${rec.createdAt} 백업으로 ${rec.scope} 설정 파일을 덮어씁니다.` +
      (app.dirty ? '\n저장하지 않은 변경도 모두 사라집니다.' : '') + '\n계속할까요?';
    if (!window.confirm(message)) return;
    try {
      lastSaveAt = Date.now();
      await restoreBackup(rec.id);
      backupPreview = null;
      backups = null;
      // Reload rules to reflect restored file.
      app.scoped = await loadSettings(app.projectRoot);
      markAllScopesSaved();
      await refreshClaudeTrust();
      await refreshEffective();
      toast('백업을 복원했습니다.');
    } catch (e) {
      fail(e);
    }
  }

  async function makeReport() {
    busy = 'report';
    try {
      report = await policyReport({
        projectRoot: app.projectRoot,
        projectName: app.projectName,
        profile: null,
        scoped: app.scoped,
        riskScore: app.view?.risk.score ?? 0,
        riskLevel: app.view?.risk.level ?? 'Low'
      });
    } catch (e) {
      fail(e);
    } finally {
      busy = null;
    }
  }

  async function copyReport() {
    if (!report) return;
    try {
      await navigator.clipboard.writeText(report);
      toast('리포트를 클립보드에 복사했습니다.');
    } catch (e) {
      fail(e);
    }
  }

  async function doExport() {
    try {
      const p = await exportTemplate(app.scoped, app.projectName);
      if (p) toast(`템플릿을 저장했습니다 → ${p}`);
    } catch (e) {
      fail(e);
    }
  }

  async function doImport() {
    if (
      !window.confirm(
        '템플릿을 불러오면 모든 scope의 현재 편집 내용을 덮어씁니다.' +
          (app.dirty ? '\n저장하지 않은 변경도 사라집니다.' : '') +
          '\n계속할까요?'
      )
    ) {
      return;
    }
    try {
      const s = await importTemplate();
      if (s) {
        app.scoped = s;
        await refreshEffective();
        toast('템플릿을 불러왔습니다. 저장하면 적용됩니다.');
      }
    } catch (e) {
      fail(e);
    }
  }

  async function saveReport() {
    if (!report) return;
    try {
      const p = await saveReportFile(report, app.projectName);
      if (p) toast(`리포트를 저장했습니다 → ${p}`);
    } catch (e) {
      fail(e);
    }
  }

  async function toggleMore() {
    moreOpen = !moreOpen;
    if (moreOpen) {
      await tick();
      moreMenu?.querySelector<HTMLElement>('[role="menuitem"]')?.focus();
    }
  }

  function closeMore(restoreFocus = false) {
    if (!moreOpen) return;
    moreOpen = false;
    if (restoreFocus) moreButton?.focus();
  }

  function runMore(action: () => void) {
    closeMore(true);
    action();
  }

  function onMoreKey(e: KeyboardEvent) {
    if (!moreMenu) return;
    const items = Array.from(moreMenu.querySelectorAll<HTMLElement>('[role="menuitem"]:not(:disabled)'));
    const i = items.indexOf(document.activeElement as HTMLElement);
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      closeMore(true);
    } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      const next = e.key === 'ArrowDown' ? (i + 1) % items.length : (i - 1 + items.length) % items.length;
      items[next]?.focus();
    } else if (e.key === 'Home' || e.key === 'End') {
      e.preventDefault();
      items[e.key === 'Home' ? 0 : items.length - 1]?.focus();
    } else if (e.key === 'Tab') {
      closeMore();
    }
  }

  function onWindowClick(e: MouseEvent) {
    const t = e.target as Node | null;
    if (moreOpen && t && !moreMenu?.contains(t) && !moreButton?.contains(t)) closeMore();
  }

  // Keyboard shortcuts: Ctrl/Cmd+S save; a/d/k set policy on the selected path.
  const POLICY_LABEL: Record<Policy, string> = { allow: 'Allow', ask: 'Ask', deny: 'Deny' };

  async function applyQuick(policy: Policy) {
    if (!app.selectedPath) return;
    const path = app.selectedPath;
    const entry = app.view?.tree.find((item) => item.path === path);
    // A nested selection has no reliable kind here; leave it to the policy editor.
    if (!entry) return;
    const scope = app.activeScope;
    const previous = app.scoped[scope].rules.find((r) => r.path === path);
    const before = previous ? $state.snapshot(previous) : null;
    setPolicy(path, policy, entry.isDir ? 'folder-and-children' : 'file');
    await refreshEffective();
    toast(`${entry.name || path} → ${POLICY_LABEL[policy]} (${scope})`, 'info', {
      label: '되돌리기',
      run: () => {
        if (before) upsertRule(scope, before);
        else removeRule(scope, path);
        refreshEffective().catch(fail);
      }
    });
  }

  async function openSensitivePaths() {
    try {
      sensitivePaths = await listSensitivePaths(app.projectId);
      showSensitivePaths = true;
    } catch (e) {
      fail(e);
    }
  }

  async function toggleSensitivePath(item: SensitivePathRecord) {
    try {
      await setSensitivePathDismissed(app.projectId, item.id, !item.dismissed);
      item.dismissed = !item.dismissed;
      if (app.view) app.view.sensitivePaths = sensitivePaths;
    } catch (e) {
      fail(e);
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
    if (e.key === 'Escape' && moreOpen) {
      closeMore(true);
      return;
    }
    if (candidates || diff || backups || showSensitivePaths || report) return;
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
      e.preventDefault();
      if (activeDirty) openSaveDialog();
      return;
    }
    if (e.ctrlKey || e.metaKey || e.altKey || moreOpen) return;
    const t = e.target as HTMLElement | null;
    if (t && (['INPUT', 'TEXTAREA', 'SELECT'].includes(t.tagName) || t.isContentEditable)) return;
    if (!app.selectedPath) return;
    if (e.key === 'a') applyQuick('allow');
    else if (e.key === 'd') applyQuick('deny');
    else if (e.key === 'k') applyQuick('ask');
  }
</script>

<svelte:window onkeydown={onKey} onclick={onWindowClick} />
<UnsavedMarker id="project-rules" when={app.dirty} />

<div class="page">
  <div class="top">
    <div class="title">
      <span class="pname truncate">{app.projectName}</span>
      {#if app.view}<span class="scan-summary truncate" use:tooltip={scanSummary}>{scanSummary}</span>{/if}
    </div>

    <div class="tools">
      <button class="btn btn-ghost btn-sm" onclick={applyScanRecommendations} disabled={busy === 'scan'}>
        {busy === 'scan' ? '후보 검사 중…' : '경로 후보 검토'}
      </button>
      <button class="btn btn-ghost btn-sm" onclick={openSensitivePaths}>
        검토 대상 <span class="chip" class:chip-ask={activeSensitiveCount > 0}>{activeSensitiveCount}</span>
      </button>
      <div class="more">
        <button
          class="btn btn-ghost btn-sm"
          bind:this={moreButton}
          aria-haspopup="menu"
          aria-expanded={moreOpen}
          aria-controls="project-more-menu"
          onclick={toggleMore}
        >
          {busy === 'report' ? '리포트 생성 중…' : busy === 'backups' ? '백업 불러오는 중…' : '더보기'}
          <span aria-hidden="true">▾</span>
        </button>
        {#if moreOpen}
          <div
            class="menu"
            id="project-more-menu"
            role="menu"
            tabindex="-1"
            aria-label="더보기"
            bind:this={moreMenu}
            onkeydown={onMoreKey}
          >
            <button role="menuitem" onclick={() => runMore(() => goto('/env'))}>환경 변수</button>
            <button role="menuitem" disabled={busy === 'backups'} onclick={() => runMore(openBackups)}>
              {busy === 'backups' ? '백업 불러오는 중…' : '백업 복원…'}
            </button>
            <button role="menuitem" disabled={busy === 'report'} onclick={() => runMore(makeReport)}>
              {busy === 'report' ? '리포트 생성 중…' : '정책 리포트…'}
            </button>
            <div class="menu-sep" role="separator"></div>
            <button role="menuitem" onclick={() => runMore(doExport)}>템플릿 내보내기…</button>
            <button role="menuitem" onclick={() => runMore(doImport)}>템플릿 가져오기…</button>
          </div>
        {/if}
      </div>
      <HelpButton section="project" compact />
    </div>

    <button
      class="btn save"
      class:btn-primary={activeDirty}
      onclick={openSaveDialog}
      disabled={!activeDirty}
      title="Ctrl+S"
    >
      {#if activeDirty}<span class="save-dot" aria-hidden="true"></span>{app.activeScope} 저장…{:else}{app.activeScope} 저장됨{/if}
    </button>
  </div>

  {#if app.dirty}<p class="dirty-line">저장하지 않은 scope: {(['project', 'local', 'user'] as const).filter(isScopeDirty).join(', ')}</p>{/if}

  {#if (gitignore && gitignore.exists && !gitignore.ignored) || showTrustWarning || externalChange}
    <div class="notices">
      {#if gitignore && gitignore.exists && !gitignore.ignored}
        <div class="callout callout-warn notice">
          <span>
            <code>.claude/settings.local.json</code> 이 <code>.gitignore</code>에 없습니다 (개인 설정 유출
            위험).
          </span>
          <span class="notice-actions"><button class="btn btn-sm" onclick={fixGitignore}>.gitignore에 추가</button></span>
        </div>
      {/if}

      {#if showTrustWarning}
        <div class="callout callout-warn notice" role="alert">
          <span>
            이 프로젝트는 아직 Claude Code에서 신뢰되지 않았습니다. 공유
            <code>.claude/settings.json</code>의 Allow 규칙은 무시되고 Deny만 적용됩니다.
            프로젝트 폴더에서 대화형 <code>claude</code>를 실행해 trust 확인을 승인하세요.
          </span>
        </div>
      {/if}

      {#if externalChange}
        <div class="callout callout-warn notice" role="alert">
          <span>
            설정 파일이 <b>외부에서 변경</b>되었습니다: <code>{externalChange}</code> — 저장하지 않은
            편집과 충돌할 수 있습니다.
          </span>
          <span class="notice-actions">
            <button class="btn btn-sm" onclick={reloadFromDisk}>다시 불러오기 (내 변경 폐기)</button>
            <button class="btn btn-ghost btn-sm" onclick={() => (externalChange = null)}>무시</button>
          </span>
        </div>
      {/if}
    </div>
  {/if}

  <div class="workspace-tabs">
    <div class="segmented" role="tablist" aria-label="편집 영역">
      <button
        role="tab"
        id="tab-files"
        aria-controls="panel-files"
        aria-selected={workspaceMode === 'files'}
        onclick={() => (workspaceMode = 'files')}>파일</button
      >
      <button
        role="tab"
        id="tab-policy"
        aria-controls="panel-policy"
        aria-selected={workspaceMode === 'policy'}
        onclick={() => (workspaceMode = 'policy')}>정책</button
      >
      <button
        role="tab"
        id="tab-preview"
        aria-controls="panel-preview"
        aria-selected={workspaceMode === 'preview'}
        onclick={() => (workspaceMode = 'preview')}>미리보기</button
      >
    </div>
  </div>

  <div class="cols">
    <section id="panel-files" class="left" class:active={workspaceMode === 'files'} aria-label="파일 탐색기"><FileExplorer /></section>
    <section id="panel-policy" class="mid" class:active={workspaceMode === 'policy'} aria-label="정책 편집기"><PolicyEditor /></section>
    <section id="panel-preview" class="right" class:active={workspaceMode === 'preview'} aria-label="적용 결과 미리보기">
      <div class="rmode">
        <div class="segmented">
          <button aria-pressed={rightMode === 'effective'} onclick={() => setRightMode('effective')}>적용 결과</button>
          <button aria-pressed={rightMode === 'raw'} onclick={() => setRightMode('raw')}>Raw JSON</button>
        </div>
      </div>
      {#if rightMode === 'effective'}<EffectivePreview />{:else}<RawJsonEditor />{/if}
    </section>
  </div>
</div>

{#if candidates}
  <CandidateReview {candidates} existing={app.scoped[candidateScope].rules} scope={candidateScope}
    onapply={acceptCandidates} onclose={() => (candidates = null)} />
{/if}

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
            <li class:viewing={backupPreview?.rec.id === b.id}>
              <span class="binfo">{b.createdAt} · {b.scope}</span>
              <span class="bactions">
                <button class="btn btn-sm" onclick={() => showBackup(b)}>미리보기</button>
                <button class="btn btn-sm btn-primary" onclick={() => doRestore(b)}>복원…</button>
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
      <h3 id="sensitive-paths-title">이름 기반 검토 후보</h3>
      <p class="nochg">무시한 경로는 다음 후보 검토에서 제외됩니다. 비밀값 유무를 검사한 결과는 아닙니다.</p>
      {#if sensitivePaths.length === 0}
        <p class="nochg">발견된 민감 경로가 없습니다.</p>
      {:else}
        <ul class="blist">
          {#each sensitivePaths as item (item.id)}
            <li>
              <span class="binfo"><code>{item.path}</code> · {item.source}</span>
              <button class="btn btn-sm" onclick={() => toggleSensitivePath(item)}>
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
      <h3 id="project-report-title">정책 리포트 (Markdown)</h3>
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
    container-type: inline-size;
    height: 100dvh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.5rem 0.8rem;
    border-bottom: 1px solid var(--border);
    background: linear-gradient(180deg, var(--bg-1), var(--bg-0));
    flex-shrink: 0;
    min-width: 0;
  }
  .title {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
  }
  .pname {
    flex-shrink: 1;
    font-weight: 600;
    letter-spacing: -0.01em;
    max-width: 100%;
  }
  .scan-summary {
    flex: 1 1 0;
    font-size: var(--fs-sm);
    color: var(--text-3);
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    flex-shrink: 0;
  }
  .tools .chip {
    padding: 0 0.4rem;
  }
  .more {
    position: relative;
  }
  .menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    z-index: 40;
    min-width: 11rem;
    display: flex;
    flex-direction: column;
    padding: 0.3rem;
    background: var(--bg-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--r-md);
    box-shadow: var(--shadow-2);
    animation: ag-rise-in 120ms cubic-bezier(0.2, 0, 0, 1);
  }
  .menu button {
    background: none;
    border: 0;
    border-radius: var(--r-sm);
    padding: 0.4rem 0.6rem;
    text-align: left;
    color: var(--text-1);
    font-size: var(--fs-md);
    white-space: nowrap;
    cursor: pointer;
  }
  .menu button:hover:not(:disabled),
  .menu button:focus-visible {
    background: var(--bg-3);
    outline: none;
  }
  .menu button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .menu-sep {
    height: 1px;
    background: var(--border);
    margin: 0.25rem 0.2rem;
  }
  .save {
    flex-shrink: 0;
    font-weight: 600;
  }
  .save-dot {
    width: 7px;
    height: 7px;
    border-radius: 999px;
    background: var(--on-accent);
  }
  .dirty-line {
    margin: 0;
    padding: 0.25rem 0.8rem;
    font-size: var(--fs-xs);
    color: var(--accent-text);
    background: var(--accent-soft);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .notices {
    display: grid;
    gap: 0.4rem;
    padding: 0.5rem 0.8rem;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .notice {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    font-size: var(--fs-md);
  }
  .notice-actions {
    margin-left: auto;
    display: flex;
    gap: 0.35rem;
    flex-shrink: 0;
  }
  .cols {
    display: grid;
    grid-template-columns: minmax(220px, 0.8fr) minmax(320px, 1.2fr) minmax(280px, 1fr);
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
    align-items: center;
    min-height: 2.6rem;
    box-sizing: border-box;
    padding: 0.4rem 0.7rem;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .rmode .segmented > button {
    font-size: var(--fs-sm);
    padding: 0.2rem 0.65rem;
  }
  .right {
    display: flex;
    flex-direction: column;
  }
  .fp {
    color: var(--text-3);
    font-size: var(--fs-sm);
    font-family: var(--font-mono);
    margin: 0 0 0.6rem;
    word-break: break-all;
  }
  .nochg {
    color: var(--text-2);
    font-size: var(--fs-md);
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
    gap: 0.5rem;
    font-size: var(--fs-md);
    border: 1px solid var(--border);
    background: var(--bg-2);
    border-radius: var(--r-sm);
    padding: 0.35rem 0.6rem;
  }
  .blist li.viewing {
    border-color: var(--accent-line);
  }
  .binfo {
    min-width: 0;
    font-variant-numeric: tabular-nums;
    color: var(--text-2);
    overflow-wrap: anywhere;
  }
  .bactions {
    display: flex;
    gap: 0.35rem;
    flex-shrink: 0;
  }
  .preview {
    background: var(--bg-0);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    padding: 0.6rem;
    font-size: var(--fs-sm);
    max-height: 40vh;
    overflow: auto;
    white-space: pre-wrap;
    margin-top: 0.6rem;
  }

  @container (max-width: 960px) {
    .scan-summary {
      display: none;
    }
  }

  @container (max-width: 860px) {
    .workspace-tabs {
      display: flex;
      flex-shrink: 0;
      padding: 0.35rem 0.5rem;
      border-bottom: 1px solid var(--border);
      background: var(--bg-1);
    }
    .workspace-tabs .segmented {
      flex: 1;
    }
    .workspace-tabs .segmented > button {
      flex: 1 0 auto;
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
  }

  @container (max-width: 660px) {
    .top {
      gap: 0.4rem;
      padding: 0.45rem 0.55rem;
      flex-wrap: wrap;
    }
    .tools {
      order: 3;
      flex: 1 0 100%;
    }
    .notice {
      align-items: flex-start;
      flex-wrap: wrap;
    }
    .notice-actions {
      flex-basis: 100%;
      margin-left: 0;
    }
  }
</style>
