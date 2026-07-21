<script lang="ts">
  // Structured rule-list editor for a single scope. Unlike PolicyEditor (which is
  // driven by a file-tree selection), this lets you add path/pattern rules by hand —
  // the natural fit for global User settings, which have no project file tree.
  // Rules are shown as a path hierarchy (`~/.ssh/**` nests under `~`), so shared
  // prefixes read as one tree instead of a flat list.
  import { onMount } from 'svelte';
  import type { AppliesTo, Permissions, Policy, PolicyRule, ScopeName } from '$lib/ipc';
  import {
    homeRelativePattern,
    inTauri,
    pickFolder,
    securityBaselineRules,
    toSettingsPreview,
    webBlockSpecifiers
  } from '$lib/ipc';
  import {
    app,
    mergeRules,
    refreshEffective,
    removeRule,
    setExtraDeny,
    upsertRule
  } from '$lib/state.svelte';
  import { describePattern } from '$lib/describe';
  import { tooltip } from '$lib/tooltip';
  import { toggleWebBlockRules, webBlockState } from '$lib/web-block';

  let { scope }: { scope: ScopeName } = $props();

  let newPath = $state('');
  let newApplies = $state<AppliesTo>('pattern');
  let preview = $state<Permissions | null>(null);
  let showPreview = $state(false);
  let status = $state<string | null>(null);
  let error = $state<string | null>(null);
  let webSet = $state<string[]>([]);
  let baselineSet = $state<PolicyRule[]>([]);
  let collapsed = $state(new Set<string>());

  onMount(async () => {
    try {
      webSet = await webBlockSpecifiers();
    } catch {
      /* ignore */
    }
    try {
      if (inTauri()) baselineSet = await securityBaselineRules();
    } catch {
      /* ignore */
    }
  });

  const bucket = $derived(app.scoped[scope]);
  const rules = $derived(bucket.rules);
  const webState = $derived(webBlockState(bucket.extraDeny ?? [], webSet));
  const webBlocked = $derived(webState === 'on');
  const webPartial = $derived(webState === 'partial');

  const counts = $derived({
    allow: rules.filter((r) => r.policy === 'allow').length,
    ask: rules.filter((r) => r.policy === 'ask').length,
    deny: rules.filter((r) => r.policy === 'deny').length
  });

  // The security baseline counts as "applied" when every one of its deny rules exists.
  const baselineApplied = $derived(
    baselineSet.length > 0 &&
      baselineSet.every((ir) => rules.some((r) => r.path === ir.path && r.policy === 'deny'))
  );

  // --- Path hierarchy -------------------------------------------------------
  interface RuleTreeNode {
    name: string;
    full: string;
    children: RuleTreeNode[];
    rule?: PolicyRule;
    recursive?: boolean;
    count: number;
  }

  function splitSegments(path: string): string[] {
    const segs = path.split('/').filter((s) => s !== '');
    // Keep the absolute-root marker on the first segment: //c/Windows → ['//c', 'Windows'].
    if (path.startsWith('//') && segs.length > 0) segs[0] = '//' + segs[0];
    return segs;
  }

  function buildTree(list: PolicyRule[]): RuleTreeNode[] {
    const roots: RuleTreeNode[] = [];
    for (const r of list) {
      const segs = splitSegments(r.path);
      if (segs.length === 0) continue;
      let recursive = false;
      if (segs.length > 1 && segs[segs.length - 1] === '**') {
        segs.pop();
        recursive = true;
      }
      let level = roots;
      let full = '';
      segs.forEach((seg, i) => {
        full = full ? `${full}/${seg}` : seg;
        let node = level.find((n) => n.name === seg && n.full === full);
        if (!node) {
          node = { name: seg, full, children: [], count: 0 };
          level.push(node);
        }
        if (i === segs.length - 1) {
          node.rule = r;
          node.recursive = recursive;
        }
        level = node.children;
      });
    }
    return sortLevel(compressAll(roots)).map(fillCount);
  }

  // Merge single-child chains without rules (~/a/b/c with one rule → one row).
  function compressAll(nodes: RuleTreeNode[]): RuleTreeNode[] {
    return nodes.map((n) => {
      let cur = n;
      while (!cur.rule && cur.children.length === 1) {
        const c = cur.children[0];
        cur = { ...c, name: `${cur.name}/${c.name}` };
      }
      return { ...cur, children: compressAll(cur.children) };
    });
  }

  function sortLevel(nodes: RuleTreeNode[]): RuleTreeNode[] {
    nodes.sort((a, b) => {
      const ga = a.children.length > 0 ? 0 : 1;
      const gb = b.children.length > 0 ? 0 : 1;
      return ga - gb || a.name.localeCompare(b.name);
    });
    nodes.forEach((n) => sortLevel(n.children));
    return nodes;
  }

  function fillCount(n: RuleTreeNode): RuleTreeNode {
    n.children = n.children.map(fillCount);
    n.count = (n.rule ? 1 : 0) + n.children.reduce((s, c) => s + c.count, 0);
    return n;
  }

  const tree = $derived(buildTree(rules));

  function toggleNode(full: string) {
    const next = new Set(collapsed);
    if (next.has(full)) next.delete(full);
    else next.add(full);
    collapsed = next;
  }

  const policies: Policy[] = ['allow', 'ask', 'deny'];
  const previewGroups = $derived(
    preview
      ? (
          [
            ['allow', preview.allow],
            ['ask', preview.ask],
            ['deny', preview.deny]
          ] as [Policy, string[]][]
        ).filter(([, arr]) => arr.length > 0)
      : []
  );

  const appliesLabels: Record<AppliesTo, string> = {
    file: '이 파일만',
    folder: '이 폴더만',
    'folder-and-children': '폴더 + 하위',
    pattern: '패턴'
  };

  const appliesTips: Record<AppliesTo, string> = {
    file: '이 파일 하나에만 적용됩니다',
    folder: '이 폴더 자체에만 적용됩니다 (하위 항목 제외)',
    'folder-and-children': '이 폴더와 그 안의 모든 하위 항목에 적용됩니다',
    pattern: '입력한 경로 패턴 그대로 적용됩니다'
  };

  // Tooltip for a tree row: rule rows explain the full pattern, group rows the prefix.
  function nodeTip(n: RuleTreeNode): string | undefined {
    const tip = n.rule
      ? describePattern(n.full + (n.recursive ? '/**' : ''))
      : describePattern(n.full, { group: true });
    return tip || undefined;
  }

  async function add(policy: Policy) {
    const path = newPath.trim();
    if (!path) return;
    upsertRule(scope, { path, policy, appliesTo: newApplies });
    newPath = '';
    await refreshEffective();
  }

  async function changePolicy(rule: PolicyRule, policy: Policy) {
    upsertRule(scope, { ...rule, policy });
    await refreshEffective();
  }

  async function remove(path: string) {
    removeRule(scope, path);
    await refreshEffective();
  }

  // Pick a folder anywhere on the PC and add it directly as an Allow (work folder) or
  // Deny (sensitive folder) rule — the explicit "designate folders" workflow. The path
  // is converted to the correct global pattern (`//c/…/**` or `~/…/**`).
  async function pickFolderAs(policy: Policy) {
    error = null;
    status = null;
    if (!inTauri()) {
      error = '데스크톱 앱에서만 폴더를 선택할 수 있습니다.';
      return;
    }
    const dir = await pickFolder();
    if (!dir) return;
    const pat = await homeRelativePattern(dir);
    upsertRule(scope, { path: pat, policy, appliesTo: 'pattern' });
    await refreshEffective();
    status = `${policy === 'allow' ? '작업' : '민감'} 폴더 ${policy === 'allow' ? 'Allow' : 'Deny'} 추가: ${pat}`;
  }

  // Toggle the web-tool/common-client block. This is not an OS-level firewall.
  function toggleWebBlock() {
    const wasBlocked = webBlocked;
    setExtraDeny(scope, toggleWebBlockRules(bucket.extraDeny ?? [], webSet));
    status = wasBlocked
      ? '웹 접근 제한을 해제했습니다.'
      : '웹 도구와 대표 HTTP 클라이언트 차단을 추가했습니다 (저장 시 적용).';
  }

  // Apply the security baseline Deny rules (SSH/cloud creds, keys, .env, …).
  async function applyBaseline() {
    error = null;
    try {
      const rules = await securityBaselineRules();
      mergeRules(scope, rules);
      await refreshEffective();
      status = `보안 베이스라인 Deny ${rules.length}개를 추가했습니다. 저장 전에 검토하세요.`;
    } catch (e) {
      error = String(e);
    }
  }

  // Live preview of the Tool(specifier) strings these rules emit. Tauri-only.
  $effect(() => {
    const snapshot = $state.snapshot(rules) as PolicyRule[];
    if (!showPreview || !inTauri()) {
      preview = null;
      return;
    }
    toSettingsPreview(snapshot)
      .then((p) => (preview = p))
      .catch(() => (preview = null));
  });
