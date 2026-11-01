<script lang="ts">
  import { toSettingsPreview, type Permissions, type ScopeName } from '$lib/ipc';
  import { app } from '$lib/state.svelte';
  import PolicySimulator from './PolicySimulator.svelte';
  import AgentSurface from './AgentSurface.svelte';

  type Tab = 'allowed' | 'denied' | 'ask' | 'conflicts' | 'byscope' | 'raw' | 'simulate' | 'surface';
  let tab = $state<Tab>('allowed');
  const uid = $props.id();

  const allowed = $derived(app.effective.filter((e) => e.effective === 'allow'));
  const denied = $derived(app.effective.filter((e) => e.effective === 'deny'));
  const asked = $derived(app.effective.filter((e) => e.effective === 'ask'));
  const conflicts = $derived(app.effective.filter((e) => e.conflict));
  const unresolved = $derived(app.effective.filter((e) => e.effective === null));
  const byScope = $derived(
    (['managed', 'user', 'project', 'local'] as const).map((s) => ({
      scope: s,
      rules: [...app.scoped[s].rules, ...(app.scoped[s].wholeToolRules ?? [])]
    }))
  );

  const scopeLabel: Record<ScopeName | 'managed', string> = {
    managed: '관리자',
    user: '사용자',
    project: '프로젝트',
    local: '로컬'
  };

  let raw = $state<Permissions | null>(null);
  // Recompute raw rules for the active scope whenever we switch to the Raw tab.
  $effect(() => {
    if (tab === 'raw') {
      toSettingsPreview(app.scoped[app.activeScope].rules).then((p) => (raw = p));
    }
  });

  const tabs = $derived<{ id: Tab; label: string; count?: number }[]>([
    { id: 'allowed', label: 'Allow', count: allowed.length },
    { id: 'denied', label: 'Deny', count: denied.length },
    { id: 'ask', label: 'Ask', count: asked.length },
    { id: 'conflicts', label: '충돌', count: conflicts.length },
    { id: 'byscope', label: '범위별' },
    { id: 'raw', label: '원본 규칙' },
    { id: 'simulate', label: '시뮬레이터' },
    { id: 'surface', label: 'MCP/Hooks' }
  ]);

  function onTabKey(e: KeyboardEvent) {
    if (e.key !== 'ArrowRight' && e.key !== 'ArrowLeft') return;
    const i = tabs.findIndex((t) => t.id === tab);
    const next = tabs[(i + (e.key === 'ArrowRight' ? 1 : tabs.length - 1)) % tabs.length];
    tab = next.id;
    document.getElementById(`${uid}-tab-${next.id}`)?.focus();
    e.preventDefault();
  }
</script>

