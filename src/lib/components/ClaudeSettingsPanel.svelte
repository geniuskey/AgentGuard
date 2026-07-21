<script lang="ts">
  // Structured editor for Claude Code settings beyond the permissions block
  // (~/.claude/settings.json). Non-developer-first: question-style Korean labels,
  // "(기본값)" = key absent, caution notes on risky values, pre-save diff + backup.
  //
  // Field registry follows the official JSON Schema
  // (https://www.schemastore.org/claude-code-settings.json). Each change writes one
  // dotted-path key via config_set_value — every other key is preserved verbatim.
  import {
    configSetValue,
    inTauri,
    lintClaudeSettings,
    pickFolder,
    readRawSettings,
    saveRawSettings,
    type DiffView,
    type LintItem
  } from '$lib/ipc';
  import DiffViewer from '$lib/components/DiffViewer.svelte';
  import UnsavedMarker from '$lib/components/UnsavedMarker.svelte';
  import { modalFocus } from '$lib/modal';

  interface Option {
    v: unknown; // null = 키 제거(기본값으로)
    label: string;
  }
  interface Field {
    path: string;
    label: string;
    type: 'select' | 'text' | 'number' | 'folders';
    options?: Option[];
    placeholder?: string;
    help?: string;
    /** Shown highlighted when the current value satisfies the predicate. */
    caution?: { when: (v: unknown) => boolean; text: string };
  }
  interface Section {
    title: string;
    fields: Field[];
  }

  let { onsaved }: { onsaved?: () => void } = $props();

  const DEFAULT: Option = { v: null, label: '(기본값 — 키 없음)' };
  const boolOpts: Option[] = [
    DEFAULT,
    { v: true, label: '켬 (true)' },
    { v: false, label: '끔 (false)' }
  ];
  const sel = (values: string[]): Option[] => [DEFAULT, ...values.map((v) => ({ v, label: v }))];

  const sections: Section[] = [
    {
      title: '모델 · 응답',
      fields: [
        {
          path: 'model',
          label: '기본 모델',
          type: 'text',
          placeholder: 'sonnet · opus · haiku 또는 전체 모델 ID',
          help: '비워두면 Claude Code 기본 모델을 사용합니다'
        },
        {
          path: 'effortLevel',
          label: '추론 노력 수준',
          type: 'select',
          options: sel(['low', 'medium', 'high', 'xhigh']),
          help: '높을수록 더 깊게 생각하지만 느려집니다'
        },
        {
          path: 'alwaysThinkingEnabled',
          label: '항상 확장 사고 사용',
          type: 'select',
          options: boolOpts,
          help: '모든 응답에서 깊은 사고 과정을 켭니다'
        },
        {
          path: 'language',
          label: '응답 언어',
          type: 'text',
          placeholder: 'korean, english …',
          help: 'Claude가 응답에 사용할 언어'
        },
        {
          path: 'outputStyle',
          label: '출력 스타일',
          type: 'text',
          placeholder: 'Explanatory, Learning …',
          help: '/output-style 명령으로 만든 스타일 이름'
        }
      ]
    },
    {
      title: '동작',
      fields: [
        { path: 'autoUpdates', label: '자동 업데이트', type: 'select', options: boolOpts },
        {
          path: 'autoUpdatesChannel',
          label: '업데이트 채널',
          type: 'select',
          options: sel(['stable', 'latest'])
        },
        {
          path: 'cleanupPeriodDays',
          label: '대화 기록 보관 일수',
          type: 'number',
          placeholder: '30',
          help: '지난 세션 기록을 며칠 동안 보관할지 (기본 30일)'
        },
        {
          path: 'autoCompactEnabled',
          label: '컨텍스트 자동 압축',
          type: 'select',
          options: boolOpts,
          help: '대화가 길어지면 자동으로 요약해 이어갑니다'
        },
        {
          path: 'autoMemoryEnabled',
          label: '자동 메모리',
          type: 'select',
          options: boolOpts,
          help: '프로젝트별로 배운 내용을 자동으로 저장합니다'
        },
        {
          path: 'fileCheckpointingEnabled',
          label: '파일 체크포인트 (/rewind)',
          type: 'select',
          options: boolOpts,
          help: '파일 변경을 체크포인트로 저장해 /rewind로 되돌릴 수 있게 합니다'
        }
      ]
    },
    {
      title: 'Git · 커밋',
      fields: [
        {
          path: 'includeCoAuthoredBy',
          label: '커밋에 Claude 서명 포함',
          type: 'select',
          options: boolOpts,
          help: 'Co-Authored-By 트레일러. 기본값은 켬입니다'
        }
      ]
    },
    {
      title: '권한 · 보안',
      fields: [
        {
          path: 'permissions.disableBypassPermissionsMode',
          label: '권한 우회 모드 금지',
          type: 'select',
          options: [
            DEFAULT,
            { v: 'disable', label: 'disable — bypassPermissions 사용 금지 (권장)' }
          ],
          help: '켜면 모든 확인을 건너뛰는 bypassPermissions 모드를 쓸 수 없게 됩니다'
        },
        {
          path: 'enableAllProjectMcpServers',
          label: '프로젝트 MCP 서버 자동 승인',
          type: 'select',
          options: boolOpts,
          caution: {
            when: (v) => v === true,
            text: '켜면 프로젝트 .mcp.json의 모든 서버가 확인 없이 실행됩니다 — 신뢰할 수 있는 저장소에서만 켜세요'
          }
        },
        {
          path: 'permissions.additionalDirectories',
          label: '추가 접근 허용 폴더',
          type: 'folders',
          help: '프로젝트 폴더 밖에서 Claude가 접근할 수 있는 폴더 목록'
        }
      ]
    }
  ];

  let text = $state('');
  let onDisk = $state('');
  let loading = $state(true);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let status = $state<string | null>(null);
  let diff = $state<DiffView | null>(null);
  let lint = $state<LintItem[]>([]);
  let newEnvName = $state('');
  let newEnvValue = $state('');

  const dirty = $derived(text !== onDisk);

  // The parsed tree drives every field display; unparseable text disables the form.
  const tree = $derived.by<Record<string, unknown> | null>(() => {
    try {
      return text.trim() === '' ? {} : JSON.parse(text);
    } catch {
      return null;
    }
  });

  async function load() {
    if (!inTauri()) {
      loading = false;
      return;
    }
    loading = true;
    error = null;
    try {
      onDisk = await readRawSettings('', 'user');
      text = onDisk;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }
  load();

  // Debounced lint of the current text (known-key types + plaintext secrets).
  $effect(() => {
    const current = text;
    if (!inTauri()) return;
    const t = setTimeout(async () => {
      try {
        lint = await lintClaudeSettings(current);
      } catch {
        lint = [];
      }
    }, 350);
    return () => clearTimeout(t);
  });

  function getAt(path: string): unknown {
    let cur: unknown = tree;
    for (const seg of path.split('.')) {
      if (cur === null || typeof cur !== 'object') return undefined;
      cur = (cur as Record<string, unknown>)[seg];
    }
    return cur;
  }

  async function commit(path: string, v: unknown) {
    error = null;
    try {
      text = await configSetValue(text, 'json', path, v);
    } catch (e) {
      error = String(e);
    }
  }

  function selectedIndex(f: Field): number {
    const value = getAt(f.path);
    const opts = f.options ?? [];
    const i = opts.findIndex((o) => (value === undefined ? o.v === null : o.v === value));
    return i >= 0 ? i : 0;
  }

  function onSelect(f: Field, e: Event) {
    const i = (e.currentTarget as HTMLSelectElement).selectedIndex;
    commit(f.path, (f.options ?? [])[i]?.v ?? null);
  }

  function onText(f: Field, e: Event) {
    const v = (e.currentTarget as HTMLInputElement).value.trim();
    const current = getAt(f.path);
    if (v === (typeof current === 'string' ? current : '')) return;
    commit(f.path, v === '' ? null : v);
  }

  function onNumber(f: Field, e: Event) {
    const raw = (e.currentTarget as HTMLInputElement).value.trim();
    if (raw === '') {
      commit(f.path, null);
      return;
    }
    const n = Number(raw);
    if (!Number.isFinite(n)) return;
    commit(f.path, n);
  }

  // --- folders (string array) ------------------------------------------------

  function folderList(f: Field): string[] {
    const v = getAt(f.path);
    return Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : [];
  }

  async function addFolder(f: Field) {
    const picked = await pickFolder();
    if (!picked) return;
    const dir = picked.replaceAll('\\', '/');
    const next = [...folderList(f), dir];
    await commit(f.path, next);
  }

  async function removeFolder(f: Field, dir: string) {
    const next = folderList(f).filter((d) => d !== dir);
    await commit(f.path, next.length ? next : null);
  }

  // --- env (string map) ------------------------------------------------------

  const ENV_NAME_RE = /^[A-Za-z_][A-Za-z0-9_]*$/;

  const envEntries = $derived.by<[string, string][]>(() => {
    const env = tree?.['env'];
    if (env === null || typeof env !== 'object' || Array.isArray(env)) return [];
    return Object.entries(env as Record<string, unknown>).map(([k, v]) => [
      k,
      typeof v === 'string' ? v : JSON.stringify(v)
    ]);
  });

  function envWarning(name: string): string | null {
    const hit = lint.find((l) => l.level === 'warn' && l.path === `env.${name}`);
    return hit ? hit.message : null;
  }

  async function addEnv() {
    const name = newEnvName.trim();
    if (!ENV_NAME_RE.test(name)) {
      error = `환경변수 이름이 올바르지 않습니다: ${name || '(비어 있음)'}`;
      return;
    }
    await commit(`env.${name}`, newEnvValue);
    newEnvName = '';
    newEnvValue = '';
  }

  async function setEnv(name: string, e: Event) {
    await commit(`env.${name}`, (e.currentTarget as HTMLInputElement).value);
  }

  async function removeEnv(name: string) {
    await commit(`env.${name}`, null);
  }

  // --- save ------------------------------------------------------------------

  function openSaveDialog() {
    error = null;
    status = null;
    if (tree === null) {
      error = '현재 텍스트가 유효한 JSON이 아닙니다.';
      return;
    }
    diff = { path: '~/.claude/settings.json', before: onDisk, after: text, changed: dirty };
  }

  async function confirmSave() {
    if (!diff) return;
    saving = true;
    try {
      const res = await saveRawSettings({
        projectRoot: '',
        projectId: '',
        scope: 'user',
        text,
        projectName: ''
      });
      onDisk = text;
      status = `저장됨 → ${res.written}${res.backup ? ` (백업 생성됨)` : ''}`;
      diff = null;
      onsaved?.();
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  const lintWarns = $derived(lint.filter((l) => l.level === 'warn'));
  const lintInfos = $derived(lint.filter((l) => l.level === 'info'));
</script>

<UnsavedMarker id="claude-general" when={dirty} />

<div class="panel">
  <div class="bar">
    <p class="intro">
      권한 규칙 외의 Claude Code 전역 설정입니다. "(기본값)"을 선택하면 키를 제거해 Claude Code
      기본 동작을 따릅니다.
    </p>
    <button class="primary" class:dirty onclick={openSaveDialog} disabled={!dirty}>
      {dirty ? '● 저장…' : '저장됨'}
    </button>
  </div>

  {#if lintWarns.length}
    <div class="lint" role="alert">
      {#each lintWarns as l (l.path + l.message)}
        <span class="lint-item warn" title={l.message}><code>{l.path}</code> {l.message}</span>
      {/each}
    </div>
  {/if}

  {#if !inTauri()}
    <p class="muted">데스크톱 앱에서만 설정을 편집할 수 있습니다.</p>
  {:else if loading}
    <p class="muted">불러오는 중…</p>
  {:else if tree === null}
    <p class="warnbox">
      현재 파일을 파싱할 수 없습니다 — Raw JSON 탭에서 구문 오류를 먼저 고쳐주세요.
    </p>
  {:else}
    <div class="scroll">
      {#each sections as s (s.title)}
        <section>
          <h4>{s.title}</h4>
          {#each s.fields as f (f.path)}
            <div class="row">
              <label class="lbl" for={f.path} title={f.path}>{f.label}</label>
              {#if f.type === 'select'}
                <select id={f.path} onchange={(e) => onSelect(f, e)}>
                  {#each f.options ?? [] as o, i (i)}
                    <option value={i} selected={i === selectedIndex(f)}>{o.label}</option>
                  {/each}
                </select>
              {:else if f.type === 'number'}
                {@const nv = getAt(f.path)}
                <input
                  id={f.path}
                  type="number"
                  value={typeof nv === 'number' ? nv : ''}
                  placeholder={f.placeholder}
                  onchange={(e) => onNumber(f, e)}
                />
              {:else if f.type === 'folders'}
                <div class="folders">
                  {#each folderList(f) as dir (dir)}
                    <span class="chip">
                      <span class="dir">{dir}</span>
                      <button
                        class="x"
                        title="목록에서 제거"
                        aria-label={`${dir} 제거`}
                        onclick={() => removeFolder(f, dir)}>×</button
                      >
                    </span>
                  {/each}
                  <button class="add" onclick={() => addFolder(f)}>+ 폴더 선택…</button>
                </div>
              {:else}
                {@const tv = getAt(f.path)}
                <input
                  id={f.path}
                  type="text"
                  value={typeof tv === 'string' ? tv : ''}
                  placeholder={f.placeholder}
                  spellcheck="false"
                  onchange={(e) => onText(f, e)}
                />
              {/if}
            </div>
            {#if f.help}<p class="help">{f.help}</p>{/if}
            {#if f.caution && f.caution.when(getAt(f.path))}
              <p class="caution">{f.caution.text}</p>
            {/if}
          {/each}
        </section>
      {/each}

      <section>
        <h4>환경변수 (env)</h4>
        <p class="help">
          Claude Code 세션에만 적용되는 환경변수입니다. API 키 같은 비밀값은 여기 넣지 말고
          OS 환경변수나 apiKeyHelper를 사용하세요.
        </p>
        {#each envEntries as [name, value] (name)}
          <div class="env-row">
            <code class="env-name">{name}</code>
            <input
              type="text"
              class="env-value"
              {value}
              spellcheck="false"
              onchange={(e) => setEnv(name, e)}
            />
            <button class="x" title="변수 삭제" aria-label={`${name} 삭제`} onclick={() => removeEnv(name)}
              >×</button
            >
          </div>
          {#if envWarning(name)}
            <p class="caution">{envWarning(name)}</p>
          {/if}
        {/each}
        <div class="env-row new">
          <input
            type="text"
            class="env-name-in"
            placeholder="이름 (예: ANTHROPIC_MODEL)"
            spellcheck="false"
            bind:value={newEnvName}
          />
          <input
            type="text"
            class="env-value"
            placeholder="값"
            spellcheck="false"
            bind:value={newEnvValue}
          />
          <button class="add" onclick={addEnv} disabled={!newEnvName.trim()}>추가</button>
        </div>
      </section>

      {#if lintInfos.length}
        <section>
          <h4>참고</h4>
          {#each lintInfos as l (l.path + l.message)}
            <p class="note"><code>{l.path}</code> — {l.message}</p>
          {/each}
        </section>
      {/if}

      <p class="note">
        변경은 즉시 위 텍스트 상태에 반영되고, 저장 버튼을 눌러야 파일에 기록됩니다(자동 백업).
        여기에 없는 키는 Raw JSON 탭에서 편집하세요 — 그대로 보존됩니다.
      </p>
    </div>
  {/if}

  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if status}<p class="ok" role="status">{status}</p>{/if}
</div>

{#if diff}
  <div class="modal-bg" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) diff = null; }}>
    <div
      class="modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="claude-general-save-title"
      tabindex="-1"
      use:modalFocus={() => (diff = null)}
    >
      <h3 id="claude-general-save-title">저장 전 변경 확인 — 일반 설정</h3>
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

<style>
  .panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 0.6rem 0.8rem;
    box-sizing: border-box;
  }
  .bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.8rem;
    margin-bottom: 0.5rem;
  }
  .intro {
    color: var(--text-3);
    font-size: 0.74rem;
    margin: 0;
    line-height: 1.5;
  }
  .bar .primary {
    padding: 0.32rem 0.9rem;
    background: var(--bg-2);
    border: 1px solid var(--border-strong);
    color: var(--text-3);
    border-radius: var(--r-sm);
    cursor: default;
    font-size: 0.8rem;
    font-weight: 600;
    white-space: nowrap;
    transition: background-color var(--t-fast), color var(--t-fast), box-shadow var(--t-fast);
  }
  .bar .primary.dirty {
    background: var(--accent-strong);
    border-color: var(--accent-strong);
    color: white;
    cursor: pointer;
    box-shadow: 0 2px 12px rgba(37, 99, 235, 0.3);
  }
  .lint {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    margin-bottom: 0.5rem;
  }
  .lint-item {
    font-size: 0.72rem;
    border-radius: var(--r-sm);
    padding: 0.22rem 0.55rem;
  }
  .lint-item.warn {
    color: var(--ask);
    background: var(--ask-soft);
    border: 1px solid rgba(251, 191, 36, 0.35);
  }
  .lint-item code {
    font-weight: 600;
    margin-right: 0.3rem;
  }
  .scroll {
    flex: 1;
    overflow: auto;
    min-height: 0;
    padding-right: 0.3rem;
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
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.25rem;
  }
  .row {
    display: grid;
    grid-template-columns: 12rem 1fr;
    align-items: center;
    gap: 0.5rem;
    padding: 0.18rem 0;
  }
  .lbl {
    font-size: 0.78rem;
    color: var(--text-1);
  }
  select,
  input {
    background: var(--bg-1);
    color: var(--text-1);
    border: 1px solid var(--border-strong);
    border-radius: var(--r-sm);
    padding: 0.32rem 0.5rem;
    font-size: 0.76rem;
    min-width: 0;
  }
  input {
    font-family: var(--font-mono);
  }
  input::placeholder {
    color: var(--text-3);
    font-family: var(--font-sans);
  }
  select:focus,
  input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .help {
    color: var(--text-3);
    font-size: 0.68rem;
    margin: 0 0 0.25rem;
    padding-left: 0.1rem;
  }
  .caution {
    color: var(--ask);
    background: var(--ask-soft);
    border: 1px solid rgba(251, 191, 36, 0.35);
    border-radius: var(--r-sm);
    font-size: 0.7rem;
    padding: 0.25rem 0.5rem;
    margin: 0.15rem 0 0.35rem;
  }
  .folders {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.3rem;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    background: var(--bg-1);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0.12rem 0.3rem 0.12rem 0.6rem;
    font-size: 0.7rem;
  }
  .dir {
    font-family: var(--font-mono);
    color: var(--text-2);
    word-break: break-all;
  }
  .x {
    background: none;
    border: none;
    color: var(--text-3);
    cursor: pointer;
    font-size: 0.85rem;
    line-height: 1;
    padding: 0.1rem 0.25rem;
    border-radius: 999px;
  }
  .x:hover {
    color: var(--deny);
    background: var(--deny-soft);
  }
  .add {
    background: var(--bg-2);
    border: 1px solid var(--border-strong);
    color: var(--text-2);
    border-radius: var(--r-sm);
    padding: 0.22rem 0.6rem;
    cursor: pointer;
    font-size: 0.72rem;
    white-space: nowrap;
  }
  .add:hover:not(:disabled) {
    background: var(--bg-3);
    color: var(--text-1);
  }
  .add:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .env-row {
    display: grid;
    grid-template-columns: 14rem 1fr auto;
    align-items: center;
    gap: 0.4rem;
    padding: 0.15rem 0;
  }
  .env-name {
    font-size: 0.74rem;
    color: var(--text-1);
    word-break: break-all;
  }
  .env-name-in {
    font-size: 0.74rem;
  }
  .env-value {
    font-size: 0.74rem;
  }
  .note {
    color: var(--text-3);
    font-size: 0.7rem;
    margin: 0.25rem 0;
    line-height: 1.5;
  }
  .note code {
    color: var(--text-2);
  }
  .muted {
    color: var(--text-3);
    font-size: 0.8rem;
  }
  .warnbox {
    color: var(--ask);
    background: var(--ask-soft);
    border-radius: var(--r-sm);
    padding: 0.4rem 0.6rem;
    font-size: 0.76rem;
  }
  .err {
    color: var(--deny);
    font-size: 0.76rem;
    margin: 0.4rem 0 0;
  }
  .ok {
    color: var(--allow);
    font-size: 0.76rem;
    word-break: break-all;
    margin: 0.4rem 0 0;
  }
  /* Modal chrome comes from the shared app.css (.modal-bg / .modal / .modal-actions). */
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
</style>