</script>

{#snippet ruleTree(nodes: RuleTreeNode[])}
  {#each nodes as n (n.full)}
    <div class="tnode">
      <div class="trow" class:has-rule={!!n.rule}>
        {#if n.children.length}
          <button
            class="twist"
            class:open={!collapsed.has(n.full)}
            aria-label={collapsed.has(n.full) ? '펼치기' : '접기'}
            onclick={() => toggleNode(n.full)}
          >
            <svg viewBox="0 0 16 16" width="11" height="11" fill="none" aria-hidden="true">
              <path d="m6 4 4 4-4 4" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
          </button>
        {:else}
          <span class="twist spacer" aria-hidden="true"></span>
        {/if}

        <span class="tlabel" use:tooltip={nodeTip(n)}>
          <span class="tname" class:group={!n.rule}>
            {n.name}{#if n.recursive}<span class="rec">/**</span>{/if}
          </span>
        </span>

        {#if n.rule}
          <span class="tmeta" use:tooltip={appliesTips[n.rule.appliesTo]}>
            {appliesLabels[n.rule.appliesTo]}{#if n.rule.tools?.length} · {n.rule.tools.join(', ')}{/if}
          </span>
          <span class="tactions">
            <span class="seg">
              {#each policies as p (p)}
                <button class={p} class:active={n.rule.policy === p} onclick={() => changePolicy(n.rule!, p)}>
                  {p}
                </button>
              {/each}
            </span>
            <button class="del" title="규칙 삭제" aria-label="규칙 삭제" onclick={() => remove(n.rule!.path)}>✕</button>
          </span>
        {:else if collapsed.has(n.full)}
          <span class="gcount">{n.count}</span>
        {/if}
      </div>

      {#if n.children.length && !collapsed.has(n.full)}
        <div class="tchildren">
          {@render ruleTree(n.children)}
        </div>
      {/if}
    </div>
  {/each}
{/snippet}

<div class="panel">
  <div class="head">
    <h3>규칙 목록 <span class="scope">{scope}</span></h3>
  </div>

  <div class="overview" aria-label="설정 현황">
    <span class="ov" class:on={counts.allow > 0}>
      <span class="ov-dot d-allow" aria-hidden="true"></span>Allow <b>{counts.allow}</b>
    </span>
    <span class="ov" class:on={counts.ask > 0}>
      <span class="ov-dot d-ask" aria-hidden="true"></span>Ask <b>{counts.ask}</b>
    </span>
    <span class="ov" class:on={counts.deny > 0}>
      <span class="ov-dot d-deny" aria-hidden="true"></span>Deny <b>{counts.deny}</b>
    </span>
    <span class="ov-sep" aria-hidden="true"></span>
    <button
      class="ov ov-toggle"
      class:on={webBlocked}
      onclick={toggleWebBlock}
      title="WebSearch·WebFetch와 Bash/PowerShell의 대표 HTTP 클라이언트 차단 — OS 방화벽은 아님"
    >
      <span class="ov-dot" class:d-guard={webBlocked} aria-hidden="true"></span>
      웹 접근 제한 <b>{webBlocked ? 'ON' : webPartial ? 'PARTIAL' : 'OFF'}</b>
    </button>
  </div>

  <div class="toolbar">
    <button class="wf" onclick={() => pickFolderAs('allow')} title="작업 폴더를 Allow로 지정">
      작업 폴더 (Allow)
    </button>
    <button class="sf" onclick={() => pickFolderAs('deny')} title="민감 정보 폴더를 Deny로 지정">
      민감 폴더 (Deny)
    </button>
    <button
      class="rec-btn"
      class:applied={baselineApplied}
      onclick={applyBaseline}
      disabled={baselineApplied}
      title="민감 자격증명/키/.env/시스템 경로를 일괄 Deny"
    >
      {baselineApplied ? '✓ 보안 베이스라인 추가됨' : '보안 베이스라인 추가'}
    </button>
  </div>

  {#if status}<p class="status" role="status">{status}</p>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}

  <div class="add">
    <input
      class="path"
      placeholder="패턴 (예: ~/.ssh/**, //c/Windows/**, //**/*.key)"
      bind:value={newPath}
      onkeydown={(e) => e.key === 'Enter' && add('deny')}
    />
    <select bind:value={newApplies} title="경로 해석 방식">
      <option value="pattern">패턴 (그대로)</option>
      <option value="file">이 파일만</option>
      <option value="folder">이 폴더만</option>
      <option value="folder-and-children">폴더 + 하위</option>
    </select>
    <div class="addbtns">
      <button class="allow" onclick={() => add('allow')} disabled={!newPath.trim()}>+ Allow</button>
      <button class="ask" onclick={() => add('ask')} disabled={!newPath.trim()}>+ Ask</button>
      <button class="deny" onclick={() => add('deny')} disabled={!newPath.trim()}>+ Deny</button>
    </div>
  </div>
  <p class="tip">
    <strong>작업 폴더(Allow)</strong> / <strong>민감 폴더(Deny)</strong> 버튼으로 폴더를 명시하세요
    — 경로는 올바른 전역 패턴으로 자동 변환됩니다 (<code>C:\Windows</code> → <code>//c/Windows</code>).
    직접 입력도 가능: 홈 <code>~/.aws/**</code>, 절대·전 드라이브 <code>//**/*.key</code>.
  </p>

  {#if rules.length === 0}
    <p class="empty">규칙이 없습니다. 위에서 경로/패턴을 추가하세요.</p>
  {:else}
    <div class="tree">
      {@render ruleTree(tree)}
    </div>
  {/if}

  <button class="pv-toggle" class:open={showPreview} onclick={() => (showPreview = !showPreview)}>
    <svg viewBox="0 0 16 16" width="11" height="11" fill="none" aria-hidden="true">
      <path d="m6 4 4 4-4 4" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
    </svg>
    생성될 permission 미리보기
  </button>
  {#if showPreview}
    {#if preview}
      <div class="pv">
        {#each previewGroups as [k, arr] (k)}
          <div class="pv-group">
            <span class="pv-k {k}">{k}</span>
            <ul>{#each arr as s (s)}<li>{s}</li>{/each}</ul>
          </div>
        {/each}
        {#if previewGroups.length === 0}
          <p class="empty">생성될 규칙이 없습니다.</p>
        {/if}
      </div>
    {:else}
      <p class="empty">{inTauri() ? '계산 중…' : '데스크톱 앱에서만 미리보기가 표시됩니다.'}</p>
    {/if}
  {/if}
</div>

<style>
  .panel {
    padding: 0.85rem;
    overflow: auto;
    height: 100%;
    box-sizing: border-box;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }
  h3 {
    margin: 0 0 0.5rem;
    font-size: 0.92rem;
    letter-spacing: -0.01em;
    display: flex;
    align-items: center;
    gap: 0.45rem;
  }
  .scope {
    font-size: 0.68rem;
    font-weight: 600;
    color: var(--accent-text);
    text-transform: capitalize;
    background: var(--accent-soft);
    border: 1px solid rgba(79, 142, 247, 0.3);
    border-radius: 999px;
    padding: 0.08rem 0.55rem;
  }

  /* 설정 현황 — what is configured, at a glance. */
  .overview {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    flex-wrap: wrap;
    background: var(--bg-1);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    padding: 0.5rem 0.75rem;
    margin-bottom: 0.6rem;
  }
  .ov {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.75rem;
    color: var(--text-3);
    font-variant-numeric: tabular-nums;
  }
  .ov b {
    font-weight: 700;
    color: inherit;
  }
  .ov.on {
    color: var(--text-1);
  }
  .ov-toggle {
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    font-family: inherit;
  }
  .ov-toggle:hover {
    color: var(--accent-text);
  }
  .ov-dot {
    width: 7px;
    height: 7px;
    border-radius: 999px;
    border: 1px solid var(--border-strong);
    background: transparent;
    flex-shrink: 0;
  }
  .ov.on .ov-dot.d-allow {
    background: var(--allow);
    border-color: var(--allow);
    box-shadow: 0 0 6px rgba(52, 211, 153, 0.6);
  }
  .ov.on .ov-dot.d-ask {
    background: var(--ask);
    border-color: var(--ask);
    box-shadow: 0 0 6px rgba(251, 191, 36, 0.6);
  }
  .ov.on .ov-dot.d-deny {
    background: var(--deny);
    border-color: var(--deny);
    box-shadow: 0 0 6px rgba(248, 113, 113, 0.6);
  }
  /* Protection toggles read green when active: ON = safer, not "danger". */
  .ov.on .ov-dot.d-guard {
    background: var(--allow);
    border-color: var(--allow);
    box-shadow: 0 0 6px rgba(52, 211, 153, 0.6);
  }
  .ov.on:has(.d-guard) {
    color: var(--allow);
  }
  .ov-sep {
    width: 1px;
    height: 0.9rem;
    background: var(--border);
  }

  .toolbar {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
  }
  .toolbar button {
    padding: 0.4rem 0.75rem;
    border-radius: var(--r-sm);
    cursor: pointer;
    font-size: 0.78rem;
    font-weight: 600;
    transition: background-color var(--t-fast), box-shadow var(--t-fast);
  }
  .toolbar .wf {
    background: var(--allow-soft);
    border: 1px solid rgba(52, 211, 153, 0.35);
    color: var(--allow);
  }
  .toolbar .wf:hover {
    background: rgba(52, 211, 153, 0.22);
  }
  .toolbar .sf {
    background: var(--deny-soft);
    border: 1px solid rgba(248, 113, 113, 0.35);
    color: var(--deny);
  }
  .toolbar .sf:hover {
    background: rgba(248, 113, 113, 0.22);
  }
  .toolbar .rec-btn {
    background: var(--deny-soft);
    border: 1px solid rgba(248, 113, 113, 0.35);
    color: var(--deny);
  }
  .toolbar .rec-btn:hover:not(:disabled) {
    background: rgba(248, 113, 113, 0.22);
  }
  .toolbar .rec-btn.applied {
    background: var(--allow-soft);
    border-color: rgba(52, 211, 153, 0.35);
    color: var(--allow);
    cursor: default;
  }

  .status {
    color: var(--accent-text);
    background: var(--accent-soft);
    border-radius: var(--r-sm);
    padding: 0.35rem 0.6rem;
    font-size: 0.76rem;
    margin: 0.5rem 0 0;
  }
  .error {
    color: var(--deny);
    background: var(--deny-soft);
    border-radius: var(--r-sm);
    padding: 0.35rem 0.6rem;
    font-size: 0.76rem;
    margin: 0.5rem 0 0;
  }
  .add {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0.4rem;
    align-items: center;
    margin-top: 0.6rem;
  }
  .path {
    padding: 0.5rem 0.6rem;
    background: var(--bg-1);
    color: var(--text-1);
    border: 1px solid var(--border-strong);
    border-radius: var(--r-sm);
    font-size: 0.82rem;
    font-family: var(--font-mono);
    transition: border-color var(--t-fast);
  }
  .path::placeholder {
    color: var(--text-3);
    font-family: var(--font-sans);
  }
  .path:focus {
    border-color: var(--accent);
    outline: none;
  }
  select {
    padding: 0.45rem;
    background: var(--bg-1);
    color: var(--text-1);
    border: 1px solid var(--border-strong);
    border-radius: var(--r-sm);
    font-size: 0.78rem;
  }
  .addbtns {
    grid-column: 1 / -1;
    display: flex;
    gap: 0.4rem;
  }
  .addbtns button {
    flex: 1;
    padding: 0.42rem;
    border-radius: var(--r-sm);
    cursor: pointer;
    font-size: 0.8rem;
    font-weight: 600;
    transition: background-color var(--t-fast);
  }
  .addbtns button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .addbtns .allow {
    background: var(--allow-soft);
    border: 1px solid rgba(52, 211, 153, 0.35);
    color: var(--allow);
  }
  .addbtns .allow:hover:not(:disabled) {
    background: rgba(52, 211, 153, 0.22);
  }
  .addbtns .ask {
    background: var(--ask-soft);
    border: 1px solid rgba(251, 191, 36, 0.35);
    color: var(--ask);
  }
  .addbtns .ask:hover:not(:disabled) {
    background: rgba(251, 191, 36, 0.22);
  }
  .addbtns .deny {
    background: var(--deny-soft);
    border: 1px solid rgba(248, 113, 113, 0.35);
    color: var(--deny);
  }
  .addbtns .deny:hover:not(:disabled) {
    background: rgba(248, 113, 113, 0.22);
  }
  .tip {
    color: var(--text-3);
    font-size: 0.74rem;
    margin: 0.55rem 0;
    line-height: 1.5;
  }
  .tip code {
    background: var(--bg-1);
    border: 1px solid var(--border);
    padding: 0 0.28rem;
    border-radius: 3px;
  }
  .empty {
    color: var(--text-3);
    font-size: 0.82rem;
  }

  /* Rule hierarchy tree. */
  .tree {
    border: 1px solid var(--border);
    background: linear-gradient(180deg, var(--bg-2), var(--bg-1));
    border-radius: var(--r-md);
    padding: 0.45rem 0.55rem;
    margin: 0.3rem 0;
  }
  .trow {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.14rem 0.3rem;
    border-radius: var(--r-sm);
    min-width: 0;
    transition: background-color var(--t-fast);
  }
  .trow:hover {
    background: var(--bg-3);
  }
  .twist {
    display: grid;
    place-items: center;
    width: 1.05rem;
    height: 1.05rem;
    color: var(--text-3);
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    flex-shrink: 0;
    transition: transform var(--t-fast), color var(--t-fast);
  }
  .twist:hover {
    color: var(--text-1);
  }
  .twist.open {
    transform: rotate(90deg);
  }
  .twist.spacer {
    visibility: hidden;
  }
  .tlabel {
    display: inline-flex;
    min-width: 0;
  }
  .tname {
    font-family: var(--font-mono);
    font-size: 0.8rem;
    color: var(--text-1);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tname.group {
    color: var(--text-2);
    font-weight: 600;
  }
  .rec {
    color: var(--text-3);
  }
  .tmeta {
    font-size: 0.66rem;
    color: var(--text-3);
    white-space: nowrap;
    flex-shrink: 0;
  }
  .gcount {
    margin-left: auto;
    font-size: 0.66rem;
    color: var(--text-3);
    background: var(--bg-1);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0.02rem 0.45rem;
    font-variant-numeric: tabular-nums;
  }
  .tactions {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 0.35rem;
    flex-shrink: 0;
  }
  .tchildren {
    margin-left: 0.85rem;
    border-left: 1px solid var(--border);
    padding-left: 0.4rem;
  }
  .seg {
    display: flex;
    background: var(--bg-1);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    padding: 0.12rem;
    gap: 0.12rem;
  }
  .seg button {
    padding: 0.13rem 0.45rem;
    background: transparent;
    border: none;
    color: var(--text-3);
    cursor: pointer;
    font-size: 0.68rem;
    font-weight: 600;
    text-transform: capitalize;
    border-radius: 4px;
    transition: background-color var(--t-fast), color var(--t-fast);
  }
  .seg button:hover {
    color: var(--text-1);
  }
  .seg button.allow.active {
    background: var(--allow-soft);
    color: var(--allow);
  }
  .seg button.ask.active {
    background: var(--ask-soft);
    color: var(--ask);
  }
  .seg button.deny.active {
    background: var(--deny-soft);
    color: var(--deny);
  }
  .del {
    background: none;
    border: 1px solid transparent;
    color: var(--text-3);
    border-radius: var(--r-sm);
    padding: 0.12rem 0.4rem;
    cursor: pointer;
    transition: color var(--t-fast), border-color var(--t-fast), background-color var(--t-fast);
  }
  .del:hover {
    border-color: rgba(248, 113, 113, 0.4);
    background: var(--deny-soft);
    color: var(--deny);
  }

  .pv-toggle {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    margin-top: 0.7rem;
    background: none;
    border: none;
    color: var(--accent-text);
    cursor: pointer;
    font-size: 0.78rem;
    padding: 0.2rem 0;
  }
  .pv-toggle svg {
    transition: transform var(--t-fast);
  }
  .pv-toggle.open svg {
    transform: rotate(90deg);
  }
  .pv {
    background: var(--bg-1);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    padding: 0.6rem;
    margin-top: 0.4rem;
  }
  .pv-group {
    margin-bottom: 0.45rem;
  }
  .pv-k {
    font-size: 0.66rem;
    font-weight: 700;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    padding: 0.08rem 0.45rem;
    border-radius: 4px;
  }
  .pv-k.allow {
    background: var(--allow-soft);
    color: var(--allow);
  }
  .pv-k.ask {
    background: var(--ask-soft);
    color: var(--ask);
  }
  .pv-k.deny {
    background: var(--deny-soft);
    color: var(--deny);
  }
  .pv ul {
    list-style: none;
    padding: 0.3rem 0 0;
    margin: 0;
  }
  .pv li {
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--text-2);
    word-break: break-all;
    padding: 0.06rem 0;
  }
</style>
