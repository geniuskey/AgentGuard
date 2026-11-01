<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import {
    inTauri,
    listAgentGlobals,
    listRecentProjects,
    pickFolder,
    type AgentGlobal,
    type ProjectRecord
  } from '$lib/ipc';
  import { app, loadProject } from '$lib/state.svelte';
  import { toast } from '$lib/toast.svelte';
  import { confirmDiscardChanges, hasUnsavedChanges } from '$lib/unsaved';
  import { prefs, toggleSidebar, toggleTheme } from '$lib/prefs.svelte';
  import { tooltip } from '$lib/tooltip';

  type Icon = 'home' | 'folder' | 'plus' | 'agent' | 'env' | 'guide' | 'settings';
  interface NavItem {
    href: string;
    label: string;
    icon: Icon;
    hint?: string;
    initial?: boolean;
  }

  const ICONS: Record<Icon, string> = {
    home: 'M4 10.5 12 4l8 6.5V20a1 1 0 0 1-1 1h-4.5v-6h-5v6H5a1 1 0 0 1-1-1v-9.5Z',
    folder: 'M3.5 7.5a2 2 0 0 1 2-2h4l2 2.2h7a2 2 0 0 1 2 2v7.8a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2V7.5Z',
    plus: 'M12 5v14M5 12h14',
    agent: 'M12 3.2 19.5 6v5.3c0 4.7-3 8.1-7.5 9.5-4.5-1.4-7.5-4.8-7.5-9.5V6L12 3.2Z',
    env: 'M4 6h16M4 12h16M4 18h10M17 16.5l2 1.5-2 1.5',
    guide: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18Zm-2.4-11.3a2.5 2.5 0 1 1 3.4 2.3c-.6.3-1 .8-1 1.4v.6M12 16.8v.2',
    settings:
      'M12 15.2a3.2 3.2 0 1 0 0-6.4 3.2 3.2 0 0 0 0 6.4Zm7.4-3.2c0-.5 0-.9-.1-1.3l2-1.6-2-3.4-2.4 1a7.6 7.6 0 0 0-2.2-1.3L14.3 3h-4.6l-.4 2.4c-.8.3-1.5.8-2.2 1.3l-2.4-1-2 3.4 2 1.6a7.7 7.7 0 0 0 0 2.6l-2 1.6 2 3.4 2.4-1c.7.5 1.4 1 2.2 1.3l.4 2.4h4.6l.4-2.4c.8-.3 1.5-.8 2.2-1.3l2.4 1 2-3.4-2-1.6c.1-.4.1-.8.1-1.3Z'
  };

  const RECENT_LIMIT = 5;

  let agents = $state<AgentGlobal[]>([]);
  let recent = $state<ProjectRecord[]>([]);
  let opening = $state(false);

  onMount(async () => {
    if (!inTauri()) return;
    try {
      agents = await listAgentGlobals();
    } catch {
      /* sidebar falls back to the Claude Code entry */
    }
  });

  $effect(() => {
    void app.projectRoot;
    if (!inTauri()) return;
    listRecentProjects()
      .then((list) => (recent = list))
      .catch(() => {});
  });

  const otherRecent = $derived(
    recent.filter((p) => p.path !== app.projectRoot).slice(0, app.loaded ? RECENT_LIMIT - 1 : RECENT_LIMIT)
  );

  async function switchTo(path: string | null) {
    if (!path || opening) return;
    if (!confirmDiscardChanges(hasUnsavedChanges(), window.confirm.bind(window))) return;
    opening = true;
    try {
      await loadProject(path);
      if ($page.url.pathname !== '/project') await goto('/project');
    } catch (e) {
      toast(`프로젝트를 열지 못했습니다: ${e}`, 'error');
    } finally {
      opening = false;
    }
  }

  async function openFolder() {
    if (!inTauri()) {
      toast('데스크톱 앱에서만 폴더를 열 수 있습니다.', 'warn');
      return;
    }
    await switchTo(await pickFolder());
  }

  const agentItems = $derived<NavItem[]>(
    agents.length
      ? agents.map((a) => ({
          href: a.route,
          label: a.name,
          icon: 'agent' as const,
          hint: a.exists ? undefined : '설정 파일 없음'
        }))
      : [{ href: '/user', label: 'Claude Code', icon: 'agent' }]
  );

  function isActive(href: string): boolean {
    const url = $page.url;
    const [path, query] = href.split('?');
    if (path === '/') return url.pathname === '/';
    if (url.pathname !== path) return false;
    if (!query) return true;
    const want = new URLSearchParams(query);
    return [...want].every(([k, v]) => url.searchParams.get(k) === v);
  }
