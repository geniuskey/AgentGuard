<script lang="ts">
  import {
    inTauri,
    noteIgnoredPath,
    pathIgnored,
    type AppliesTo,
    type Policy,
    type ScopeName
  } from '$lib/ipc';
  import { app, clearPolicy, refreshEffective, upsertRule } from '$lib/state.svelte';
  import { buildPolicyRule } from '$lib/policy-rule';

  let appliesTo = $state<AppliesTo>('folder-and-children');
  let reason = $state('');
  let notes = $state('');
  let riskLevel = $state<'' | 'low' | 'medium' | 'high'>('');
  let useRead = $state(true);
  let useEdit = $state(true);

  const target = $derived(app.selectedPath || '(project root)');

  // Git-ignored paths need a heads-up: an Allow rule opens access, but the
  // agent's Grep search still skips them.
  let ignored = $state(false);
  let noteStatus = $state<string | null>(null);

  $effect(() => {
    const p = app.selectedPath;
    ignored = false;
    noteStatus = null;
    if (!p || !inTauri()) return;
    pathIgnored(app.projectRoot, p)
      .then((v) => (ignored = v))
      .catch(() => {});
  });

  async function noteInClaudeMd() {
    try {
      const added = await noteIgnoredPath(app.projectRoot, app.selectedPath);
      noteStatus = added
        ? 'CLAUDE.md에 안내를 추가했습니다 — 에이전트가 이 경로를 인지합니다.'
        : 'CLAUDE.md에 이미 안내가 있습니다.';
    } catch (e) {
      noteStatus = String(e);
    }
  }

  // The explicit rule currently on this path in the active scope, if any.
  const current = $derived(
    app.scoped[app.activeScope].rules.find((r) => r.path === app.selectedPath)
  );

  $effect(() => {
    const rule = current;
    appliesTo = rule?.appliesTo ?? 'folder-and-children';
    reason = rule?.reason ?? '';
    notes = rule?.notes ?? '';
    riskLevel = rule?.riskLevel ?? '';
    const tools = rule?.tools;
    useRead = !tools || tools.includes('Read');
    useEdit = !tools || tools.includes('Edit');
  });

  async function apply(policy: Policy) {
    if (!app.selectedPath || (!useRead && !useEdit)) return;
    const rule = buildPolicyRule({
      path: app.selectedPath,
      policy,
      appliesTo,
      useRead,
      useEdit,
      reason,
      riskLevel,
      notes
    });
    if (!rule) return;
    upsertRule(app.activeScope, rule);
    await refreshEffective();
  }

  async function clear() {
    if (!app.selectedPath) return;
    clearPolicy(app.selectedPath);
    await refreshEffective();
  }

  const scopes: ScopeName[] = ['project', 'local'];
</script>