<div class="panel">
  <div class="tabs" role="tablist" aria-label="유효 정책 보기" tabindex="-1" onkeydown={onTabKey}>
    {#each tabs as t (t.id)}
      <button
        role="tab"
        id="{uid}-tab-{t.id}"
        aria-selected={tab === t.id}
        aria-controls="{uid}-panel"
        tabindex={tab === t.id ? 0 : -1}
        onclick={() => (tab = t.id)}
      >
        {t.label}
        {#if t.count}<span class="cnt" class:cnt-deny={t.id === 'conflicts'}>{t.count}</span>{/if}
      </button>
    {/each}
  </div>

  <div class="tabpanel" role="tabpanel" id="{uid}-panel" aria-labelledby="{uid}-tab-{tab}">
    {#if tab === 'simulate'}
      <PolicySimulator />
    {:else if tab === 'surface'}
      <AgentSurface />
    {:else}
      <div class="body">
        <p class="muted">도구별 명시 규칙의 근사 비교입니다. 실제 승인 결과는 실행 모드·신뢰·CLI 설정·훅 등에 따라 다릅니다.</p>
        {#if unresolved.length}
          <details class="unresolved">
            <summary>판정 불가 · 일치 규칙 없음 {unresolved.length}건</summary>
            {#each unresolved as e (e.path + e.tool)}
              <p class="muted ur">
                <span class="p">{e.tool} · {e.path}</span>: {e.incomplete
                  ? '지원하지 않는 패턴 또는 컨텍스트 부족으로 판정하지 않음'
                  : '일치한 명시 규칙 없음'}
              </p>
            {/each}
          </details>
        {/if}
        {#if tab === 'allowed'}
          {#each allowed as e (e.path + e.tool)}
            <div class="row"><span class="pd pd-allow" aria-hidden="true"></span><span class="p">{e.tool} · {e.path}</span></div>
          {:else}
            <p class="muted">없음</p>
          {/each}
        {:else if tab === 'denied'}
          {#each denied as e (e.path + e.tool)}
            <div class="row">
              <span class="pd pd-deny" aria-hidden="true"></span><span class="p">{e.tool} · {e.path}</span>
              <span class="src">{e.sourceScope ? scopeLabel[e.sourceScope] : ''}</span>
            </div>
          {:else}
            <p class="muted">없음</p>
          {/each}
        {:else if tab === 'ask'}
          {#each asked as e (e.path + e.tool)}
            <div class="row"><span class="pd pd-ask" aria-hidden="true"></span><span class="p">{e.tool} · {e.path}</span></div>
          {:else}
            <p class="muted">없음</p>
          {/each}
        {:else if tab === 'conflicts'}
          {#each conflicts as e (e.path + e.tool)}
            <div class="row conflict">
              <span class="pd pd-conflict" aria-hidden="true"></span>
              <span class="p">{e.tool} · {e.path} → {e.effective?.toUpperCase() ?? '판정 불가'}</span>
              <span class="src">deny &gt; ask &gt; allow</span>
            </div>
          {:else}
            <p class="muted">충돌 없음</p>
          {/each}
        {:else if tab === 'byscope'}
          {#if app.scoped.managed.enforceManagedOnly}
            <p class="callout callout-warn managed-only">
              관리자가 <code>allowManagedPermissionRulesOnly</code>를 활성화했습니다. 아래 사용자/프로젝트/로컬
              규칙은 파일에 남아 있지만 유효 정책 계산에서는 제외됩니다.
            </p>
          {/if}
          {#each byScope as g (g.scope)}
            <div class="scope-group">
              <b>{scopeLabel[g.scope]}</b>
              {#each g.rules as r (r.path + r.policy + (r.tools ?? []).join(','))}
                <div class="row">
                  <span class="pletter l-{r.policy}">{r.policy[0].toUpperCase()}</span>
                  <span class="p">{r.path} {r.reason?.startsWith('전체 도구 규칙:') ? r.reason : ''}</span>
                </div>
              {:else}
                <p class="muted">규칙 없음</p>
              {/each}
            </div>
          {/each}
        {:else if tab === 'raw'}
          {#if raw}
            <div class="raw-group"><b class="rk-allow">allow</b>{#each raw.allow as r}<code>{r}</code>{/each}</div>
            <div class="raw-group"><b class="rk-ask">ask</b>{#each raw.ask as r}<code>{r}</code>{/each}</div>
            <div class="raw-group"><b class="rk-deny">deny</b>{#each raw.deny as r}<code>{r}</code>{/each}</div>
            <p class="note">= {scopeLabel[app.activeScope]} 범위에 저장될 실제 규칙</p>
          {:else}
            <p class="muted">계산 중…</p>
          {/if}
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
  }
  .tabs {
    display: flex;
    flex-shrink: 0;
    border-bottom: 1px solid var(--border);
    flex-wrap: wrap;
    padding: 0 0.25rem;
  }
  .tabs button {
    display: inline-flex;
    align-items: center;
    flex-shrink: 0;
    padding: 0.55rem 0.7rem;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--text-2);
    cursor: pointer;
    font-size: var(--fs-md);
    white-space: nowrap;
    transition: color var(--t-fast), border-color var(--t-fast);
  }
  .tabs button:hover {
    color: var(--text-1);
  }
  .tabs button[aria-selected='true'] {
    color: var(--accent-text);
    border-bottom-color: var(--accent);
  }
  .tabs button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
  .cnt {
    background: var(--bg-2);
    color: var(--text-2);
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    padding: 0 0.38rem;
    font-size: 0.64rem;
    font-weight: 700;
    margin-left: 0.3rem;
    font-variant-numeric: tabular-nums;
  }
  .cnt-deny {
    background: var(--deny-soft);
    color: var(--deny);
    border-color: var(--deny-line);
  }
  .tabpanel {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }
  .body {
    overflow: auto;
    padding: 0.55rem;
    flex: 1;
  }
  .unresolved {
    margin: 0 0 0.6rem;
    padding: 0.35rem 0.55rem;
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
  }
  .unresolved summary {
    cursor: pointer;
    color: var(--text-2);
    font-size: var(--fs-sm);
  }
  .ur {
    margin: 0.3rem 0 0;
    font-size: var(--fs-sm);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: var(--fs-md);
    font-family: var(--font-mono);
    padding: 0.22rem 0.3rem;
    border-radius: var(--r-sm);
    min-width: 0;
  }
  .row:hover {
    background: var(--hover);
  }
  .p {
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .pd {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    flex-shrink: 0;
  }
  .pd-allow {
    background: var(--allow);
  }
  .pd-deny {
    background: var(--deny);
  }
  .pd-ask {
    background: var(--ask);
  }
  .pd-conflict {
    background: var(--deny);
    outline: 2px solid var(--deny-line);
  }
  .row .src {
    margin-left: auto;
    color: var(--text-3);
    font-size: var(--fs-xs);
    font-family: var(--font-sans);
    flex-shrink: 0;
    white-space: nowrap;
  }
  .row.conflict {
    color: var(--deny);
  }
  .managed-only {
    margin: 0 0 0.75rem;
  }
  .pletter {
    display: inline-grid;
    place-items: center;
    width: 1.15rem;
    height: 1.15rem;
    border-radius: var(--r-sm);
    font-size: 0.62rem;
    font-weight: 700;
    font-family: var(--font-sans);
    flex-shrink: 0;
  }
  .l-allow {
    background: var(--allow-soft);
    color: var(--allow);
  }
  .l-deny {
    background: var(--deny-soft);
    color: var(--deny);
  }
  .l-ask {
    background: var(--ask-soft);
    color: var(--ask);
  }
  .raw-group {
    margin-bottom: 0.7rem;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .raw-group b {
    font-size: var(--fs-xs);
    font-weight: 700;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    width: fit-content;
    padding: 0.08rem 0.45rem;
    border-radius: var(--r-sm);
    margin-bottom: 0.15rem;
  }
  .rk-allow {
    background: var(--allow-soft);
    color: var(--allow);
  }
  .rk-ask {
    background: var(--ask-soft);
    color: var(--ask);
  }
  .rk-deny {
    background: var(--deny-soft);
    color: var(--deny);
  }
  code {
    background: var(--bg-1);
    border: 1px solid var(--border);
    padding: 0.18rem 0.45rem;
    border-radius: var(--r-sm);
    font-size: var(--fs-sm);
    overflow-wrap: anywhere;
  }
  .note {
    color: var(--text-3);
    font-size: var(--fs-xs);
  }
  .scope-group {
    margin-bottom: 0.8rem;
  }
  .scope-group b {
    display: block;
    color: var(--text-2);
    font-size: var(--fs-xs);
    margin-bottom: 0.2rem;
  }
</style>
