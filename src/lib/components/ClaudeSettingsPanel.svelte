<script lang="ts">
  // Structured editor for Claude Code settings beyond the permissions block
  // (~/.claude/settings.json). Field knowledge lives in claude-settings-catalog.ts:
  // every documented setting and env var is shown with the right control, so users
  // pick values instead of typing key names. Each change writes one dotted-path key
  // via config_set_value — every other key is preserved verbatim.
  import {
    configSetValue,
    getEnvStatus,
    inTauri,
    lintClaudeSettings,
    pickFolder,
    readRawSettings,
    saveRawSettings,
    type DiffView,
    type LintItem
  } from '$lib/ipc';
  import {
    CLAUDE_GROUPS,
    ENV_GROUPS,
    ENV_PRESETS,
    KNOWN_ENV,
    SECRET_ENV,
    type EnvPreset
  } from '$lib/claude-settings-catalog';
  import { getAt, type FieldSpec } from '$lib/setting-field';
  import DiffViewer from '$lib/components/DiffViewer.svelte';
  import SettingControl from '$lib/components/SettingControl.svelte';
  import UnsavedMarker from '$lib/components/UnsavedMarker.svelte';
  import { modalFocus } from '$lib/modal';
  import { toast } from '$lib/toast.svelte';
  import { tick } from 'svelte';

  let {
    onsaved,
    dirty = $bindable(false)
  }: { onsaved?: () => void; dirty?: boolean } = $props();

  const DIRS = 'permissions.additionalDirectories';

  let text = $state('');
  let onDisk = $state('');
  let loading = $state(true);
  let saving = $state(false);
  let pending: Promise<void> = Promise.resolve();
  let panelEl = $state<HTMLDivElement>();
  let diff = $state<DiffView | null>(null);
  let lint = $state<LintItem[]>([]);
  let osEnv = $state<Record<string, string>>({});
  let envQuery = $state('');
  let customName = $state('');
  let customValue = $state('');

  const changed = $derived(text !== onDisk);
  $effect(() => {
    dirty = changed;
  });

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
    try {
      onDisk = await readRawSettings('', 'user');
      text = onDisk;
    } catch (e) {
      toast(String(e), 'error');
    } finally {
      loading = false;
    }
    try {
      const st = await getEnvStatus();
      osEnv = Object.fromEntries(
        st.vars.filter((v) => v.present && !v.isSecret).map((v) => [v.name, v.display])
      );
    } catch {
      osEnv = {};
    }
  }
  load();

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

  function commit(path: string, v: unknown): Promise<void> {
    pending = pending.then(async () => {
      try {
        text = await configSetValue(text, 'json', path, v);
      } catch (e) {
        toast(String(e), 'error');
      }
    });
    return pending;
  }

  // --- folders ---------------------------------------------------------------

  const folders = $derived.by<string[]>(() => {
    const v = getAt(tree, DIRS);
    return Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : [];
  });

  async function addFolder() {
    const picked = await pickFolder();
    if (!picked) return;
    const dir = picked.replaceAll('\\', '/');
    if (folders.includes(dir)) return;
    await commit(DIRS, [...folders, dir]);
  }

  async function removeFolder(dir: string) {
    const prev = folders;
    const next = prev.filter((d) => d !== dir);
    await commit(DIRS, next.length ? next : null);
    toast(`폴더를 목록에서 제거했습니다: ${dir}`, 'info', {
      label: '되돌리기',
      run: () => commit(DIRS, prev)
    });
  }

  // --- env -------------------------------------------------------------------

  const ENV_NAME_RE = /^[A-Za-z_][A-Za-z0-9_]*$/;

  const env = $derived.by<Record<string, string>>(() => {
    const e = tree?.['env'];
    if (e === null || typeof e !== 'object' || Array.isArray(e)) return {};
    return Object.fromEntries(
      Object.entries(e as Record<string, unknown>).map(([k, v]) => [
        k,
        typeof v === 'string' ? v : JSON.stringify(v)
      ])
    );
  });

  const secretEntries = $derived(Object.keys(env).filter((k) => k in SECRET_ENV));
  const customEntries = $derived(
    Object.entries(env).filter(([k]) => !KNOWN_ENV.has(k) && !(k in SECRET_ENV))
  );

  function matches(f: FieldSpec, q: string): boolean {
    if (!q) return true;
    const hay = `${f.key} ${f.label} ${f.help ?? ''}`.toLowerCase();
    return q
      .toLowerCase()
      .split(/\s+/)
      .every((w) => hay.includes(w));
  }

  const envView = $derived(
    ENV_GROUPS.map((g) => ({
      ...g,
      shown: g.fields.filter((f) => matches(f, envQuery.trim())),
      setCount: g.fields.filter((f) => f.key in env).length
    })).filter((g) => g.shown.length > 0)
  );

  function setEnv(name: string, v: unknown) {
    return commit(`env.${name}`, v === null || v === undefined ? null : String(v));
  }

  function envWarning(name: string): string | null {
    const hit = lint.find((l) => l.level === 'warn' && l.path === `env.${name}`);
    return hit ? hit.message : null;
  }

  async function applyPreset(p: EnvPreset) {
    const values = p.values(osEnv);
    const before = Object.fromEntries(Object.keys(values).map((k) => [k, env[k] ?? null]));
    for (const [k, v] of Object.entries(values)) await setEnv(k, v);
    toast(`"${p.label}" 적용 — 저장하면 반영됩니다`, 'info', {
      label: '되돌리기',
      run: async () => {
        for (const [k, v] of Object.entries(before)) await setEnv(k, v);
      }
    });
  }

  function presetActive(p: EnvPreset): boolean {
    return Object.entries(p.values(osEnv)).every(([k, v]) => env[k] === v);
  }

  async function removeEnv(name: string) {
    const value = env[name];
    await setEnv(name, null);
    toast(`환경변수를 삭제했습니다: ${name}`, 'info', {
      label: '되돌리기',
      run: () => setEnv(name, value)
    });
  }

  async function addCustom() {
    const name = customName.trim();
    if (!ENV_NAME_RE.test(name)) {
      toast(`환경변수 이름이 올바르지 않습니다: ${name || '(비어 있음)'}`, 'error');
      return;
    }
    if (KNOWN_ENV.has(name)) {
      envQuery = name;
      toast(`${name} 은(는) 위 목록에 이미 있습니다 — 거기서 설정하세요`, 'info');
      return;
    }
    await setEnv(name, customValue);
    customName = '';
    customValue = '';
  }

  // --- save ------------------------------------------------------------------

  export async function openSaveDialog() {
    const active = document.activeElement;
    if (active instanceof HTMLElement && panelEl?.contains(active)) {
      active.blur();
      await tick();
    }
    await pending;
    if (tree === null) {
      toast('현재 텍스트가 유효한 JSON이 아닙니다.', 'error');
      return;
    }
    if (!changed) return;
    diff = { path: '~/.claude/settings.json', before: onDisk, after: text, changed };
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
      toast(`저장했습니다 → ${res.written}${res.backup ? ' (백업 생성됨)' : ''}`);
      diff = null;
      onsaved?.();
    } catch (e) {
      toast(String(e), 'error');
    } finally {
      saving = false;
    }
  }

  const lintWarns = $derived(
    lint.filter((l) => l.level === 'warn' && !l.path.startsWith('env.'))
  );
  const lintInfos = $derived(lint.filter((l) => l.level === 'info'));
