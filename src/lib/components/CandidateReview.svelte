<script lang="ts">
  import type { Policy, PolicyRule, ScopeName } from '$lib/ipc';
  import { modalFocus } from '$lib/modal';

  let { candidates, existing, scope, onapply, onclose }: {
    candidates: PolicyRule[];
    existing: PolicyRule[];
    scope: ScopeName;
    onapply: (rules: PolicyRule[], replacements: string[]) => void;
    onclose: () => void;
  } = $props();

  // Detection does not establish an appropriate permission. Every choice is opt-in.
  let choices = $state<{ rule: PolicyRule; selected: boolean; policy: Policy; replace: boolean; tools: 'Read' | 'Edit' | 'both' }[]>([]);
  $effect(() => {
    choices = candidates.map((rule) => ({ rule, selected: false, policy: 'ask', replace: false, tools: 'Read' }));
  });
  const selectedCount = $derived(choices.filter((item) => item.selected &&
    (!existing.some((rule) => rule.path === item.rule.path) || item.replace)).length);

  function apply() {
    const selected = choices.filter((item) => item.selected &&
      (!existing.some((rule) => rule.path === item.rule.path) || item.replace));
    onapply(selected.map((item) => ({ ...item.rule, policy: item.policy,
      tools: item.tools === 'both' ? null : [item.tools] })),
      selected.filter((item) => item.replace).map((item) => item.rule.path));
  }
</script>

<div class="modal-bg" role="presentation" onclick={(event) => { if (event.target === event.currentTarget) onclose(); }}>
  <div class="modal review" role="dialog" aria-modal="true" aria-labelledby="candidate-review-title" tabindex="-1" use:modalFocus={onclose}>
    <h3 id="candidate-review-title">경로 후보 검토 — {scope}</h3>
    <p class="lead">패턴과 일치하는 경로입니다. 이름만으로 민감 여부나 적절한 권한을 판단할 수 없습니다.
      항목·도구·권한을 직접 선택하세요. 선택한 규칙도 저장 전까지 적용되지 않습니다.</p>
    <div class="items">
      {#each choices as item (item.rule.path)}
        {@const previous = existing.filter((rule) => rule.path === item.rule.path)}
        <div class="item" class:on={item.selected}>
          <label class="choose"><input type="checkbox" bind:checked={item.selected} /> <code>{item.rule.path}</code></label>
          <p>{item.rule.reason ?? '자격증명·설정 파일에 자주 쓰이는 경로 패턴입니다. 공개 예제·개발용 파일일 수도 있습니다.'}</p>
          <div class="options">
            <label>도구 <select class="input" bind:value={item.tools} disabled={!item.selected}>
              <option value="Read">Read</option><option value="Edit">Edit</option><option value="both">Read + Edit</option>
            </select></label>
            <label>권한 <select class="input" bind:value={item.policy} disabled={!item.selected}>
              <option value="ask">Ask</option><option value="deny">Deny</option><option value="allow">Allow</option>
            </select></label>
          </div>
          {#if item.selected && item.policy === 'allow'}<p class="callout callout-warn">Allow는 이 도구의 확인 절차를 줄입니다. 수정 권한이 필요한지도 확인하세요.</p>{/if}
          {#if previous.length}
            <p class="callout callout-warn">기존 규칙: {previous.map((rule) => `${rule.policy[0].toUpperCase()}${rule.policy.slice(1)} (${rule.tools?.join(', ') ?? 'Read, Edit'})`).join(' / ')} — 기본적으로 보존합니다.</p>
            <label class="replace"><input type="checkbox" bind:checked={item.replace} disabled={!item.selected} /> 이 경로의 기존 규칙 전체를 선택한 규칙으로 교체</label>
          {/if}
        </div>
      {:else}<p class="muted">표시할 후보가 없습니다.</p>{/each}
    </div>
    <div class="modal-actions">
      <button data-modal-initial onclick={onclose}>취소</button>
      <button class="primary" onclick={apply} disabled={selectedCount === 0}>선택한 {selectedCount}개를 편집에 반영</button>
    </div>
  </div>
</div>

<style>
  .review { width: min(720px, 95vw); display: flex; flex-direction: column; overflow: hidden; }
  .lead, .item p:not(.callout) { font-size: var(--fs-md); color: var(--text-2); line-height: 1.6; margin: 0.4rem 0; }
  .items { overflow-y: auto; min-height: 0; }
  .item { padding: 0.8rem 0; border-top: 1px solid var(--border); }
  .item .callout { margin: 0.45rem 0; font-size: var(--fs-sm); }
  .choose { display: flex; align-items: center; gap: 0.4rem; font-weight: 600; min-width: 0; }
  .choose code { overflow-wrap: anywhere; }
  label { font-size: var(--fs-md); }
  .options { display: flex; flex-wrap: wrap; gap: 1rem; }
  .options label { display: inline-flex; align-items: center; gap: 0.4rem; white-space: nowrap; }
  .options .input { width: auto; padding-block: 0.25rem; }
  .replace { display: flex; align-items: center; gap: 0.4rem; }
</style>
