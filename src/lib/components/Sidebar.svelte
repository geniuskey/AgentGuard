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
  import { projectMark } from '$lib/project-mark';

  type Icon = 'home' | 'plus' | 'switch' | 'env' | 'guide' | 'settings';
  interface NavItem {
    href: string;
    label: string;
    icon?: Icon;
    agent?: string;
    hint?: string;
  }

  const ICONS: Record<Icon, string> = {
    home: 'M4 10.5 12 4l8 6.5V20a1 1 0 0 1-1 1h-4.5v-6h-5v6H5a1 1 0 0 1-1-1v-9.5Z',
    plus: 'M12 5v14M5 12h14',
    switch: 'm8 9 4-4 4 4M8 15l4 4 4-4',
    env: 'M4 6h16M4 12h16M4 18h10M17 16.5l2 1.5-2 1.5',
    guide: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18Zm-2.4-11.3a2.5 2.5 0 1 1 3.4 2.3c-.6.3-1 .8-1 1.4v.6M12 16.8v.2',
    settings:
      'M12 15.2a3.2 3.2 0 1 0 0-6.4 3.2 3.2 0 0 0 0 6.4Zm7.4-3.2c0-.5 0-.9-.1-1.3l2-1.6-2-3.4-2.4 1a7.6 7.6 0 0 0-2.2-1.3L14.3 3h-4.6l-.4 2.4c-.8.3-1.5.8-2.2 1.3l-2.4-1-2 3.4 2 1.6a7.7 7.7 0 0 0 0 2.6l-2 1.6 2 3.4 2.4-1c.7.5 1.4 1 2.2 1.3l.4 2.4h4.6l.4-2.4c.8-.3 1.5-.8 2.2-1.3l2.4 1 2-3.4-2-1.6c.1-.4.1-.8.1-1.3Z'
  };

  // Brand marks: Claude (Simple Icons, CC0), Codex / OpenCode (Lobe Icons, MIT).
  const AGENT_LOGOS: Record<string, { d: string; color?: string; evenodd?: boolean }> = {
    'claude-code': {
      color: '#d97757',
      d: 'm4.7144 15.9555 4.7174-2.6471.079-.2307-.079-.1275h-.2307l-.7893-.0486-2.6956-.0729-2.3375-.0971-2.2646-.1214-.5707-.1215-.5343-.7042.0546-.3522.4797-.3218.686.0608 1.5179.1032 2.2767.1578 1.6514.0972 2.4468.255h.3886l.0546-.1579-.1336-.0971-.1032-.0972L6.973 9.8356l-2.55-1.6879-1.3356-.9714-.7225-.4918-.3643-.4614-.1578-1.0078.6557-.7225.8803.0607.2246.0607.8925.686 1.9064 1.4754 2.4893 1.8336.3643.3035.1457-.1032.0182-.0728-.164-.2733-1.3539-2.4467-1.445-2.4893-.6435-1.032-.17-.6194c-.0607-.255-.1032-.4674-.1032-.7285L6.287.1335 6.6997 0l.9957.1336.419.3642.6192 1.4147 1.0018 2.2282 1.5543 3.0296.4553.8985.2429.8318.091.255h.1579v-.1457l.1275-1.706.2368-2.0947.2307-2.6957.0789-.7589.3764-.9107.7468-.4918.5828.2793.4797.686-.0668.4433-.2853 1.8517-.5586 2.9021-.3643 1.9429h.2125l.2429-.2429.9835-1.3053 1.6514-2.0643.7286-.8196.85-.9046.5464-.4311h1.0321l.759 1.1293-.34 1.1657-1.0625 1.3478-.8804 1.1414-1.2628 1.7-.7893 1.36.0729.1093.1882-.0183 2.8535-.607 1.5421-.2794 1.8396-.3157.8318.3886.091.3946-.3278.8075-1.967.4857-2.3072.4614-3.4364.8136-.0425.0304.0486.0607 1.5482.1457.6618.0364h1.621l3.0175.2247.7892.522.4736.6376-.079.4857-1.2142.6193-1.6393-.3886-3.825-.9107-1.3113-.3279h-.1822v.1093l1.0929 1.0686 2.0035 1.8092 2.5075 2.3314.1275.5768-.3218.4554-.34-.0486-2.2039-1.6575-.85-.7468-1.9246-1.621h-.1275v.17l.4432.6496 2.3436 3.5214.1214 1.0807-.17.3521-.6071.2125-.6679-.1214-1.3721-1.9246L14.38 17.959l-1.1414-1.9428-.1397.079-.674 7.2552-.3156.3703-.7286.2793-.6071-.4614-.3218-.7468.3218-1.4753.3886-1.9246.3157-1.53.2853-1.9004.17-.6314-.0121-.0425-.1397.0182-1.4328 1.9672-2.1796 2.9446-1.7243 1.8456-.4128.164-.7164-.3704.0667-.6618.4008-.5889 2.386-3.0357 1.4389-1.882.929-1.0868-.0062-.1579h-.0546l-6.3385 4.1164-1.1293.1457-.4857-.4554.0608-.7467.2307-.2429 1.9064-1.3114Z'
    },
    codex: {
      evenodd: true,
      d: 'M8.086.457a6.105 6.105 0 013.046-.415c1.333.153 2.521.72 3.564 1.7a.117.117 0 00.107.029c1.408-.346 2.762-.224 4.061.366l.063.03.154.076c1.357.703 2.33 1.77 2.918 3.198.278.679.418 1.388.421 2.126a5.655 5.655 0 01-.18 1.631.167.167 0 00.04.155 5.982 5.982 0 011.578 2.891c.385 1.901-.01 3.615-1.183 5.14l-.182.22a6.063 6.063 0 01-2.934 1.851.162.162 0 00-.108.102c-.255.736-.511 1.364-.987 1.992-1.199 1.582-2.962 2.462-4.948 2.451-1.583-.008-2.986-.587-4.21-1.736a.145.145 0 00-.14-.032c-.518.167-1.04.191-1.604.185a5.924 5.924 0 01-2.595-.622 6.058 6.058 0 01-2.146-1.781c-.203-.269-.404-.522-.551-.821a7.74 7.74 0 01-.495-1.283 6.11 6.11 0 01-.017-3.064.166.166 0 00.008-.074.115.115 0 00-.037-.064 5.958 5.958 0 01-1.38-2.202 5.196 5.196 0 01-.333-1.589 6.915 6.915 0 01.188-2.132c.45-1.484 1.309-2.648 2.577-3.493.282-.188.55-.334.802-.438.286-.12.573-.22.861-.304a.129.129 0 00.087-.087A6.016 6.016 0 015.635 2.31C6.315 1.464 7.132.846 8.086.457zm-.804 7.85a.848.848 0 00-1.473.842l1.694 2.965-1.688 2.848a.849.849 0 001.46.864l1.94-3.272a.849.849 0 00.007-.854l-1.94-3.393zm5.446 6.24a.849.849 0 000 1.695h4.848a.849.849 0 000-1.696h-4.848z'
    },
    opencode: { evenodd: true, d: 'M16 6H8v12h8V6zm4 16H4V2h16v20z' }
  };

  const RECENT_LIMIT = 5;
  const SWITCHER_LIMIT = 12;

  let agents = $state<AgentGlobal[]>([]);
  let recent = $state<ProjectRecord[]>([]);
  let opening = $state(false);
  let switcher = $state<{ top: number; left: number } | null>(null);
  let switcherEl = $state<HTMLDivElement>();

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

  const others = $derived(recent.filter((p) => p.path !== app.projectRoot));
  const otherRecent = $derived(others.slice(0, app.loaded ? RECENT_LIMIT - 1 : RECENT_LIMIT));
  const switcherRecent = $derived(others.slice(0, SWITCHER_LIMIT));

  function parentDir(path: string): string {
    return path.replace(/[\\/]+$/, '').replace(/[\\/][^\\/]*$/, '');
  }

  function toggleSwitcher(e: MouseEvent) {
    if (switcher) {
      switcher = null;
      return;
    }
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    switcher = { top: Math.max(8, Math.min(r.top - 8, window.innerHeight - 428)), left: r.right + 10 };
  }

  $effect(() => {
    if (!switcher) return;
    switcherEl?.querySelector<HTMLElement>('.sw-item')?.focus();
    const onDown = (e: PointerEvent) => {
      const t = e.target as Element;
      if (switcherEl?.contains(t) || t.closest?.('[data-switcher-toggle]')) return;
      switcher = null;
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') switcher = null;
    };
    document.addEventListener('pointerdown', onDown, true);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('pointerdown', onDown, true);
      document.removeEventListener('keydown', onKey);
    };
  });

  $effect(() => {
    if (!prefs.sidebarCollapsed) switcher = null;
  });

  async function switchTo(path: string | null) {
    switcher = null;
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
    switcher = null;
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
          agent: a.id,
          hint: a.exists ? undefined : '설정 파일 없음'
        }))
      : [{ href: '/user', label: 'Claude Code', agent: 'claude-code' }]
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