<div class="panel">
  <h3>Policy Editor</h3>

  <div class="field">
    <span>Path</span>
    <code class="target">{target}</code>
  </div>

  <div class="field">
    <span>Scope</span>
    <div class="segmented">
      {#each scopes as s (s)}
        <button class:active={app.activeScope === s} onclick={() => (app.activeScope = s)}>{s}</button>
      {/each}
    </div>
  </div>

  <label class="field">
    <span>Applies to</span>
    <select bind:value={appliesTo} disabled={!app.selectedPath}>
      <option value="file">This file only</option>
      <option value="folder">This folder only</option>
      <option value="folder-and-children">This folder and children</option>
      <option value="pattern">Matching pattern</option>
    </select>
  </label>

  <fieldset class="field tools-field" disabled={!app.selectedPath}>
    <legend>적용 도구</legend>
    <label><input type="checkbox" bind:checked={useRead} /> Read</label>
    <label><input type="checkbox" bind:checked={useEdit} /> Edit</label>
    {#if !useRead && !useEdit}<small>도구를 하나 이상 선택하세요.</small>{/if}
  </fieldset>

  <label class="field">
    <span>사유</span>
    <input bind:value={reason} disabled={!app.selectedPath} placeholder="예: 소스 코드 작업에 필요" />
  </label>

  <label class="field">
    <span>위험도</span>
    <select bind:value={riskLevel} disabled={!app.selectedPath}>
      <option value="">지정 안 함</option>
      <option value="low">낮음</option>
      <option value="medium">중간</option>
      <option value="high">높음</option>
    </select>
  </label>

  <label class="field">
    <span>메모</span>
    <textarea bind:value={notes} disabled={!app.selectedPath} rows="2" placeholder="팀 또는 개인 참고 사항"></textarea>
  </label>

  <div class="field">
    <span>Current (this scope)</span>
    {#if current}
      <span class="cur cur-{current.policy}">{current.policy.toUpperCase()}</span>
    {:else}
      <span class="cur cur-none">— untracked</span>
    {/if}
  </div>

  <div class="buttons">
    <button class="allow" onclick={() => apply('allow')} disabled={!app.selectedPath || (!useRead && !useEdit)}>
      Allow <kbd>A</kbd>
    </button>
    <button class="ask" onclick={() => apply('ask')} disabled={!app.selectedPath || (!useRead && !useEdit)}>
      Ask <kbd>K</kbd>
    </button>
    <button class="deny" onclick={() => apply('deny')} disabled={!app.selectedPath || (!useRead && !useEdit)}>
      Deny <kbd>D</kbd>
    </button>
    <button class="clear" onclick={clear} disabled={!current}>Clear rule</button>
  </div>

  <div class="boundary-note">
    <b>권한 경계</b>
    Allow는 일치한 호출을 미리 승인할 뿐, 미등록 접근을 차단하는 화이트리스트가 아닙니다.
    Read/Edit Deny는 Claude의 내장 파일 도구에는 적용되지만, 허용된 Bash·PowerShell 프로세스의
    OS 파일 접근까지 차단하지는 않습니다.
  </div>

  {#if ignored}
    <div class="ginote">
      <b>.gitignore에 포함된 경로입니다.</b> 차단된 것은 아닙니다 —
      Grep 검색으로는 <b>발견 못 함</b> · 경로를 알면 Read <b>가능</b> · 접근 권한은
      Allow/Ask/Deny 규칙이 결정. CLAUDE.md에 경로를 알려주면 에이전트가 직접 읽거나
      <code>rg --no-ignore</code>로 검색할 수 있습니다.
      <button class="gbtn" onclick={noteInClaudeMd}>CLAUDE.md에 알리기</button>
      {#if noteStatus}<span class="gstat">{noteStatus}</span>{/if}
    </div>
  {/if}

  {#if !app.selectedPath}
    <p class="hint">왼쪽 트리에서 파일/폴더를 선택하세요.</p>
  {/if}
</div>

<style>
  .panel {
    padding: 0.85rem;
  }
  h3 {
    margin: 0 0 0.9rem;
    font-size: 0.8rem;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--text-2);
  }
  .field {
    display: block;
    margin-bottom: 0.85rem;
  }
  .field > span {
    display: block;
    font-size: 0.68rem;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-3);
    margin-bottom: 0.3rem;
  }
  .target {
    background: var(--bg-1);
    border: 1px solid var(--border);
    padding: 0.3rem 0.5rem;
    border-radius: var(--r-sm);
    font-size: 0.79rem;
    display: block;
    word-break: break-all;
    color: var(--text-1);
  }
  select,
  input,
  textarea {
    width: 100%;
    padding: 0.45rem;
    background: var(--bg-1);
    color: var(--text-1);
    border: 1px solid var(--border-strong);
    border-radius: var(--r-sm);
    font-size: 0.84rem;
  }
  textarea {
    resize: vertical;
    font-family: inherit;
  }
  .tools-field {
    border: 0;
    padding: 0;
    display: flex;
    align-items: center;
    gap: 0.8rem;
  }
  .tools-field legend {
    color: var(--text-3);
    font-size: 0.68rem;
    font-weight: 600;
    letter-spacing: 0.06em;
    margin-bottom: 0.3rem;
    text-transform: uppercase;
  }
  .tools-field label {
    color: var(--text-2);
    font-size: 0.76rem;
  }
  .tools-field input {
    width: auto;
  }
  .tools-field small {
    color: var(--deny);
  }
  select:disabled {
    opacity: 0.5;
  }
  .segmented {
    display: flex;
    background: var(--bg-1);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    padding: 0.18rem;
    gap: 0.18rem;
  }
  .segmented button {
    flex: 1;
    padding: 0.32rem;
    background: transparent;
    border: none;
    color: var(--text-2);
    border-radius: 4px;
    cursor: pointer;
    text-transform: capitalize;
    font-size: 0.8rem;
    transition: background-color var(--t-fast), color var(--t-fast);
  }
  .segmented button:hover {
    color: var(--text-1);
  }
  .segmented button.active {
    background: var(--bg-3);
    color: var(--accent-text);
    box-shadow: var(--shadow-1);
  }
  .cur {
    display: inline-block;
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    padding: 0.18rem 0.6rem;
    border-radius: 999px;
  }
  .cur-allow {
    background: var(--allow-soft);
    color: var(--allow);
    border: 1px solid rgba(52, 211, 153, 0.3);
  }
  .cur-ask {
    background: var(--ask-soft);
    color: var(--ask);
    border: 1px solid rgba(251, 191, 36, 0.3);
  }
  .cur-deny {
    background: var(--deny-soft);
    color: var(--deny);
    border: 1px solid rgba(248, 113, 113, 0.3);
  }
  .cur-none {
    color: var(--text-3);
    border: 1px dashed var(--border-strong);
    font-weight: 500;
  }
  .buttons {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.45rem;
    margin-top: 0.6rem;
  }
  .buttons button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    padding: 0.55rem;
    border-radius: var(--r-sm);
    border: 1px solid var(--border-strong);
    cursor: pointer;
    color: var(--text-1);
    background: var(--bg-2);
    font-size: 0.85rem;
    font-weight: 600;
    transition: background-color var(--t-fast), border-color var(--t-fast), box-shadow var(--t-fast);
  }
  .buttons button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .buttons kbd {
    font-size: 0.62rem;
    padding: 0 0.3rem;
    opacity: 0.7;
  }
  .allow {
    border-color: rgba(52, 211, 153, 0.35);
    color: var(--allow);
    background: var(--allow-soft);
  }
  .allow:hover:not(:disabled) {
    background: rgba(52, 211, 153, 0.22);
    box-shadow: 0 0 14px rgba(52, 211, 153, 0.15);
  }
  .ask {
    border-color: rgba(251, 191, 36, 0.35);
    color: var(--ask);
    background: var(--ask-soft);
  }
  .ask:hover:not(:disabled) {
    background: rgba(251, 191, 36, 0.22);
    box-shadow: 0 0 14px rgba(251, 191, 36, 0.15);
  }
  .deny {
    border-color: rgba(248, 113, 113, 0.35);
    color: var(--deny);
    background: var(--deny-soft);
  }
  .deny:hover:not(:disabled) {
    background: rgba(248, 113, 113, 0.22);
    box-shadow: 0 0 14px rgba(248, 113, 113, 0.15);
  }
  .clear:hover:not(:disabled) {
    background: var(--bg-3);
  }
  .hint {
    color: var(--text-3);
    font-size: 0.8rem;
    margin-top: 0.8rem;
  }
  .boundary-note {
    margin-top: 0.9rem;
    background: var(--bg-1);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    padding: 0.55rem 0.7rem;
    color: var(--text-3);
    font-size: 0.72rem;
    line-height: 1.5;
  }
  .boundary-note b {
    display: block;
    color: var(--text-2);
    margin-bottom: 0.15rem;
  }
  .ginote {
    margin-top: 0.9rem;
    background: var(--ask-soft);
    border: 1px solid rgba(251, 191, 36, 0.3);
    border-radius: var(--r-sm);
    padding: 0.55rem 0.7rem;
    font-size: 0.76rem;
    color: var(--text-2);
    line-height: 1.55;
  }
  .ginote b {
    color: var(--ask);
  }
  .ginote code {
    background: var(--bg-1);
    border: 1px solid var(--border);
    padding: 0 0.3rem;
    border-radius: 3px;
    font-size: 0.72rem;
  }
  .gbtn {
    display: block;
    margin-top: 0.5rem;
    background: rgba(251, 191, 36, 0.15);
    border: 1px solid rgba(251, 191, 36, 0.35);
    color: var(--ask);
    border-radius: var(--r-sm);
    padding: 0.3rem 0.7rem;
    cursor: pointer;
    font-size: 0.74rem;
    font-weight: 600;
    transition: background-color var(--t-fast);
  }
  .gbtn:hover {
    background: rgba(251, 191, 36, 0.25);
  }
  .gstat {
    display: block;
    margin-top: 0.4rem;
    color: var(--accent-text);
    font-size: 0.72rem;
  }
</style>
