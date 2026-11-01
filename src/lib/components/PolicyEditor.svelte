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
  import { toast } from '$lib/toast.svelte';

  let appliesTo = $state<AppliesTo>('folder-and-children');
  let reason = $state('');
  let notes = $state('');
  let riskLevel = $state<'' | 'low' | 'medium' | 'high'>('');
  let useRead = $state(true);
  let useEdit = $state(true);

  // Git-ignored paths need a heads-up: an Allow rule opens access, but the
  // agent's Grep search still skips them.
  let ignored = $state(false);
  let noting = $state(false);

  $effect(() => {
    const p = app.selectedPath;
    ignored = false;
    if (!p || !inTauri()) return;
    pathIgnored(app.projectRoot, p)
      .then((v) => (ignored = v))
      .catch(() => {});
  });

  async function noteInClaudeMd() {
    noting = true;
    try {
      const added = await noteIgnoredPath(app.projectRoot, app.selectedPath);
      toast(
        added
          ? 'CLAUDE.md에 안내를 추가했습니다 — 에이전트가 이 경로를 인지합니다.'
          : 'CLAUDE.md에 이미 안내가 있습니다.',
        added ? 'ok' : 'info'
      );
    } catch (e) {
      toast(String(e), 'error');
    } finally {
      noting = false;
    }
  }

  // The explicit rule currently on this path in the active scope, if any.
  const current = $derived(
    app.scoped[app.activeScope].rules.find((r) => r.path === app.selectedPath)
  );
  const hasSavedDetails = $derived(!!(current?.reason || current?.notes || current?.riskLevel));
  const noTool = $derived(!useRead && !useEdit);

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
    if (!app.selectedPath || noTool) return;
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
    if (!app.selectedPath || !current) return;
    const removed = $state.snapshot(current);
    const scope = app.activeScope;
    clearPolicy(app.selectedPath);
    await refreshEffective();
    toast(`규칙을 지웠습니다 — ${removed.path}`, 'info', {
      label: '되돌리기',
      run: () => {
        upsertRule(scope, removed);
        refreshEffective();
      }
    });
  }

  const scopes: { id: ScopeName; label: string }[] = [
    { id: 'project', label: '프로젝트' },
    { id: 'local', label: '로컬' }
  ];

  const decisions: { policy: Policy; label: string; key: string }[] = [
    { policy: 'allow', label: 'Allow', key: 'A' },
    { policy: 'ask', label: 'Ask', key: 'K' },
    { policy: 'deny', label: 'Deny', key: 'D' }
  ];
</script>

