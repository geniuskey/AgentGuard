<script lang="ts">
  import { onMount } from 'svelte';
  import {
    inTauri,
    inspectAgentSurface,
    configGet,
    configSetValue,
    mcpPreview,
    mcpSave,
    readRawSettings,
    saveRawSettings,
    type AgentSurface,
    type DiffView,
    type McpServer,
    type McpTarget
  } from '$lib/ipc';
  import { app } from '$lib/state.svelte';
  import { modalFocus } from '$lib/modal';
  import { toast } from '$lib/toast.svelte';
  import { tooltip } from '$lib/tooltip';
  import DiffViewer from './DiffViewer.svelte';
  import McpAddDialog, { type McpAddItem } from './McpAddDialog.svelte';

  interface McpChange {
    target: McpTarget;
    name: string;
    config: Record<string, unknown> | null;
  }

  let surface = $state<AgentSurface | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(false);
  let adding = $state(false);
  let queue: McpChange[] = [];
  let pending = $state<
    (McpChange & { diff: DiffView; overwrite: boolean; approval?: { enable: boolean; text: string } }) | null
  >(null);
  let saving = $state(false);
  let savedAny = false;

  const editable = inTauri();

  function targetOf(s: McpServer): McpTarget | null {
    for (const t of ['project', 'local', 'user'] as const) if (s.source.startsWith(t)) return t;
    return null;
  }

  function exists(target: McpTarget, name: string): boolean {
    return !!surface?.mcpServers.some((s) => s.name === name && targetOf(s) === target);
  }

  async function next() {
    const change = queue.shift();
    if (!change) {
      pending = null;
      if (savedAny) await load();
      savedAny = false;
      return;
    }
    try {
      const diff = await mcpPreview(app.projectRoot, change.target, change.name, change.config);
      pending = { ...change, diff, overwrite: change.config !== null && exists(change.target, change.name) };
    } catch (e) {
      toast(`${change.name}: ${String(e)}`, 'error');
      await next();
    }
  }

  function start(changes: McpChange[]) {
    queue = changes;
    savedAny = false;
    next();
  }

  function onAdd(target: McpTarget, items: McpAddItem[]) {
    adding = false;
    start(items.map((i) => ({ target, name: i.name, config: i.config })));
  }

  function remove(s: McpServer) {
    const target = targetOf(s);
    if (target) start([{ target, name: s.name, config: null }]);
  }

  async function setApproval(s: McpServer, enable: boolean) {
    try {
      const before = await readRawSettings(app.projectRoot, 'local');
      const tree = await configGet(before, 'json');
      const list = (k: string) =>
        (Array.isArray(tree[k]) ? (tree[k] as unknown[]) : []).filter(
          (n): n is string => typeof n === 'string' && n !== s.name
        );
      const enabled = list('enabledMcpjsonServers');
      const disabled = list('disabledMcpjsonServers');
      (enable ? enabled : disabled).push(s.name);
      let text = await configSetValue(before, 'json', 'enabledMcpjsonServers', enabled.length ? enabled : null);
      text = await configSetValue(text, 'json', 'disabledMcpjsonServers', disabled.length ? disabled : null);
      queue = [];
      pending = {
        target: 'project',
        name: s.name,
        config: {},
        overwrite: false,
        approval: { enable, text },
        diff: { path: '.claude/settings.local.json', before, after: text, changed: before !== text }
      };
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  async function confirmApproval(name: string, enable: boolean, text: string) {
    const res = await saveRawSettings({
      projectRoot: app.projectRoot,
      projectId: app.projectId,
      scope: 'local',
      text,
      projectName: app.projectName
    });
    toast(`저장했습니다 → ${res.written}${res.backup ? ' (백업 생성됨)' : ''}`);
    pending = null;
    await load();
    const now = surface?.mcpServers.find((s) => s.name === name && targetOf(s) === 'project');
    if (now && now.active !== enable) {
      toast(`${name}: 다른 설정 파일의 값이 우선해 상태가 바뀌지 않았습니다 (${now.statusReason})`, 'error');
    }
  }

  async function confirm() {
    if (!pending) return;
    saving = true;
    try {
      if (pending.approval) {
        await confirmApproval(pending.name, pending.approval.enable, pending.approval.text);
        return;
      }
      const res = await mcpSave({
        projectRoot: app.projectRoot,
        projectId: app.projectId,
        projectName: app.projectName,
        target: pending.target,
        name: pending.name,
        config: pending.config
      });
      savedAny = true;
      toast(`저장했습니다 → ${res.written}${res.backup ? ' (백업 생성됨)' : ''}`);
      await next();
    } catch (e) {
      toast(String(e), 'error');
    } finally {
      saving = false;
    }
  }

  function skip() {
    if (!saving) next();
  }

  const pendingTitle = $derived(
    !pending
      ? ''
      : pending.approval
        ? `${pending.name} ${pending.approval.enable ? '켜기' : '끄기'} — 변경 내용 확인`
        : pending.config === null
        ? `${pending.name} 삭제 — 변경 내용 확인`
        : `${pending.name} 추가 — 변경 내용 확인`
  );

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
      Hooks와 MCP 서버는 경로 권한 규칙 <b>밖에서</b> 동작하며, 활성 상태와 위험도를 리스크 점수에
      반영합니다. Hooks는 읽기 전용이고, MCP 서버는 여기서 추가·삭제할 수 있습니다 — 저장 전 변경
      내용(Diff)을 보여주고 자동으로 백업합니다.
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
        <span class="h-action" use:tooltip={editable ? undefined : '데스크톱 앱에서만 편집할 수 있습니다'}>
          <button class="btn btn-sm" disabled={!editable} onclick={() => (adding = true)}>MCP 서버 추가</button>
        </span>
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
            {#if editable && targetOf(s) === 'project' && app.projectRoot}
              <button class="btn btn-sm btn-ghost" onclick={() => setApproval(s, !s.active)}>
                {s.active ? '끄기' : '켜기'}
              </button>
            {/if}
            {#if editable && targetOf(s)}
              <button class="btn btn-sm btn-ghost del" aria-label={`${s.name} 삭제`} onclick={() => remove(s)}>
                삭제
              </button>
            {/if}
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

{#if adding}
  <McpAddDialog onclose={() => (adding = false)} onsubmit={onAdd} />
{/if}

{#if pending}
  <div class="modal-bg" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) skip(); }}>
    <div
      class="modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="mcp-change-title"
      tabindex="-1"
      use:modalFocus={skip}
    >
      <h3 id="mcp-change-title">{pendingTitle}</h3>
      <p class="fp">{pending.diff.path}</p>
      {#if pending.overwrite}
        <p class="callout callout-warn note">같은 이름의 서버가 이미 있어 기존 설정을 바꿉니다.</p>
      {/if}
      {#if pending.approval}
        <p class="callout callout-info note">
          이 프로젝트의 내 설정(settings.local.json)에만 기록되어 팀원에게는 영향이 없습니다.
        </p>
      {/if}
      {#if pending.target !== 'project'}
        <p class="callout callout-info note">
          Claude Code가 실행 중이면 ~/.claude.json을 덮어쓸 수 있으니 저장 후 Claude Code를 다시 시작하세요.
        </p>
      {/if}
      {#if pending.diff.changed}
        <DiffViewer diff={pending.diff} />
      {:else}
        <p class="nochg">변경 사항이 없습니다.</p>
      {/if}
      <div class="modal-actions">
        <button data-modal-initial onclick={skip}>취소</button>
        <button class="primary" onclick={confirm} disabled={saving || !pending.diff.changed}>
          {saving ? '저장 중…' : pending.approval ? '백업 후 저장' : pending.config === null ? '백업 후 삭제' : pending.overwrite ? '덮어쓰기' : '백업 후 저장'}
        </button>
      </div>
    </div>
  </div>
{/if}

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
  .h-action {
    margin-left: auto;
    text-transform: none;
    letter-spacing: normal;
  }
  .del {
    flex-shrink: 0;
    color: var(--deny);
  }
  .fp {
    color: var(--text-3);
    font-size: 0.75rem;
    font-family: var(--font-mono);
    margin: 0 0 0.6rem;
    word-break: break-all;
  }
  .note {
    font-size: var(--fs-sm);
    margin: 0 0 0.6rem;
  }
  .nochg {
    color: var(--text-2);
  }
</style>
