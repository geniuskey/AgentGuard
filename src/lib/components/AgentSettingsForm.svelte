<script lang="ts">
  // Structured settings form over a raw agent config (opencode.json / config.toml).
  // Reads the parsed tree, writes one dotted-path key per change via the backend
  // (all other keys preserved), and hands the new text back to the raw editor.
  // Field registries follow the official docs:
  //   OpenCode: https://opencode.ai/docs/config, /docs/permissions
  //   Codex:    https://developers.openai.com/codex/config-reference
  import { configGet, configSetValue, inTauri } from '$lib/ipc';

  interface Option {
    v: unknown; // null = 키 제거(기본값으로)
    label: string;
  }
  interface Field {
    path: string;
    label: string;
    type: 'select' | 'text';
    options?: Option[];
    placeholder?: string;
    help?: string;
    /** OpenCode permission values can be a pattern map — edit its `"*"` base. */
    permMap?: boolean;
  }
  interface Section {
    title: string;
    fields: Field[];
  }

  let {
    agentId,
    format,
    text,
    onchange
  }: {
    agentId: string;
    format: string;
    text: string;
    onchange: (t: string) => void;
  } = $props();

  const DEFAULT: Option = { v: null, label: '(기본값 — 키 없음)' };
  const boolOpts: Option[] = [DEFAULT, { v: true, label: 'true' }, { v: false, label: 'false' }];
  const permOpts: Option[] = [
    DEFAULT,
    { v: 'allow', label: 'allow — 자동 허용' },
    { v: 'ask', label: 'ask — 확인 후 실행' },
    { v: 'deny', label: 'deny — 차단' }
  ];
  const sel = (values: string[]): Option[] => [DEFAULT, ...values.map((v) => ({ v, label: v }))];
  const perm = (key: string, label: string, help?: string): Field => ({
    path: `permission.${key}`,
    label,
    type: 'select',
    options: permOpts,
    help,
    permMap: true
  });

  const registries: Record<string, Section[]> = {
    opencode: [
      {
        title: '모델',
        fields: [
          { path: 'model', label: '기본 모델', type: 'text', placeholder: 'anthropic/claude-sonnet-4-5' },
          { path: 'small_model', label: '보조 모델', type: 'text', placeholder: '제목 생성 등 경량 작업용', help: '비워두면 기본 모델을 사용합니다' }
        ]
      },
      {
        title: '동작',
        fields: [
          { path: 'theme', label: '테마', type: 'text', placeholder: 'opencode' },
          {
            path: 'share',
            label: '대화 공유',
            type: 'select',
            options: [
              DEFAULT,
              { v: 'manual', label: 'manual — 명시적으로만 공유' },
              { v: 'auto', label: 'auto — 자동 공유' },
              { v: 'disabled', label: 'disabled — 공유 금지' }
            ],
            help: '공유는 대화를 외부 서버에 업로드합니다'
          },
          {
            path: 'autoupdate',
            label: '자동 업데이트',
            type: 'select',
            options: [DEFAULT, { v: true, label: 'true' }, { v: false, label: 'false' }, { v: 'notify', label: 'notify — 알림만' }]
          }
        ]
      },
      {
        title: '권한 (permission)',
        fields: [
          perm('edit', '파일 수정', 'edit/write/patch 도구'),
          perm('bash', '셸 실행'),
          perm('webfetch', '웹 요청 (URL fetch)'),
          perm('websearch', '웹 검색'),
          perm('read', '파일 읽기'),
          perm('external_directory', '프로젝트 밖 접근', '작업 디렉터리 외부 경로')
        ]
      }
    ],
    codex: [
      {
        title: '모델',
        fields: [
          { path: 'model', label: '모델', type: 'text', placeholder: 'gpt-5.5' },
          { path: 'model_reasoning_effort', label: '추론 강도', type: 'select', options: sel(['minimal', 'low', 'medium', 'high', 'xhigh']) },
          { path: 'model_verbosity', label: '응답 상세도', type: 'select', options: sel(['low', 'medium', 'high']) },
          { path: 'personality', label: '말투', type: 'select', options: sel(['none', 'friendly', 'pragmatic']) }
        ]
      },
      {
        title: '승인 · 샌드박스',
        fields: [
          {
            path: 'approval_policy',
            label: '승인 정책',
            type: 'select',
            options: [
              DEFAULT,
              { v: 'untrusted', label: 'untrusted — 신뢰 명령 외 모두 확인' },
              { v: 'on-request', label: 'on-request — 모델이 요청할 때' },
              { v: 'never', label: 'never — 확인 없음 (주의)' }
            ]
          },
          {
            path: 'sandbox_mode',
            label: '샌드박스',
            type: 'select',
            options: [
              DEFAULT,
              { v: 'read-only', label: 'read-only — 읽기 전용' },
              { v: 'workspace-write', label: 'workspace-write — 작업 폴더만 쓰기' },
              { v: 'danger-full-access', label: 'danger-full-access — 무제한 (주의)' }
            ]
          },
          { path: 'sandbox_workspace_write.network_access', label: '샌드박스 네트워크', type: 'select', options: boolOpts, help: 'workspace-write에서 네트워크 허용 여부' },
          { path: 'tools.web_search', label: '웹 검색 도구', type: 'select', options: boolOpts }
        ]
      },
      {
        title: '기타',
        fields: [
          { path: 'file_opener', label: '파일 열기 (인용 링크)', type: 'select', options: sel(['vscode', 'vscode-insiders', 'windsurf', 'cursor', 'none']) },
          { path: 'hide_agent_reasoning', label: '추론 과정 숨김', type: 'select', options: boolOpts },
          { path: 'history.persistence', label: '히스토리 저장', type: 'select', options: sel(['save-all', 'none']) }
        ]
      }
    ]
  };

  const sections = $derived(registries[agentId] ?? []);

  let tree = $state<Record<string, unknown> | null>(null);
  let parseError = $state(false);
  let error = $state<string | null>(null);

  // Re-parse the config whenever the text changes (raw edits included).
  $effect(() => {
    const current = text;
    const fmt = format;
    if (!inTauri()) return;
    const t = setTimeout(async () => {
      try {
        tree = await configGet(current, fmt);
        parseError = false;
      } catch {
        parseError = true;
      }
    }, 300);
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

  /** Current raw value; permission pattern maps resolve to their `"*"` base. */
  function fieldValue(f: Field): { value: unknown; patterns: number } {
    const raw = getAt(f.path);
    if (f.permMap && raw !== null && typeof raw === 'object' && !Array.isArray(raw)) {
      const m = raw as Record<string, unknown>;
      const extra = Object.keys(m).filter((k) => k !== '*').length;
      return { value: m['*'], patterns: extra };
    }
    return { value: raw, patterns: 0 };
  }

  function writePath(f: Field): string {
    const raw = getAt(f.path);
    if (f.permMap && raw !== null && typeof raw === 'object' && !Array.isArray(raw)) {
      return `${f.path}.*`; // keep the user's pattern entries intact
    }
    return f.path;
  }

  async function commit(f: Field, v: unknown) {
    error = null;
    try {
      onchange(await configSetValue(text, format, writePath(f), v));
    } catch (e) {
      error = String(e);
    }
  }

  function selectedIndex(f: Field): number {
    const { value } = fieldValue(f);
    const opts = f.options ?? [];
    const i = opts.findIndex((o) => (value === undefined ? o.v === null : o.v === value));
    return i >= 0 ? i : 0;
  }

  function onSelect(f: Field, e: Event) {
    const i = (e.currentTarget as HTMLSelectElement).selectedIndex;
    commit(f, (f.options ?? [])[i]?.v ?? null);
  }

  function onText(f: Field, e: Event) {
    const v = (e.currentTarget as HTMLInputElement).value.trim();
    const current = fieldValue(f).value;
    if (v === (typeof current === 'string' ? current : '')) return;
    commit(f, v === '' ? null : v);
  }
</script>

<div class="form">
  {#if !inTauri()}
    <p class="muted">데스크톱 앱에서만 설정을 편집할 수 있습니다.</p>
  {:else if parseError}
    <p class="warn">현재 텍스트를 파싱할 수 없습니다 — Raw 탭에서 구문 오류를 먼저 고쳐주세요.</p>
  {:else if !tree}
    <p class="muted">읽는 중…</p>
  {:else}
    {#each sections as s (s.title)}
      <section>
        <h4>{s.title}</h4>
        {#each s.fields as f (f.path)}
          {@const fv = fieldValue(f)}
          <div class="row">
            <label class="lbl" for={f.path} title={f.help}>{f.label}</label>
            {#if f.type === 'select'}
              <select id={f.path} onchange={(e) => onSelect(f, e)}>
                {#each f.options ?? [] as o, i (i)}
                  <option value={i} selected={i === selectedIndex(f)}>{o.label}</option>
                {/each}
              </select>
            {:else}
              <input
                id={f.path}
                type="text"
                value={typeof fv.value === 'string' ? fv.value : ''}
                placeholder={f.placeholder}
                spellcheck="false"
                onchange={(e) => onText(f, e)}
              />
            {/if}
            {#if fv.patterns > 0}
              <span class="pat" title="패턴별 세부 규칙은 Raw 탭에서 편집하세요">+패턴 {fv.patterns}</span>
            {/if}
          </div>
          {#if f.help}<p class="help">{f.help}</p>{/if}
        {/each}
      </section>
    {/each}
    <p class="note">
      변경은 즉시 텍스트에 반영되고, 저장 버튼을 눌러야 파일에 기록됩니다. 여기에 없는 키는 Raw
      탭에서 편집하세요 — 그대로 보존됩니다.
    </p>
  {/if}

  {#if error}<p class="err" role="alert">{error}</p>{/if}
</div>

<style>
  .form {
    flex: 1;
    overflow: auto;
    padding: 0.2rem 0.1rem;
  }
  section {
    margin-bottom: 0.9rem;
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
    grid-template-columns: 11rem 1fr auto;
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
  .pat {
    font-size: 0.64rem;
    color: var(--ask);
    background: var(--ask-soft);
    border: 1px solid rgba(251, 191, 36, 0.35);
    border-radius: 999px;
    padding: 0.05rem 0.45rem;
    white-space: nowrap;
  }
  .help {
    grid-column: 1 / -1;
    color: var(--text-3);
    font-size: 0.68rem;
    margin: 0 0 0.25rem;
    padding-left: 0.1rem;
  }
  .note {
    color: var(--text-3);
    font-size: 0.7rem;
    margin-top: 0.6rem;
    line-height: 1.5;
  }
  .muted {
    color: var(--text-3);
    font-size: 0.8rem;
  }
  .warn {
    color: var(--ask);
    background: var(--ask-soft);
    border-radius: var(--r-sm);
    padding: 0.4rem 0.6rem;
    font-size: 0.76rem;
  }
  .err {
    color: var(--deny);
    font-size: 0.76rem;
  }
</style>