{#snippet agentLogo(id: string, label: string)}
  {@const logo = AGENT_LOGOS[id]}
  {#if logo}
    <svg class="logo" viewBox="0 0 24 24" width="17" height="17" aria-hidden="true">
      <path d={logo.d} fill={logo.color ?? 'currentColor'} fill-rule={logo.evenodd ? 'evenodd' : undefined} />
    </svg>
  {:else}
    <span class="mono" aria-hidden="true">{label.slice(0, 2).toUpperCase()}</span>
  {/if}
{/snippet}

{#snippet mark(name: string, key: string, size = 22)}
  {@const m = projectMark(name, key)}
  <span class="mark" style:--mark-hue={m.hue} style:--mark-size="{size}px" aria-hidden="true">{m.text}</span>
{/snippet}

{#snippet link(item: NavItem)}
  <a
    href={item.href}
    class="nav-item"
    aria-current={isActive(item.href) ? 'page' : undefined}
    use:tooltip={prefs.sidebarCollapsed ? item.label : undefined}
  >
    {#if item.agent}
      {@render agentLogo(item.agent, item.label)}
    {:else if item.icon}
      {@render icon(item.icon)}
    {/if}
    <span class="label truncate">{item.label}</span>
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
      <a
        href="/project"
        class="nav-item"
        aria-current={isActive('/project') ? 'page' : undefined}
        use:tooltip={prefs.sidebarCollapsed ? `${app.projectName}\n${app.projectRoot}` : undefined}
      >
        {@render mark(app.projectName, app.projectRoot)}
        <span class="label truncate">{app.projectName}</span>
        {#if app.dirty}<span class="badge" aria-label="저장하지 않은 변경">●</span>{/if}
      </a>
    {/if}
    {#if prefs.sidebarCollapsed}
      <button
        class="nav-item subtle"
        data-switcher-toggle
        onclick={toggleSwitcher}
        aria-haspopup="menu"
        aria-expanded={switcher !== null}
        aria-label="프로젝트 전환"
        use:tooltip={switcher ? undefined : '프로젝트 전환'}
      >
        {@render icon('switch')}
      </button>
    {:else}
      {#each otherRecent as p (p.id)}
        <button class="nav-item" onclick={() => switchTo(p.path)} disabled={opening} title={p.path}>
          {@render mark(p.name, p.path)}
          <span class="label truncate">{p.name}</span>
        </button>
      {/each}
      <button class="nav-item subtle" onclick={openFolder} disabled={opening}>
        {@render icon('plus')}
        <span class="label">{opening ? '여는 중…' : '폴더 열기…'}</span>
      </button>
    {/if}

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

{#if switcher}
  <div
    class="switcher"
    role="menu"
    aria-label="프로젝트 전환"
    bind:this={switcherEl}
    style:top="{switcher.top}px"
    style:left="{switcher.left}px"
  >
    <div class="sw-title">프로젝트 전환</div>
    {#if app.loaded}
      <a href="/project" class="sw-item current" role="menuitem" onclick={() => (switcher = null)}>
        {@render mark(app.projectName, app.projectRoot, 28)}
        <span class="sw-text">
          <span class="sw-name truncate">{app.projectName}</span>
          <span class="sw-path truncate">{parentDir(app.projectRoot)}</span>
        </span>
        <span class="sw-tag">현재</span>
      </a>
    {/if}
    {#each switcherRecent as p (p.id)}
      <button class="sw-item" role="menuitem" onclick={() => switchTo(p.path)} disabled={opening} title={p.path}>
        {@render mark(p.name, p.path, 28)}
        <span class="sw-text">
          <span class="sw-name truncate">{p.name}</span>
          <span class="sw-path truncate">{parentDir(p.path)}</span>
        </span>
      </button>
    {/each}
    {#if !app.loaded && switcherRecent.length === 0}
      <p class="sw-empty">최근 프로젝트가 없습니다.</p>
    {/if}
    <div class="sw-sep"></div>
    <button class="sw-item" role="menuitem" onclick={openFolder} disabled={opening}>
      <span class="sw-icon">{@render icon('plus')}</span>
      <span class="sw-name">{opening ? '여는 중…' : '폴더 열기…'}</span>
    </button>
  </div>
{/if}

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
    background: var(--bg-3);
    color: var(--text-1);
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
  .logo {
    width: 18px;
    padding: 0 2px;
    color: var(--text-1);
  }
  .mono {
    flex-shrink: 0;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border: 1px solid var(--border-strong);
    border-radius: var(--r-xs);
    font-size: 0.6rem;
    font-weight: 650;
  }
  .mark {
    flex-shrink: 0;
    display: grid;
    place-items: center;
    width: var(--mark-size);
    height: var(--mark-size);
    border-radius: calc(var(--mark-size) * 0.27);
    background: oklch(0.56 0.13 var(--mark-hue));
    color: #fff;
    font-size: calc(var(--mark-size) * 0.42);
    font-weight: 650;
    letter-spacing: -0.02em;
    line-height: 1;
  }
  .collapsed .nav-item[aria-current='page'] .mark {
    outline: 2px solid var(--text-1);
    outline-offset: 2px;
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
    right: 0.3rem;
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
  .switcher {
    position: fixed;
    z-index: 60;
    width: 280px;
    max-height: min(420px, calc(100vh - 16px));
    overflow-y: auto;
    box-sizing: border-box;
    padding: 0.35rem;
    background: var(--bg-1);
    border: 1px solid var(--border-strong);
    border-radius: var(--r-md);
    box-shadow: var(--shadow-2);
    animation: ag-fade-in 100ms ease-out;
  }
  .sw-title {
    padding: 0.35rem 0.5rem 0.3rem;
    color: var(--text-3);
    font-size: var(--fs-xs);
    font-weight: 600;
  }
  .sw-item {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    width: 100%;
    min-height: 2.6rem;
    padding: 0.3rem 0.5rem;
    border: 0;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--text-1);
    font: inherit;
    font-size: var(--fs-md);
    text-align: left;
    text-decoration: none;
    cursor: pointer;
    box-sizing: border-box;
  }
  .sw-item:hover,
  .sw-item:focus-visible {
    background: var(--hover);
    outline: none;
  }
  .sw-item:disabled {
    cursor: progress;
    opacity: 0.6;
  }
  .sw-item.current {
    background: var(--bg-3);
  }
  .sw-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .sw-name {
    font-weight: 600;
  }
  .sw-path {
    color: var(--text-3);
    font-size: var(--fs-xs);
    font-family: var(--font-mono);
  }
  .sw-tag {
    color: var(--text-3);
    font-size: var(--fs-xs);
  }
  .sw-icon {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    color: var(--text-2);
  }
  .sw-sep {
    height: 1px;
    margin: 0.3rem 0.2rem;
    background: var(--border);
  }
  .sw-empty {
    margin: 0.2rem 0.5rem 0.4rem;
    color: var(--text-3);
    font-size: var(--fs-sm);
  }
</style>