<div class="panel">
  <h3 class="label-caps">정책 편집</h3>

  {#if !app.selectedPath}
    <div class="empty">
      <p>왼쪽 트리에서 파일이나 폴더를 선택하세요.</p>
      <p class="muted">선택한 경로에 Allow / Ask / Deny 규칙을 지정할 수 있습니다. 단축키 A · K · D.</p>
    </div>
  {:else}
    <div class="head">
      <span class="label-caps">경로</span>
      <code class="target">{app.selectedPath}</code>
      <div class="status">
        <span class="label-caps">현재 상태(이 범위)</span>
        {#if current}
          <span class="chip chip-{current.policy}">{current.policy[0].toUpperCase() + current.policy.slice(1)}</span>
        {:else}
          <span class="chip chip-none">추적 안 됨</span>
        {/if}
      </div>
    </div>

    <div class="decide" role="group" aria-label="정책 결정">
      {#each decisions as d (d.policy)}
        <button
          class="btn decision d-{d.policy}"
          aria-pressed={current?.policy === d.policy}
          aria-keyshortcuts={d.key}
          onclick={() => apply(d.policy)}
          disabled={noTool}
        >
          {d.label} <kbd>{d.key}</kbd>
        </button>
      {/each}
    </div>
    <button class="btn btn-ghost btn-sm clear" onclick={clear} disabled={!current}>규칙 지우기</button>

    <div class="field">
      <span class="label-caps" id="pe-scope">범위</span>
      <div class="segmented" role="group" aria-labelledby="pe-scope">
        {#each scopes as s (s.id)}
          <button aria-pressed={app.activeScope === s.id} onclick={() => (app.activeScope = s.id)}>{s.label}</button>
        {/each}
      </div>
    </div>

    <label class="field">
      <span class="label-caps">적용 대상</span>
      <select class="input" bind:value={appliesTo}>
        <option value="file">이 파일만</option>
        <option value="folder">이 폴더만</option>
        <option value="folder-and-children">이 폴더와 하위</option>
        <option value="pattern">패턴 일치</option>
      </select>
    </label>

    <fieldset class="field tools-field">
      <legend class="label-caps">적용 도구</legend>
      <label><input type="checkbox" bind:checked={useRead} /> Read</label>
      <label><input type="checkbox" bind:checked={useEdit} /> Edit</label>
      {#if noTool}<small role="alert">도구를 하나 이상 선택하세요.</small>{/if}
    </fieldset>

    <details class="more" open={hasSavedDetails}>
      <summary>세부 정보 (선택)</summary>
      <label class="field">
        <span class="label-caps">사유</span>
        <input class="input" bind:value={reason} placeholder="예: 소스 코드 작업에 필요" />
      </label>
      <label class="field">
        <span class="label-caps">위험도</span>
        <select class="input" bind:value={riskLevel}>
          <option value="">지정 안 함</option>
          <option value="low">낮음</option>
          <option value="medium">중간</option>
          <option value="high">높음</option>
        </select>
      </label>
      <label class="field">
        <span class="label-caps">메모</span>
        <textarea class="input" bind:value={notes} rows="2" placeholder="팀 또는 개인 참고 사항"></textarea>
      </label>
      <p class="muted small">세부 정보는 Allow / Ask / Deny 를 다시 누르면 규칙에 반영됩니다.</p>
    </details>

    {#if ignored}
      <div class="callout callout-warn ginote">
        <b>.gitignore에 포함된 경로입니다.</b> 차단된 것은 아닙니다 —
        Grep 검색으로는 <b>발견 못 함</b> · 경로를 알면 Read <b>가능</b> · 접근 권한은
        Allow/Ask/Deny 규칙이 결정. CLAUDE.md에 경로를 알려주면 에이전트가 직접 읽거나
        <code>rg --no-ignore</code>로 검색할 수 있습니다.
        <button class="btn btn-sm gbtn" onclick={noteInClaudeMd} disabled={noting}>
          {noting ? '추가 중…' : 'CLAUDE.md에 알리기'}
        </button>
      </div>
    {/if}
  {/if}

  <div class="boundary-note">
    <b>권한 경계</b>
    Allow는 일치한 호출을 미리 승인할 뿐, 미등록 접근을 차단하는 화이트리스트가 아닙니다.
    Read/Edit Deny는 Claude의 내장 파일 도구에는 적용되지만, 허용된 Bash·PowerShell 프로세스의
    OS 파일 접근까지 차단하지는 않습니다.
  </div>
</div>

<style>
  .panel {
    padding: 0.85rem;
    min-width: 0;
  }
  h3 {
    margin: 0 0 0.75rem;
  }
  .empty {
    padding: 1rem 0.2rem;
    color: var(--text-2);
    font-size: var(--fs-md);
  }
  .empty p {
    margin: 0 0 0.4rem;
  }
  .head {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    margin-bottom: 0.7rem;
  }
  .target {
    background: var(--bg-1);
    border: 1px solid var(--border);
    padding: 0.3rem 0.5rem;
    border-radius: var(--r-sm);
    font-size: var(--fs-md);
    display: block;
    overflow-wrap: anywhere;
    color: var(--text-1);
  }
  .status {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .chip-none {
    color: var(--text-3);
    border-style: dashed;
    background: transparent;
  }
  .decide {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.4rem;
  }
  .decision {
    font-weight: 600;
    padding: 0.5rem 0.4rem;
    min-width: 0;
  }
  .decision kbd {
    font-size: 0.62rem;
    padding: 0 0.3rem;
    opacity: 0.7;
  }
  .d-allow {
    color: var(--allow);
    background: var(--allow-soft);
    border-color: var(--allow-line);
  }
  .d-allow:hover:not(:disabled) {
    background: var(--allow-soft-hover);
  }
  .d-ask {
    color: var(--ask);
    background: var(--ask-soft);
    border-color: var(--ask-line);
  }
  .d-ask:hover:not(:disabled) {
    background: var(--ask-soft-hover);
  }
  .d-deny {
    color: var(--deny);
    background: var(--deny-soft);
    border-color: var(--deny-line);
  }
  .d-deny:hover:not(:disabled) {
    background: var(--deny-soft-hover);
  }
  .d-allow[aria-pressed='true'] {
    background: var(--allow);
    border-color: var(--allow);
    color: var(--on-policy);
  }
  .d-ask[aria-pressed='true'] {
    background: var(--ask);
    border-color: var(--ask);
    color: var(--on-policy);
  }
  .d-deny[aria-pressed='true'] {
    background: var(--deny);
    border-color: var(--deny);
    color: var(--on-policy);
  }
  .clear {
    margin: 0.35rem 0 0.9rem auto;
    display: flex;
  }
  .field {
    display: block;
    margin-bottom: 0.8rem;
    min-width: 0;
  }
  .field > .label-caps {
    display: block;
    margin-bottom: 0.3rem;
  }
  textarea.input {
    resize: vertical;
    font-family: inherit;
  }
  .tools-field {
    border: 0;
    margin-inline: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.8rem;
  }
  .tools-field legend {
    padding: 0;
    margin-bottom: 0.3rem;
  }
  .tools-field label {
    color: var(--text-2);
    font-size: var(--fs-sm);
    white-space: nowrap;
  }
  .tools-field small {
    color: var(--deny);
  }
  .more {
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    padding: 0.4rem 0.6rem;
    margin-bottom: 0.8rem;
  }
  .more summary {
    cursor: pointer;
    color: var(--text-2);
    font-size: var(--fs-sm);
    padding: 0.15rem 0;
  }
  .more[open] summary {
    margin-bottom: 0.6rem;
  }
  .small {
    font-size: var(--fs-xs);
    margin: 0;
  }
  .boundary-note {
    margin-top: 0.9rem;
    background: var(--bg-1);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    padding: 0.55rem 0.7rem;
    color: var(--text-3);
    font-size: var(--fs-xs);
    line-height: 1.5;
  }
  .boundary-note b {
    display: block;
    color: var(--text-2);
    margin-bottom: 0.15rem;
  }
  .ginote {
    margin-top: 0.9rem;
    font-size: var(--fs-sm);
    color: var(--text-2);
  }
  .ginote b {
    color: var(--ask);
  }
  .gbtn {
    display: flex;
    margin-top: 0.5rem;
    color: var(--ask);
    border-color: var(--ask-line);
    background: var(--ask-soft);
  }
  .gbtn:hover:not(:disabled) {
    background: var(--ask-soft-hover);
  }
</style>