</script>

{#snippet icon(name: Icon)}
  <svg viewBox="0 0 24 24" width="18" height="18" fill="none" aria-hidden="true">
    <path d={ICONS[name]} stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
  </svg>
{/snippet}

{#snippet glyph(name: Icon, label: string, initial = false)}
  {#if initial && prefs.sidebarCollapsed}
    <span class="initial" aria-hidden="true">{label.slice(0, 1).toUpperCase()}</span>
  {:else}
    {@render icon(name)}
  {/if}
{/snippet}

{#snippet link(item: NavItem, extra?: string)}
  <a
    href={item.href}
    class="nav-item"
    aria-current={isActive(item.href) ? 'page' : undefined}
    use:tooltip={prefs.sidebarCollapsed ? item.label : undefined}
  >
    {@render glyph(item.icon, item.label, item.initial)}
    <span class="label truncate">{item.label}</span>
    {#if extra}<span class="badge">{extra}</span>{/if}
    {#if item.hint && !prefs.sidebarCollapsed}<span class="hint">{item.hint}</span>{/if}
  </a>
{/snippet}

<nav class="sidebar" class:collapsed={prefs.sidebarCollapsed} aria-label="주 메뉴">
  <div class="brand">
    <svg viewBox="0 0 24 24" width="22" height="22" fill="none" aria-hidden="true">
      <path
        d="M12 2.2 20 5.4v5.7c0 5.4-3.3 9.2-8 10.7-4.7-1.5-8-5.3-8-10.7V5.4l8-3.2Z"
        fill="var(--accent)"
        fill-opacity="0.16"
        stroke="var(--accent)"
        stroke-width="1.5"
        stroke-linejoin="round"
      />
      <path d="m8.6 11.8 2.4 2.4 4.4-4.6" stroke="var(--accent-text)" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
    </svg>
    <span class="label">Agent Guard</span>
  </div>

  <div class="scroll">
    {@render link({ href: '/', label: '홈', icon: 'home' })}

    <div class="group-title">프로젝트</div>
    {#if app.loaded}
      {@render link({ href: '/project', label: app.projectName, icon: 'folder', initial: true }, app.dirty ? '●' : undefined)}
    {/if}
    {#each otherRecent as p (p.id)}
      <button
        class="nav-item"
        onclick={() => switchTo(p.path)}
        disabled={opening}
        title={p.path}
        use:tooltip={prefs.sidebarCollapsed ? p.name : undefined}
      >
        {@render glyph('folder', p.name, true)}
        <span class="label truncate">{p.name}</span>
      </button>
    {/each}
    <button
      class="nav-item subtle"
      onclick={openFolder}
      disabled={opening}
      use:tooltip={prefs.sidebarCollapsed ? '폴더 열기…' : undefined}
    >
      {@render icon('plus')}
      <span class="label">{opening ? '여는 중…' : '폴더 열기…'}</span>
    </button>

    <div class="group-title">전역 설정</div>
    {#each agentItems as item (item.href)}
      {@render link(item)}
    {/each}
    {@render link({ href: '/env', label: '환경 변수', icon: 'env' })}
  </div>

  <div class="footer">
    {@render link({ href: '/guide', label: '사용 가이드', icon: 'guide' })}
    {@render link({ href: '/settings', label: '설정', icon: 'settings' })}
    <div class="footer-row">
      <button
        class="btn btn-ghost btn-icon"
        onclick={toggleTheme}
        aria-label={prefs.resolved === 'dark' ? '라이트 테마로 전환' : '다크 테마로 전환'}
        use:tooltip={prefs.resolved === 'dark' ? '라이트 테마로 전환' : '다크 테마로 전환'}
      >
        {#if prefs.resolved === 'dark'}
          <svg viewBox="0 0 24 24" width="16" height="16" fill="none" aria-hidden="true">
            <circle cx="12" cy="12" r="4.2" stroke="currentColor" stroke-width="1.6" />
            <path
              d="M12 3v2.2M12 18.8V21M21 12h-2.2M5.2 12H3m14.8-6.8-1.6 1.6M7.8 16.2l-1.6 1.6m11.6 0-1.6-1.6M7.8 7.8 6.2 6.2"
              stroke="currentColor"
              stroke-width="1.6"
              stroke-linecap="round"
            />
          </svg>
        {:else}
          <svg viewBox="0 0 24 24" width="16" height="16" fill="none" aria-hidden="true">
            <path d="M20.2 14.1A8.2 8.2 0 0 1 9.9 3.8a8.2 8.2 0 1 0 10.3 10.3Z" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round" />
          </svg>
        {/if}
      </button>
      <button
        class="btn btn-ghost btn-icon"
        onclick={toggleSidebar}
        aria-label={prefs.sidebarCollapsed ? '사이드바 펼치기 (Ctrl+B)' : '사이드바 접기 (Ctrl+B)'}
        aria-expanded={!prefs.sidebarCollapsed}
        use:tooltip={prefs.sidebarCollapsed ? '사이드바 펼치기 (Ctrl+B)' : '사이드바 접기 (Ctrl+B)'}
      >
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" aria-hidden="true">
          <rect x="3.5" y="4.5" width="17" height="15" rx="2" stroke="currentColor" stroke-width="1.6" />
          <path d="M9.5 4.5v15" stroke="currentColor" stroke-width="1.6" />
          <path
            d={prefs.sidebarCollapsed ? 'm13.5 10 2 2-2 2' : 'm16 10-2 2 2 2'}
            stroke="currentColor"
            stroke-width="1.6"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </button>
    </div>
  </div>
</nav>

<style>
  .sidebar {
    position: sticky;
    top: 0;
    height: 100vh;
    width: 216px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    box-sizing: border-box;
    background: var(--bg-1);
    border-right: 1px solid var(--border);
    transition: width var(--t-fast);
  }
  .sidebar.collapsed {
    width: 56px;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    height: 3rem;
    padding: 0 1.05rem;
    font-weight: 700;
    font-size: var(--fs-lg);
    letter-spacing: -0.01em;
    white-space: nowrap;
    overflow: hidden;
    flex-shrink: 0;
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 0.25rem 0.5rem;
  }
  .group-title {
    margin: 0.9rem 0.55rem 0.3rem;
    color: var(--text-3);
    font-size: var(--fs-xs);
    font-weight: 600;
    white-space: nowrap;
  }
  .collapsed .group-title {
    height: 1px;
    margin: 0.6rem 0.4rem;
    background: var(--border);
    font-size: 0;
  }
  .nav-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 0.6rem;
    min-height: 2.1rem;
    padding: 0 0.55rem;
    border-radius: var(--r-sm);
    color: var(--text-2);
    font-size: var(--fs-md);
    text-decoration: none;
    white-space: nowrap;
    transition: background-color var(--t-fast), color var(--t-fast);
  }
  .nav-item svg {
    flex-shrink: 0;
  }
  .nav-item:hover {
    background: var(--hover);
    color: var(--text-1);
  }
  .nav-item[aria-current='page'] {
    background: var(--accent-soft);
    color: var(--accent-text);
    font-weight: 600;
  }
  button.nav-item {
    width: 100%;
    border: 0;
    background: transparent;
    font: inherit;
    font-size: var(--fs-md);
    text-align: left;
    cursor: pointer;
  }
  button.nav-item:disabled {
    cursor: progress;
    opacity: 0.6;
  }
  .nav-item.subtle {
    color: var(--text-3);
  }
  .label {
    flex: 1;
  }
  .initial {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border: 1px solid var(--border-strong);
    border-radius: var(--r-xs);
    background: var(--bg-2);
    font-size: var(--fs-sm);
    font-weight: 700;
  }
  .nav-item[aria-current='page'] .initial {
    border-color: var(--accent-line);
    background: var(--accent-soft);
  }
  .badge {
    color: var(--ask);
    font-size: var(--fs-xs);
  }
  .hint {
    color: var(--text-3);
    font-size: var(--fs-xs);
  }
  .collapsed .label,
  .collapsed .hint,
  .collapsed .brand .label {
    display: none;
  }
  .collapsed .badge {
    position: absolute;
    top: 0.1rem;
    right: 0.45rem;
  }
  .collapsed .nav-item {
    justify-content: center;
    padding: 0;
  }
  .collapsed .brand {
    justify-content: center;
    padding: 0;
  }
  .footer {
    flex-shrink: 0;
    padding: 0.4rem 0.5rem 0.55rem;
    border-top: 1px solid var(--border);
  }
  .footer-row {
    display: flex;
    justify-content: space-between;
    margin-top: 0.3rem;
    padding: 0 0.15rem;
  }
  .collapsed .footer-row {
    flex-direction: column;
    align-items: center;
    gap: 0.2rem;
  }
</style>
