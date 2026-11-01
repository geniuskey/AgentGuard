<script lang="ts" module>
  export interface McpAddItem {
    name: string;
    config: Record<string, unknown>;
  }
</script>

<script lang="ts">
  import { mcpParseImport, type McpTarget } from '$lib/ipc';
  import { MCP_CATALOG, applyInputs, windowsWrap, type McpCatalogEntry } from '$lib/mcp-catalog';
  import { modalFocus } from '$lib/modal';
  import { app } from '$lib/state.svelte';

  let {
    onclose,
    onsubmit
  }: { onclose: () => void; onsubmit: (target: McpTarget, items: McpAddItem[]) => void } = $props();

  const NAME_RE = /^[A-Za-z0-9_.-]+$/;
  const isWindows = typeof navigator !== 'undefined' && navigator.userAgent.includes('Windows');
  const hasProject = $derived(!!app.projectRoot);

  const TARGETS: { value: McpTarget; label: string; hint?: string; needsProject: boolean }[] = [
    {
      value: 'project',
      label: '이 프로젝트 (팀 공유 · .mcp.json)',
      hint: '팀원이 처음 열 때 승인 여부를 묻습니다',
      needsProject: true
    },
    { value: 'local', label: '이 프로젝트 (나만 · ~/.claude.json)', needsProject: true },
    { value: 'user', label: '내 컴퓨터 전체 (~/.claude.json)', needsProject: false }
  ];

  let tab = $state<'catalog' | 'paste'>('catalog');
  let target = $state<McpTarget>(app.projectRoot ? 'project' : 'user');
  let inputValues = $state<Record<string, Record<string, string>>>({});

  let pasteText = $state('');
  let parsing = $state(false);
  let parseError = $state<string | null>(null);
  let parsed = $state<
    { name: string; config: Record<string, unknown>; warnings: string[]; selected: boolean }[]
  >([]);

  function setTarget(t: McpTarget | null) {
    if (!t) return;
    if (!hasProject && t !== 'user') return;
    target = t;
  }

  function inputsFilled(e: McpCatalogEntry): boolean {
    return (e.inputs ?? []).every((i) => inputValues[e.id]?.[i.key]?.trim());
  }

  function wrapParsed(p: (typeof parsed)[number]) {
    p.config = windowsWrap(p.config);
    p.warnings = p.warnings.filter((w) => !w.includes('cmd /c'));
  }

  function addCatalog(e: McpCatalogEntry) {
    const config = applyInputs(e, inputValues[e.id] ?? {});
    onsubmit(target, [{ name: e.name, config: isWindows ? windowsWrap(config) : config }]);
  }

  async function parse() {
    parsing = true;
    parseError = null;
    parsed = [];
    try {
      const result = await mcpParseImport(pasteText);
      parsed = result.map((s) => ({ ...s, selected: true }));
      if (!result.length) parseError = '추가할 MCP 서버를 찾지 못했습니다.';
      setTarget(result.find((s) => s.scopeHint)?.scopeHint ?? null);
    } catch (e) {
      parseError = String(e);
    } finally {
      parsing = false;
    }
  }

  const chosen = $derived(parsed.filter((p) => p.selected));
  const namesValid = $derived(chosen.every((p) => NAME_RE.test(p.name.trim())));
  const namesUnique = $derived(new Set(chosen.map((p) => p.name.trim())).size === chosen.length);

  function addParsed() {
    onsubmit(
      target,
      chosen.map((p) => ({ name: p.name.trim(), config: $state.snapshot(p.config) }))
    );
  }

  const PLACEHOLDER = `claude mcp add 명령이나 JSON을 붙여넣으세요. 예:

claude mcp add --transport http sentry https://mcp.sentry.dev/mcp

{
  "mcpServers": {
    "memory": { "command": "npx", "args": ["-y", "@modelcontextprotocol/server-memory"] }
  }
}`;
</script>

