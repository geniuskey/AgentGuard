<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import {
    appInfo,
    diagnosticReport,
    inTauri,
    listAgentGlobals,
    listRecentProjects,
    loadSettings,
    openProject,
    pickFolder,
    type AgentGlobal,
    type AppInfo,
    type ProjectRecord
  } from '$lib/ipc';
  import { setProject, refreshEffective } from '$lib/state.svelte';
  import { themeState, toggleTheme } from '$lib/theme.svelte';
  import { toast } from '$lib/toast.svelte';
  import HelpButton from '$lib/components/HelpButton.svelte';

  let info = $state<AppInfo | null>(null);
  let recent = $state<ProjectRecord[]>([]);
  let agents = $state<AgentGlobal[]>([]);
  let error = $state<string | null>(null);
  let busy = $state(false);
  let loading = $state(true);
  let copying = $state(false);
  const desktop = inTauri();
  const themeLabel = $derived(themeState.current === 'dark' ? '라이트' : '다크');

  onMount(async () => {
    const [i, r, a] = await Promise.allSettled([appInfo(), listRecentProjects(), listAgentGlobals()]);
    if (i.status === 'fulfilled') info = i.value;
    if (r.status === 'fulfilled') recent = r.value;
    if (a.status === 'fulfilled') agents = a.value;
    const failed = [i, r, a].find((x) => x.status === 'rejected');
    if (failed && desktop) error = String((failed as PromiseRejectedResult).reason);
    loading = false;
  });

  function displayPath(path: string) {
    return path.includes('\\') ? path.replaceAll('/', '\\') : path;
  }

  function formatOpened(at: string) {
    const d = new Date(at);
    if (Number.isNaN(d.getTime())) return '';
    return d.toLocaleString('ko-KR', {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    });
  }

  async function open(path: string | null) {
    if (!path) return;
    busy = true;
    error = null;
    try {
      const view = await openProject(path);
      const scoped = await loadSettings(view.project.path);
      setProject(view, scoped);
      await refreshEffective();
      await goto('/project');
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function onOpenClick() {
    if (!desktop) {
      error = '데스크톱 앱에서만 폴더를 열 수 있습니다 (npm run tauri dev).';
      return;
    }
    await open(await pickFolder());
  }

  async function copyDiagnostics() {
    copying = true;
    try {
      const report = await diagnosticReport();
      await navigator.clipboard.writeText(report);
      toast('비밀값을 제외한 진단 정보를 클립보드에 복사했습니다.');
    } catch (e) {
      toast(`진단 정보 복사 실패: ${String(e)}`, 'error');
    } finally {
      copying = false;
    }
  }
</script>

<main>
  <header class="hero">
    <div class="logo" aria-hidden="true">
      <svg viewBox="0 0 24 24" width="44" height="44" fill="none">
        <defs>
          <linearGradient id="ag-shield" x1="4" y1="2" x2="20" y2="22" gradientUnits="userSpaceOnUse">
            <stop offset="0" style="stop-color: var(--accent-text)" />
            <stop offset="1" style="stop-color: var(--accent-strong)" />
          </linearGradient>
        </defs>
        <path
          d="M12 2.2 20 5.4v5.7c0 5.4-3.3 9.2-8 10.7-4.7-1.5-8-5.3-8-10.7V5.4l8-3.2Z"
          fill="url(#ag-shield)"
          fill-opacity="0.16"
          stroke="url(#ag-shield)"
          stroke-width="1.5"
          stroke-linejoin="round"
        />
        <path
          d="m8.6 11.8 2.4 2.4 4.4-4.6"
          style="stroke: var(--accent-text)"
          stroke-width="1.8"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </div>
    <div class="hero-text">
      <h1>Agent Guard</h1>
      <p class="tagline">
        코딩 에이전트가 프로젝트에서 접근할 수 있는 경계를 눈으로 보고 안전하게 설정하세요.
      </p>
    </div>
    <div class="hero-actions">
      <HelpButton section="home" />
      <button
        class="theme-toggle"
        onclick={toggleTheme}
        title={themeState.current === 'dark' ? '라이트 테마로 전환' : '다크 테마로 전환'}
        aria-label="{themeLabel} 테마로 전환"
      >
        {#if themeState.current === 'dark'}
          <svg viewBox="0 0 24 24" width="15" height="15" fill="none" aria-hidden="true">
            <circle cx="12" cy="12" r="4.2" stroke="currentColor" stroke-width="1.6" />
            <path
              d="M12 3v2.2M12 18.8V21M21 12h-2.2M5.2 12H3m14.8-6.8-1.6 1.6M7.8 16.2l-1.6 1.6m11.6 0-1.6-1.6M7.8 7.8 6.2 6.2"
              stroke="currentColor"
              stroke-width="1.6"
              stroke-linecap="round"
            />
          </svg>
        {:else}
          <svg viewBox="0 0 24 24" width="15" height="15" fill="none" aria-hidden="true">
            <path
              d="M20.2 14.1A8.2 8.2 0 0 1 9.9 3.8a8.2 8.2 0 1 0 10.3 10.3Z"
              stroke="currentColor"
              stroke-width="1.6"
              stroke-linejoin="round"
            />
          </svg>
        {/if}
        {themeLabel}
      </button>
    </div>
  </header>

  {#if error}
    <p class="callout callout-error home-error" role="alert">{error}</p>
  {/if}

  <section class="actions">
    <button class="primary" onclick={onOpenClick} disabled={busy}>
      <svg viewBox="0 0 24 24" width="17" height="17" fill="none" aria-hidden="true">
        <path
          d="M3.5 7.5v10a1.5 1.5 0 0 0 1.5 1.5h14a1.5 1.5 0 0 0 1.5-1.5V9A1.5 1.5 0 0 0 19 7.5h-7.2L9.9 5.4a1.5 1.5 0 0 0-1.06-.44H5A1.5 1.5 0 0 0 3.5 6.5v1Z"
          stroke="currentColor"
          stroke-width="1.6"
          stroke-linejoin="round"
        />
      </svg>
      {busy ? '여는 중…' : '프로젝트 열기'}
    </button>
  </section>

  <section>
    <div class="sec-head">
      <h2>전역 에이전트 설정</h2>
      {#if agents.length}<span class="count">{agents.length}</span>{/if}
    </div>
    <p class="sec-hint">
      홈 폴더(<code>~/</code>)에서 발견한 코딩 에이전트의 전역 설정입니다. 클릭하면 해당 에이전트
      설정으로 바로 들어갑니다.
    </p>
    {#if loading}
      <p class="muted" role="status">불러오는 중…</p>
    {:else if agents.length === 0}
      <p class="muted">
        {desktop
          ? '발견한 에이전트 전역 설정이 없습니다.'
          : '데스크톱 앱에서 실행하면 에이전트 설정 목록이 표시됩니다.'}
      </p>
    {:else}
      <ul class="cards two-col">
        {#each agents as a (a.id)}
          <li>
            <button class="card" onclick={() => goto(a.route)} disabled={busy}>
              <div class="card-top">
                <span class="name">{a.name}</span>
                <span class="tags">
                  <span class="chip">{a.format.toUpperCase()}</span>
                  {#if a.structured}<span class="chip chip-accent">시각 편집</span>{/if}
                  {#if a.exists}
                    <span class="chip chip-allow"><span class="dot" aria-hidden="true"></span>설정됨</span>
                  {:else}
                    <span class="chip">없음</span>
                  {/if}
                </span>
              </div>
              <div class="path">{displayPath(a.path)}</div>
              <div class="meta">{a.description}</div>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section>
    <div class="sec-head">
      <h2>최근 프로젝트</h2>
      {#if recent.length}<span class="count">{recent.length}</span>{/if}
    </div>
    {#if loading}
      <p class="muted" role="status">불러오는 중…</p>
    {:else if recent.length === 0}
      <div class="empty">
        <p class="empty-title">아직 열어본 프로젝트가 없습니다. 이렇게 시작하세요.</p>
        <ol>
          <li><strong>프로젝트 열기</strong>로 작업 폴더를 선택합니다.</li>
          <li>민감 경로 후보와 위험 점수를 검토합니다.</li>
          <li>허용·확인·차단 규칙을 정리해 저장합니다.</li>
        </ol>
      </div>
    {:else}
      <ul class="rows">
        {#each recent as p (p.id)}
          {@const opened = formatOpened(p.lastOpenedAt)}
          <li>
            <button class="row" onclick={() => open(p.path)} disabled={busy}>
              <span class="row-main">
                <span class="name">{p.name}</span>
                <span class="path">{displayPath(p.path)}</span>
              </span>
              {#if opened}<time class="when" datetime={p.lastOpenedAt}>{opened}</time>{/if}
              <svg class="arrow" viewBox="0 0 24 24" width="16" height="16" fill="none" aria-hidden="true">
                <path d="m9 6 6 6-6 6" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
              </svg>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <footer>
    <span class="footer-info truncate">
      {#if info}{info.name} v{info.version} · db v{info.dbSchemaVersion} · {displayPath(info.dataDir)}{/if}
    </span>
    {#if desktop}
      <button class="btn btn-ghost btn-sm" onclick={copyDiagnostics} disabled={copying}>
        {copying ? '복사 중…' : '진단 정보 복사'}
      </button>
    {/if}
  </footer>
</main>

<style>
  main {
    max-width: 920px;
    margin: 0 auto;
    padding: 1.6rem 1.5rem 1.5rem;
  }
  .hero {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  .hero-text {
    min-width: 0;
  }
  .hero-actions {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-shrink: 0;
    align-self: flex-start;
  }
  .hero-actions :global(button) {
    white-space: nowrap;
  }
  .theme-toggle {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    background: var(--bg-1);
    border: 1px solid var(--border-strong);
    color: var(--text-2);
    border-radius: 999px;
    padding: 0.34rem 0.8rem;
    cursor: pointer;
    font-size: var(--fs-sm);
    font-weight: 600;
    white-space: nowrap;
    transition: color var(--t-fast), border-color var(--t-fast), background-color var(--t-fast);
  }
  .theme-toggle:hover {
    color: var(--text-1);
    border-color: var(--accent);
    background: var(--bg-2);
  }
  .logo {
    flex-shrink: 0;
    display: grid;
    place-items: center;
    width: 64px;
    height: 64px;
    border-radius: 18px;
    background: linear-gradient(160deg, var(--bg-3), var(--bg-1));
    border: 1px solid var(--border-strong);
    box-shadow: var(--shadow-1);
  }
  h1 {
    margin: 0;
    font-size: 1.9rem;
    letter-spacing: -0.03em;
    background: linear-gradient(100deg, var(--text-1) 55%, var(--accent-text));
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
  }
  .tagline {
    color: var(--text-2);
    margin: 0.25rem 0 0;
    text-wrap: pretty;
  }
  .home-error {
    margin-top: 1rem;
  }
  .actions {
    margin: 1.3rem 0 1.6rem;
    display: flex;
    gap: 0.6rem;
    flex-wrap: wrap;
  }
  button.primary {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.65rem 1.3rem;
    border-radius: var(--r-md);
    border: 1px solid var(--accent-line);
    background: var(--accent-strong);
    color: var(--on-accent);
    font-size: 0.98rem;
    font-weight: 600;
    white-space: nowrap;
    cursor: pointer;
    box-shadow: var(--glow-accent);
    transition: background-color var(--t-fast), transform var(--t-fast);
  }
  button.primary:hover:not(:disabled) {
    background: var(--accent-hover);
    transform: translateY(-1px);
  }
  button.primary:active:not(:disabled) {
    transform: translateY(0);
  }
  button.primary:disabled {
    opacity: 0.6;
    cursor: default;
  }
  section {
    margin-bottom: 1.6rem;
  }
  .sec-head {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.5rem;
    margin-bottom: 0.8rem;
  }
  h2 {
    margin: 0;
    font-size: 0.86rem;
    font-weight: 600;
    letter-spacing: 0.05em;
    color: var(--text-2);
  }
  .count {
    font-size: var(--fs-xs);
    color: var(--text-3);
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0.05rem 0.5rem;
    font-variant-numeric: tabular-nums;
  }
  .sec-hint {
    color: var(--text-3);
    font-size: var(--fs-md);
    margin: -0.2rem 0 0.8rem;
  }
  .sec-hint code {
    background: var(--bg-1);
    border: 1px solid var(--border);
    padding: 0 0.3rem;
    border-radius: 4px;
    font-size: 0.9em;
  }
  .tags {
    display: flex;
    gap: 0.3rem;
    align-items: center;
    flex-shrink: 0;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 999px;
    background: var(--allow);
  }
  .cards {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: 0.6rem;
  }
  .cards.two-col {
    grid-template-columns: repeat(auto-fill, minmax(min(320px, 100%), 1fr));
  }
  .card {
    width: 100%;
    height: 100%;
    text-align: left;
    padding: 0.85rem 1rem;
    border-radius: var(--r-md);
    border: 1px solid var(--border);
    background: linear-gradient(180deg, var(--bg-2), var(--bg-1));
    color: var(--text-1);
    cursor: pointer;
    box-sizing: border-box;
    transition: border-color var(--t-fast), transform var(--t-fast), box-shadow var(--t-fast);
  }
  .card:hover:not(:disabled) {
    border-color: var(--border-strong);
    transform: translateY(-1px);
    box-shadow: var(--shadow-2);
  }
  .card:disabled,
  .row:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .card-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
  }
  .name {
    font-weight: 600;
    letter-spacing: -0.01em;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .path {
    color: var(--text-3);
    font-size: var(--fs-sm);
    font-family: var(--font-mono);
    margin-top: 0.25rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    color: var(--text-2);
    font-size: var(--fs-md);
    margin-top: 0.4rem;
  }
  .rows {
    list-style: none;
    padding: 0;
    margin: 0;
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    background: var(--bg-1);
    overflow: hidden;
  }
  .rows li + li {
    border-top: 1px solid var(--border);
  }
  .row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 0.8rem;
    padding: 0.55rem 0.9rem;
    border: 0;
    background: transparent;
    color: var(--text-1);
    text-align: left;
    cursor: pointer;
    transition: background-color var(--t-fast);
  }
  .row:hover:not(:disabled) {
    background: var(--hover);
  }
  .row:hover:not(:disabled) .arrow {
    color: var(--accent-text);
  }
  .row-main {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: 0.7rem;
  }
  .row-main .name {
    flex-shrink: 0;
    max-width: 40%;
  }
  .row-main .path {
    margin: 0;
    min-width: 0;
  }
  .when {
    flex-shrink: 0;
    color: var(--text-3);
    font-size: var(--fs-sm);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .arrow {
    flex-shrink: 0;
    color: var(--text-3);
  }
  .empty {
    border: 1px dashed var(--border-strong);
    border-radius: var(--r-md);
    padding: 0.8rem 1rem;
    font-size: var(--fs-md);
    color: var(--text-2);
  }
  .empty-title {
    margin: 0 0 0.4rem;
    color: var(--text-1);
  }
  .empty ol {
    margin: 0;
    padding-left: 1.2rem;
    line-height: 1.7;
  }
  footer {
    margin-top: 2rem;
    padding-top: 0.8rem;
    border-top: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: 0.75rem;
    font-size: var(--fs-sm);
    font-family: var(--font-mono);
    color: var(--text-3);
  }
  .footer-info {
    flex: 1;
  }
  footer .btn {
    flex-shrink: 0;
    font-family: var(--font-sans);
  }
</style>
