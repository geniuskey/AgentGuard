<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import {
    appInfo,
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

  let info = $state<AppInfo | null>(null);
  let recent = $state<ProjectRecord[]>([]);
  let agents = $state<AgentGlobal[]>([]);
  let error = $state<string | null>(null);
  let busy = $state(false);

  onMount(async () => {
    try {
      info = await appInfo();
      recent = await listRecentProjects();
      agents = await listAgentGlobals();
    } catch (e) {
      error = String(e);
    }
  });

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
    if (!inTauri()) {
      error = '데스크톱 앱에서만 폴더를 열 수 있습니다 (npm run tauri dev).';
      return;
    }
    await open(await pickFolder());
  }
</script>

<main>
  <header class="hero">
    <div class="logo" aria-hidden="true">
      <svg viewBox="0 0 24 24" width="52" height="52" fill="none">
        <defs>
          <linearGradient id="ag-shield" x1="4" y1="2" x2="20" y2="22" gradientUnits="userSpaceOnUse">
            <stop offset="0" stop-color="#60a5fa" />
            <stop offset="1" stop-color="#2563eb" />
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
          stroke="#7db2fb"
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
  </header>

  {#if error}
    <p class="error" role="alert">{error}</p>
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
    {#if agents.length === 0}
      <p class="muted">데스크톱 앱에서 실행하면 에이전트 설정 목록이 표시됩니다.</p>
    {:else}
      <ul class="cards two-col">
        {#each agents as a (a.id)}
          <li>
            <button class="card" onclick={() => goto(a.route)} disabled={busy}>
              <div class="card-top">
                <span class="name">{a.name}</span>
                <span class="tags">
                  <span class="tag tag-{a.format}">{a.format.toUpperCase()}</span>
                  {#if a.structured}<span class="tag tag-vis">Visual</span>{/if}
                  {#if a.exists}
                    <span class="tag tag-on"><span class="dot" aria-hidden="true"></span>설정됨</span>
                  {:else}
                    <span class="tag tag-off">없음</span>
                  {/if}
                </span>
              </div>
              <div class="path">{a.path}</div>
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
    {#if recent.length === 0}
      <p class="muted">아직 열어본 프로젝트가 없습니다.</p>
    {:else}
      <ul class="cards">
        {#each recent as p (p.id)}
          <li>
            <button class="card" onclick={() => open(p.path)} disabled={busy}>
              <div class="card-top">
                <span class="name">{p.name}</span>
                {#if p.riskLevel}
                  <span class="risk risk-{p.riskLevel.toLowerCase()}">{p.riskLevel}</span>
                {/if}
              </div>
              <div class="path">{p.path}</div>
              <div class="meta">
                {p.riskProfile ?? 'No profile'} · score {p.riskScore ?? '?'}
              </div>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  {#if info}
    <footer>{info.name} v{info.version} · db v{info.dbSchemaVersion} · {info.dataDir}</footer>
  {/if}
</main>

<style>
  main {
    max-width: 920px;
    margin: 0 auto;
    padding: 3rem 1.5rem 2rem;
  }
  .hero {
    display: flex;
    align-items: center;
    gap: 1.1rem;
  }
  .logo {
    flex-shrink: 0;
    display: grid;
    place-items: center;
    width: 76px;
    height: 76px;
    border-radius: 22px;
    background: linear-gradient(160deg, var(--bg-3), var(--bg-1));
    border: 1px solid var(--border-strong);
    box-shadow: var(--shadow-1), inset 0 1px 0 rgba(255, 255, 255, 0.05);
  }
  h1 {
    margin: 0;
    font-size: 2.1rem;
    letter-spacing: -0.03em;
    background: linear-gradient(100deg, var(--text-1) 55%, var(--accent-text));
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
  }
  .tagline {
    color: var(--text-2);
    margin: 0.3rem 0 0;
    text-wrap: pretty;
  }
  .actions {
    margin: 1.8rem 0 2.2rem;
    display: flex;
    gap: 0.6rem;
    flex-wrap: wrap;
  }
  button.primary {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.7rem 1.35rem;
    border-radius: var(--r-md);
    border: 1px solid rgba(147, 197, 253, 0.25);
    background: linear-gradient(180deg, #3b82f6, #2563eb);
    color: white;
    font-size: 0.98rem;
    font-weight: 600;
    cursor: pointer;
    box-shadow: 0 4px 18px rgba(37, 99, 235, 0.3), inset 0 1px 0 rgba(255, 255, 255, 0.18);
    transition: box-shadow var(--t-fast), transform var(--t-fast), filter var(--t-fast);
  }
  button.primary:hover:not(:disabled) {
    filter: brightness(1.08);
    transform: translateY(-1px);
    box-shadow: 0 6px 24px rgba(37, 99, 235, 0.4), inset 0 1px 0 rgba(255, 255, 255, 0.18);
  }
  button.primary:active:not(:disabled) {
    transform: translateY(0);
  }
  button.primary:disabled {
    opacity: 0.6;
    cursor: default;
  }
  section {
    margin-bottom: 2rem;
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
    font-size: 0.68rem;
    color: var(--text-3);
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0.05rem 0.5rem;
    font-variant-numeric: tabular-nums;
  }
  .sec-hint {
    color: var(--text-3);
    font-size: 0.8rem;
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
  .tag {
    font-size: 0.66rem;
    font-weight: 600;
    padding: 0.12rem 0.5rem;
    border-radius: 999px;
    border: 1px solid var(--border-strong);
    color: var(--text-2);
    display: inline-flex;
    align-items: center;
    gap: 0.28rem;
  }
  .tag-json {
    color: var(--accent-text);
    border-color: rgba(79, 142, 247, 0.35);
    background: var(--accent-soft);
  }
  .tag-toml {
    color: #fda4af;
    border-color: rgba(248, 113, 113, 0.3);
    background: var(--deny-soft);
  }
  .tag-vis {
    color: var(--allow);
    border-color: rgba(52, 211, 153, 0.3);
    background: var(--allow-soft);
  }
  .tag-on {
    color: var(--allow);
    border-color: rgba(52, 211, 153, 0.3);
  }
  .tag-on .dot {
    width: 6px;
    height: 6px;
    border-radius: 999px;
    background: var(--allow);
    box-shadow: 0 0 6px rgba(52, 211, 153, 0.8);
  }
  .tag-off {
    color: var(--text-3);
    border-color: var(--border);
  }
  .cards {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: 0.6rem;
  }
  .cards.two-col {
    grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
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
  .card:disabled {
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
    font-size: 0.76rem;
    font-family: var(--font-mono);
    margin-top: 0.25rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    color: var(--text-2);
    font-size: 0.79rem;
    margin-top: 0.4rem;
  }
  .risk {
    font-size: 0.68rem;
    font-weight: 700;
    padding: 0.12rem 0.55rem;
    border-radius: 999px;
    flex-shrink: 0;
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
  .muted {
    color: var(--text-3);
    font-size: 0.85rem;
  }
  .error {
    color: var(--deny);
    background: var(--deny-soft);
    border: 1px solid rgba(248, 113, 113, 0.3);
    border-radius: var(--r-sm);
    padding: 0.5rem 0.8rem;
    font-size: 0.85rem;
  }
  footer {
    margin-top: 2.5rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border);
    font-size: 0.74rem;
    font-family: var(--font-mono);
    color: var(--text-3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