<div class="modal-bg" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) onclose(); }}>
  <div
    class="modal"
    role="dialog"
    aria-modal="true"
    aria-labelledby="mcp-add-title"
    tabindex="-1"
    use:modalFocus={onclose}
  >
    <h3 id="mcp-add-title">MCP 서버 추가</h3>

    <fieldset class="targets">
      <legend>어디에 추가할까요?</legend>
      {#each TARGETS as t (t.value)}
        {@const disabled = t.needsProject && !hasProject}
        <label class:disabled>
          <input type="radio" name="mcp-target" value={t.value} bind:group={target} {disabled} />
          <span>
            {t.label}
            {#if t.hint}<span class="hint">{t.hint}</span>{/if}
            {#if disabled}<span class="hint">프로젝트를 먼저 열어야 합니다</span>{/if}
          </span>
        </label>
      {/each}
    </fieldset>

    <div class="segmented tabs" role="tablist" aria-label="추가 방법">
      <button role="tab" aria-selected={tab === 'catalog'} data-modal-initial onclick={() => (tab = 'catalog')}>
        추천 목록
      </button>
      <button role="tab" aria-selected={tab === 'paste'} onclick={() => (tab = 'paste')}>붙여넣기</button>
    </div>

    {#if tab === 'catalog'}
      <div class="cards">
        {#each MCP_CATALOG as e (e.id)}
          <div class="card">
            <div class="card-head">
              <span class="title">{e.title}</span>
              {#if e.usesWeb}<span class="chip chip-ask">웹 접근</span>{/if}
            </div>
            <p class="desc">{e.description}</p>
            {#each e.inputs ?? [] as i (i.key)}
              <label class="field">
                <span>{i.label}</span>
                <input
                  class="input mono"
                  placeholder={i.placeholder}
                  spellcheck="false"
                  value={inputValues[e.id]?.[i.key] ?? ''}
                  oninput={(ev) =>
                    (inputValues[e.id] = {
                      ...inputValues[e.id],
                      [i.key]: (ev.currentTarget as HTMLInputElement).value
                    })}
                />
              </label>
            {/each}
            {#if e.note}<p class="note">{e.note}</p>{/if}
            <div class="card-foot">
              <code>{e.name}</code>
              <button class="btn btn-sm btn-primary" disabled={!inputsFilled(e)} onclick={() => addCatalog(e)}>
                추가
              </button>
            </div>
          </div>
        {/each}
      </div>
    {:else}
      <textarea
        class="input mono paste"
        rows="9"
        spellcheck="false"
        aria-label="MCP 설정 붙여넣기"
        placeholder={PLACEHOLDER}
        bind:value={pasteText}
      ></textarea>
      <div class="paste-actions">
        <button class="btn btn-sm" onclick={parse} disabled={parsing || !pasteText.trim()}>
          {parsing ? '해석 중…' : '해석'}
        </button>
      </div>
      {#if parseError}<p class="callout callout-error" role="alert">{parseError}</p>{/if}
      {#each parsed as p, i (i)}
        <div class="parsed">
          <div class="parsed-head">
            <input type="checkbox" bind:checked={p.selected} aria-label="추가할 서버 선택" />
            <input class="input mono name" spellcheck="false" aria-label="서버 이름" bind:value={p.name} />
            {#if !NAME_RE.test(p.name.trim())}
              <span class="bad">영문·숫자·_ . - 만 쓸 수 있습니다</span>
            {/if}
          </div>
          <pre class="cfg">{JSON.stringify(p.config, null, 2)}</pre>
          {#each p.warnings as w (w)}<p class="callout callout-warn warn">{w}</p>{/each}
          {#if isWindows && p.config.command === 'npx'}
            <button class="btn btn-sm wrap" onclick={() => wrapParsed(p)}>cmd /c 로 감싸기</button>
          {/if}
        </div>
      {/each}
    {/if}

    <div class="modal-actions">
      <button onclick={onclose}>닫기</button>
      {#if tab === 'paste' && parsed.length}
        <button class="primary" onclick={addParsed} disabled={!chosen.length || !namesValid || !namesUnique}>
          선택한 서버 추가 ({chosen.length})
        </button>
      {/if}
    </div>
  </div>
</div>

<style>
  .targets {
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    padding: 0.5rem 0.75rem;
    margin: 0.6rem 0 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  legend {
    font-size: var(--fs-sm);
    color: var(--text-2);
    padding: 0 0.3rem;
  }
  .targets label {
    display: flex;
    align-items: flex-start;
    gap: 0.45rem;
    font-size: var(--fs-md);
    cursor: pointer;
  }
  .targets label.disabled {
    color: var(--text-3);
    cursor: default;
  }
  .hint {
    display: block;
    color: var(--text-3);
    font-size: var(--fs-xs);
  }
  .tabs {
    margin-bottom: 0.6rem;
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(16rem, 1fr));
    gap: 0.5rem;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    padding: 0.6rem 0.7rem;
  }
  .card-head {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .title {
    font-weight: 600;
    font-size: var(--fs-md);
    flex: 1;
  }
  .desc,
  .note {
    margin: 0;
    font-size: var(--fs-xs);
    color: var(--text-2);
    line-height: 1.45;
  }
  .note {
    color: var(--text-3);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    font-size: var(--fs-xs);
    color: var(--text-2);
  }
  .card-foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: auto;
    padding-top: 0.2rem;
  }
  .card-foot code {
    font-size: var(--fs-xs);
    color: var(--text-3);
  }
  .input {
    font-size: var(--fs-sm);
    padding: 0.32rem 0.5rem;
  }
  .mono {
    font-family: var(--font-mono);
  }
  .paste {
    width: 100%;
    box-sizing: border-box;
    resize: vertical;
  }
  .paste-actions {
    display: flex;
    justify-content: flex-end;
    margin: 0.4rem 0;
  }
  .parsed {
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    padding: 0.5rem 0.65rem;
    margin-top: 0.5rem;
  }
  .parsed-head {
    display: flex;
    align-items: center;
    gap: 0.45rem;
  }
  .name {
    width: 14rem;
  }
  .bad {
    color: var(--deny);
    font-size: var(--fs-xs);
  }
  .cfg {
    margin: 0.4rem 0 0;
    font-size: 0.7rem;
    background: var(--bg-0);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    padding: 0.4rem 0.55rem;
    overflow: auto;
    max-height: 12rem;
  }
  .warn {
    font-size: var(--fs-xs);
    margin: 0.35rem 0 0;
  }
  .wrap {
    margin-top: 0.35rem;
  }
</style>