</script>

<UnsavedMarker id="claude-general" when={changed} />

<div class="panel" bind:this={panelEl}>
  {#if lintWarns.length}
    <div class="lint" role="alert">
      {#each lintWarns as l (l.path + l.message)}
        <p class="callout callout-warn lint-item"><code>{l.path}</code> {l.message}</p>
      {/each}
    </div>
  {/if}

  {#if !inTauri()}
    <p class="muted">데스크톱 앱에서만 설정을 편집할 수 있습니다.</p>
  {:else if loading}
    <p class="muted">불러오는 중…</p>
  {:else if tree === null}
    <p class="callout callout-warn">
      현재 파일을 파싱할 수 없습니다 — Raw JSON 탭에서 구문 오류를 먼저 고쳐주세요.
    </p>
  {:else}
    <div class="scroll">
      <p class="intro">
        모든 프로젝트에 적용되는 Claude Code 설정입니다. <b>기본값</b>을 고르면 설정이 지워지고
        Claude Code 기본 동작을 따릅니다. 항목 이름에 마우스를 올리면 실제 키가 보입니다.
      </p>

      {#each CLAUDE_GROUPS as g (g.title)}
        <section class="card">
          <header>
            <h3>{g.title}</h3>
            {#if g.description}<p class="desc">{g.description}</p>{/if}
          </header>
          {#each g.fields as f (f.key)}
            <SettingControl field={f} value={getAt(tree, f.key)} onchange={(v) => commit(f.key, v)} />
          {/each}
          {#if g.title === '안전'}
            <div class="row-custom">
              <div>
                <span class="lbl">프로젝트 밖에서 접근할 폴더</span>
                <p class="help">작업 중인 프로젝트 외에 Claude가 읽고 쓸 수 있는 폴더입니다</p>
              </div>
              <div class="folders">
                {#each folders as dir (dir)}
                  <span class="folder-chip">
                    <span class="dir">{dir}</span>
                    <button class="x" aria-label={`${dir} 제거`} onclick={() => removeFolder(dir)}>×</button>
                  </span>
                {:else}
                  <span class="none">없음</span>
                {/each}
                <button class="btn btn-sm" onclick={addFolder}>+ 폴더 선택…</button>
              </div>
            </div>
          {/if}
        </section>
      {/each}

      <section class="card">
        <header>
          <h3>환경변수</h3>
          <p class="desc">
            Claude Code가 지원하는 환경변수를 모두 모아 두었습니다. 필요한 항목만 켜거나 고르세요 —
            Claude Code 세션에만 적용됩니다.
          </p>
        </header>

        <div class="presets" aria-label="빠른 설정">
          {#each ENV_PRESETS as p (p.id)}
            {@const active = presetActive(p)}
            <button class="preset" class:active disabled={active} onclick={() => applyPreset(p)}>
              <span class="p-label">{active ? '적용됨 · ' : ''}{p.label}</span>
              <span class="p-desc">{p.description}</span>
            </button>
          {/each}
        </div>

        {#if secretEntries.length}
          <div class="callout callout-warn secrets" role="alert">
            <p>
              비밀값이 설정 파일에 평문으로 저장되어 있습니다. 파일이 유출되면 그대로 노출되므로 OS
              환경변수나 <code>apiKeyHelper</code>로 옮기는 것을 권장합니다.
            </p>
            {#each secretEntries as k (k)}
              <div class="secret-row">
                <span>{SECRET_ENV[k]} <code>{k}</code></span>
                <button class="btn btn-sm btn-danger" onclick={() => removeEnv(k)}>설정에서 삭제</button>
              </div>
            {/each}
          </div>
        {/if}

        <input
          class="input search"
          type="search"
          placeholder="환경변수 찾기 — 예: 프록시, 타임아웃, bedrock"
          aria-label="환경변수 검색"
          bind:value={envQuery}
        />

        {#each envView as g (g.id)}
          <details class="env-group" open={envQuery.trim() !== '' || g.setCount > 0}>
            <summary>
              <span class="g-title">{g.title}</span>
              {#if g.setCount}<span class="chip chip-accent">{g.setCount}개 설정됨</span>{/if}
            </summary>
            {#if g.description}<p class="desc g-desc">{g.description}</p>{/if}
            {#each g.shown as f (f.key)}
              <SettingControl field={f} value={env[f.key]} onchange={(v) => setEnv(f.key, v)}>
                {#snippet extra()}
                  {#if osEnv[f.key] && env[f.key] === undefined}
                    <p class="os-hint">
                      이 PC에 이미 <code>{osEnv[f.key]}</code> 로 설정되어 있어 그대로 적용됩니다.
                      <button class="link" onclick={() => setEnv(f.key, osEnv[f.key])}>
                        설정 파일에 고정
                      </button>
                    </p>
                  {/if}
                  {#if envWarning(f.key)}
                    <p class="callout callout-warn caution">{envWarning(f.key)}</p>
                  {/if}
                {/snippet}
              </SettingControl>
            {/each}
          </details>
        {:else}
          <p class="muted">"{envQuery}" 와 일치하는 환경변수가 없습니다. 아래에서 직접 추가할 수 있습니다.</p>
        {/each}

        <details class="env-group" open={customEntries.length > 0}>
          <summary>
            <span class="g-title">목록에 없는 변수</span>
            {#if customEntries.length}<span class="chip">{customEntries.length}개</span>{/if}
          </summary>
          <p class="desc g-desc">
            도구나 MCP 서버가 요구하는 변수처럼 위 목록에 없는 것은 이름과 값을 직접 입력합니다.
          </p>
          {#each customEntries as [name, value] (name)}
            <div class="env-row">
              <code class="env-name">{name}</code>
              <input
                class="input mono"
                {value}
                spellcheck="false"
                aria-label={`${name} 값`}
                onchange={(e) => setEnv(name, (e.currentTarget as HTMLInputElement).value)}
              />
              <button class="x" aria-label={`${name} 삭제`} onclick={() => removeEnv(name)}>×</button>
            </div>
            {#if envWarning(name)}
              <p class="callout callout-warn caution">{envWarning(name)}</p>
            {/if}
          {/each}
          <div class="env-row">
            <input
              class="input mono"
              placeholder="이름 (예: MY_TOOL_HOME)"
              spellcheck="false"
              aria-label="새 환경변수 이름"
              bind:value={customName}
            />
            <input
              class="input mono"
              placeholder="값"
              spellcheck="false"
              aria-label="새 환경변수 값"
              bind:value={customValue}
              onkeydown={(e) => e.key === 'Enter' && customName.trim() && addCustom()}
            />
            <button class="btn btn-sm" onclick={addCustom} disabled={!customName.trim()}>추가</button>
          </div>
        </details>
      </section>

      {#if lintInfos.length}
        <section class="card">
          <header><h3>그 밖의 설정</h3></header>
          {#each lintInfos as l (l.path + l.message)}
            <p class="note"><code>{l.path}</code> — {l.message}</p>
          {/each}
        </section>
      {/if}

      <p class="note">
        변경은 저장 버튼(Ctrl+S)을 눌러야 파일에 기록되며, 저장 전 자동으로 백업됩니다. 여기에 없는
        설정은 Raw JSON 탭에서 편집하세요 — 그대로 보존됩니다.
      </p>
    </div>
  {/if}
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
  .lint {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    margin-bottom: 0.5rem;
  }
  .lint-item {
    font-size: var(--fs-sm);
    padding: 0.25rem 0.55rem;
  }
  .scroll {
    flex: 1;
    overflow: auto;
    min-height: 0;
    padding-right: 0.3rem;
    container-type: inline-size;
  }
  .intro {
    color: var(--text-2);
    font-size: var(--fs-sm);
    margin: 0 0 0.7rem;
    line-height: 1.55;
  }
  .card {
    background: var(--bg-1);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    padding: 0.7rem 0.9rem 0.3rem;
    margin-bottom: 0.8rem;
  }
  .card header {
    margin-bottom: 0.2rem;
  }
  h3 {
    font-size: var(--fs-lg);
    margin: 0;
    color: var(--text-1);
  }
  .desc {
    color: var(--text-3);
    font-size: var(--fs-xs);
    margin: 0.2rem 0 0;
    line-height: 1.5;
  }
  .row-custom {
    display: grid;
    grid-template-columns: minmax(10rem, 15rem) minmax(0, 1fr);
    gap: 0.35rem 1rem;
    padding: 0.55rem 0;
    border-top: 1px solid var(--border);
  }
  .lbl {
    font-size: var(--fs-md);
    font-weight: 500;
  }
  .help {
    color: var(--text-3);
    font-size: var(--fs-xs);
    margin: 0.15rem 0 0;
  }
  .folders {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.3rem;
    min-width: 0;
  }
  .none {
    color: var(--text-3);
    font-size: var(--fs-sm);
  }
  .folder-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    max-width: 100%;
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0.12rem 0.3rem 0.12rem 0.6rem;
    font-size: var(--fs-xs);
  }
  .dir {
    font-family: var(--font-mono);
    color: var(--text-2);
    word-break: break-all;
  }
  .x {
    flex-shrink: 0;
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
  .presets {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(13rem, 1fr));
    gap: 0.45rem;
    margin: 0.6rem 0;
  }
  .preset {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    text-align: left;
    background: var(--bg-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--r-sm);
    padding: 0.5rem 0.65rem;
    cursor: pointer;
    color: var(--text-1);
    transition: border-color var(--t-fast), background-color var(--t-fast);
  }
  .preset:hover:not(:disabled) {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .preset.active {
    border-color: var(--allow-line);
    background: var(--allow-soft);
    cursor: default;
  }
  .p-label {
    font-size: var(--fs-md);
    font-weight: 600;
  }
  .p-desc {
    font-size: var(--fs-xs);
    color: var(--text-2);
    line-height: 1.4;
  }
  .secrets {
    margin: 0.4rem 0;
    font-size: var(--fs-sm);
  }
  .secrets p {
    margin: 0 0 0.4rem;
  }
  .secret-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
    padding: 0.15rem 0;
  }
  .search {
    margin: 0.3rem 0 0.5rem;
    font-size: var(--fs-sm);
  }
  .env-group {
    border-top: 1px solid var(--border);
    padding: 0.15rem 0;
  }
  .env-group summary {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.45rem 0;
    cursor: pointer;
    list-style: none;
  }
  .env-group summary::-webkit-details-marker {
    display: none;
  }
  .env-group summary::before {
    content: '▸';
    color: var(--text-3);
    font-size: 0.7rem;
    transition: transform var(--t-fast);
  }
  .env-group[open] summary::before {
    transform: rotate(90deg);
  }
  .g-title {
    font-size: var(--fs-md);
    font-weight: 600;
    color: var(--text-1);
  }
  .g-desc {
    margin: 0 0 0.2rem 1.1rem;
  }
  .os-hint {
    margin: 0;
    font-size: var(--fs-xs);
    color: var(--allow);
  }
  .os-hint code {
    font-family: var(--font-mono);
  }
  .link {
    background: none;
    border: none;
    padding: 0;
    margin-left: 0.3rem;
    color: var(--accent-text);
    font-size: var(--fs-xs);
    cursor: pointer;
    text-decoration: underline;
  }
  .caution {
    font-size: var(--fs-xs);
    padding: 0.25rem 0.5rem;
    margin: 0.15rem 0 0.35rem;
  }
  .env-row {
    display: grid;
    grid-template-columns: minmax(8rem, 14rem) minmax(0, 1fr) auto;
    align-items: center;
    gap: 0.4rem;
    padding: 0.2rem 0;
  }
  .env-name {
    font-size: var(--fs-sm);
    word-break: break-all;
  }
  .input {
    font-size: var(--fs-sm);
    padding: 0.32rem 0.5rem;
  }
  .mono {
    font-family: var(--font-mono);
  }
  .mono::placeholder {
    font-family: var(--font-sans);
  }
  .note {
    color: var(--text-3);
    font-size: var(--fs-xs);
    margin: 0.3rem 0;
    line-height: 1.5;
  }
  .note code {
    color: var(--text-2);
  }
  @container (max-width: 34rem) {
    .row-custom {
      grid-template-columns: minmax(0, 1fr);
    }
    .env-row {
      grid-template-columns: minmax(0, 1fr) auto;
    }
    .env-row > :first-child {
      grid-column: 1 / -1;
    }
  }
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
