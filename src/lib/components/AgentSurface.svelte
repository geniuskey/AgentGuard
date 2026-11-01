<script lang="ts">
  import { onMount } from 'svelte';
  import { inspectAgentSurface, type AgentSurface } from '$lib/ipc';
  import { app } from '$lib/state.svelte';

  let surface = $state<AgentSurface | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(false);

  async function load() {
    loading = true;
    error = null;
    try {
      surface = await inspectAgentSurface(app.projectRoot);
    } catch (e) {
      surface = null;
      error = String(e);
    } finally {
      loading = false;
    }
  }

  onMount(load);
</script>

<div class="surf">
  <div class="head">
    <p class="intro">
      Hooks와 MCP 서버는 경로 권한 규칙 <b>밖에서</b> 동작합니다. 읽기 전용으로 표시하며,
      실제 활성 상태와 핸들러 위험도를 리스크 점수에 반영합니다.
    </p>
    <button class="btn btn-sm refresh" onclick={load} disabled={loading}>
      {loading ? '새로고침 중…' : '새로고침'}
    </button>
  </div>

  {#if error}<p class="callout callout-error err" role="alert">{error}</p>{/if}

  {#if surface}
    <section>
      <h4>
        Hooks
        {#if surface.hooks.length}<span class="cnt warn">{surface.hooks.length}</span>{/if}
      </h4>
      {#if surface.hooks.length}
        <div class="callout callout-warn warnbox">⚠ 훅은 명령·에이전트·HTTP·MCP 도구를 자동 실행할 수 있습니다.</div>
        {#each surface.hooks as h (h.scope + h.event + (h.matcher ?? '') + h.command)}
          <div class="row">
            <span class="scope">{h.scope}</span>
            <span class="event">{h.event}</span>
            {#if h.matcher}<span class="matcher">{h.matcher}</span>{/if}
            <span class="transport">{h.handlerType}</span>
            <span class:risk-high={h.riskLevel === 'high'} class="risk-tag">{h.riskLevel}</span>
            {#if h.usesWeb}<span class="web">웹 접근</span>{/if}
            <code>{h.command}</code>
          </div>
        {/each}
      {:else}
        <p class="muted">설정된 훅 없음</p>
      {/if}
    </section>

    <section>
      <h4>
        MCP 서버
        {#if surface.mcpServers.length}<span class="cnt">{surface.mcpServers.length}</span>{/if}
      </h4>
      {#if surface.mcpServers.length}
        {#if surface.mcpServers.some((s) => s.active && s.usesWeb)}
          <div class="callout callout-warn warnbox">
            ⚠ 일부 MCP 서버는 <b>외부 네트워크와 통신</b>합니다 (예: context7, 원격 서버).
            프롬프트·코드가 밖으로 나갈 수 있으니 신뢰할 수 있는 서버만 사용하세요.
          </div>
        {/if}
        {#each surface.mcpServers as s (s.source + s.name)}
          <div class="row">
            <span class="name">{s.name}</span>
            <span class="transport">{s.transport}</span>
            <span class:inactive={!s.active} class="active-state" title={s.statusReason}>
              {s.active ? '활성' : '비활성'}
            </span>
            {#if s.active && s.usesWeb}<span class="web">웹 접근</span>{/if}
            <code>{s.target}</code>
            <span class="src">{s.source}</span>
          </div>
        {/each}
      {:else}
        <p class="muted">설정된 MCP 서버 없음</p>
      {/if}
    </section>
  {:else if !error}
    <p class="muted">읽는 중…</p>
  {/if}
</div>

<style>
  .surf {
    padding: 0.65rem;
    overflow: auto;
    flex: 1;
  }
  .head {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    margin-bottom: 0.7rem;
  }
  .intro {
    color: var(--text-3);
    font-size: var(--fs-sm);
    margin: 0;
    flex: 1;
    min-width: 0;
  }
  .intro b {
    color: var(--text-2);
  }
  .refresh {
    flex-shrink: 0;
  }
  .err {
    margin-bottom: 0.7rem;
    word-break: break-all;
  }
  section {
    margin-bottom: 1rem;
  }
  h4 {
    color: var(--text-2);
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    margin: 0 0 0.4rem;
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }
  .cnt {
    background: var(--accent-soft);
    color: var(--accent-text);
    border-radius: 999px;
    padding: 0 0.4rem;
    font-size: 0.64rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .cnt.warn {
    background: var(--ask-soft);
    color: var(--ask);
  }
  .warnbox {
    font-size: var(--fs-sm);
    margin-bottom: 0.45rem;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    padding: 0.26rem 0.35rem;
    border-radius: var(--r-sm);
    font-size: 0.76rem;
    min-width: 0;
  }
  .row:hover {
    background: var(--hover);
  }
  .scope,
  .transport {
    background: var(--bg-2);
    border: 1px solid var(--border);
    color: var(--text-2);
    border-radius: 999px;
    padding: 0.02rem 0.45rem;
    font-size: 0.64rem;
    text-transform: capitalize;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .event {
    color: var(--accent-text);
    font-size: 0.7rem;
    font-weight: 600;
    flex-shrink: 0;
  }
  .matcher {
    color: var(--text-3);
    font-size: var(--fs-xs);
    font-family: var(--font-mono);
    flex-shrink: 0;
  }
  .name {
    color: var(--text-1);
    font-weight: 600;
    font-size: 0.76rem;
    flex-shrink: 0;
  }
  .web {
    background: var(--ask-soft);
    border: 1px solid var(--ask-line);
    color: var(--ask);
    border-radius: 999px;
    padding: 0.02rem 0.45rem;
    font-size: 0.64rem;
    font-weight: 700;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .risk-tag,
  .active-state {
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    color: var(--ask);
    font-size: 0.64rem;
    padding: 0.02rem 0.4rem;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .risk-tag.risk-high {
    color: var(--deny);
    border-color: var(--deny-line);
  }
  .active-state {
    color: var(--allow);
  }
  .active-state.inactive {
    color: var(--text-3);
  }
  code {
    font-family: var(--font-mono);
    font-size: 0.7rem;
    color: var(--text-2);
    background: var(--bg-1);
    border: 1px solid var(--border);
    padding: 0.1rem 0.4rem;
    border-radius: var(--r-sm);
    word-break: break-all;
    min-width: 0;
  }
  .src {
    margin-left: auto;
    color: var(--text-3);
    font-size: 0.64rem;
    flex-shrink: 0;
  }
</style>
