<script lang="ts">
  // Raw config editor for a single agent global file (JSON or TOML). Validates by
  // format before saving, shows a pre-save diff, and always backs up first. Used for
  // agents without a structured editor (Codex, OpenCode); Claude Code uses /user.
  import type { AgentGlobal, DiffView } from '$lib/ipc';
  import {
    inTauri,
    intranetRecommendation,
    readAgentConfig,
    saveAgentConfig,
    validateConfig
  } from '$lib/ipc';
  import DiffViewer from '$lib/components/DiffViewer.svelte';

  let { agent }: { agent: AgentGlobal } = $props();

  let text = $state('');
  let onDisk = $state('');
  let loading = $state(false);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let status = $state<string | null>(null);
  let diff = $state<DiffView | null>(null);

  async function load() {
    if (!inTauri()) return;
    loading = true;
    error = null;
    status = null;
    try {
      onDisk = await readAgentConfig(agent.path);
      text = onDisk;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  // Reload when the agent (path) changes.
  $effect(() => {
    agent.path;
    load();
  });

  async function validate() {
    error = await validateConfig(text, agent.format);
    status = error ? null : '유효합니다.';
  }

  function format() {
    if (agent.format !== 'json') {
      status = 'TOML은 자동 포맷을 지원하지 않습니다.';
      return;
    }
    try {
      text = JSON.stringify(JSON.parse(text || '{}'), null, 2) + '\n';
      error = null;
      status = '포맷 완료';
    } catch (e) {
      error = String(e);
    }
  }

  async function openSaveDialog() {
    error = null;
    status = null;
    const v = await validateConfig(text, agent.format);
    if (v) {
      error = v;
      return;
    }
    diff = { path: agent.path, before: onDisk, after: text, changed: onDisk !== text };
  }

  // Merge this agent's intranet security baseline into the editor for review.
  async function applyIntranet() {
    error = null;
    try {
      text = await intranetRecommendation(agent.id, text);
      status = '인트라넷 추천 보안 셋을 적용했습니다. 검토 후 저장하세요.';
    } catch (e) {
      error = String(e);
    }
  }

  async function confirmSave() {
    if (!diff) return;
    saving = true;
    try {
      const res = await saveAgentConfig({
        path: agent.path,
        text,
        format: agent.format,
        agentId: agent.id
      });
      onDisk = text;
      status = `저장됨 → ${res.written}${res.backup ? ` (백업: ${res.backup})` : ''}`;
      diff = null;
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  const dirty = $derived(text !== onDisk);
</script>

<div class="panel">
  <div class="bar">
    <span class="fmt">{agent.format.toUpperCase()}</span>
    <div class="tools">
      <button class="rec" onclick={applyIntranet} title="이 에이전트의 인트라넷 보안 베이스라인을 병합">
        인트라넷 추천
      </button>
      <button onclick={validate}>Validate</button>
      {#if agent.format === 'json'}<button onclick={format}>Format</button>{/if}
      <button class="primary" class:dirty onclick={openSaveDialog} disabled={!dirty}>
        {dirty ? '● 저장…' : '저장됨'}
      </button>
    </div>
  </div>

  {#if !agent.exists}
    <p class="new">이 파일은 아직 없습니다. 저장하면 새로 생성됩니다.</p>
  {/if}

  {#if loading}
    <p class="muted">불러오는 중…</p>
  {:else}
    <textarea bind:value={text} spellcheck="false" placeholder={agent.format === 'toml' ? '# TOML' : '{}'}></textarea>
  {/if}

  {#if error}<p class="err" role="alert">오류: {error}</p>{/if}
  {#if status}<p class="ok" role="status">{status}</p>{/if}
</div>

{#if diff}
  <div class="modal-bg" role="presentation" onclick={() => (diff = null)}>
    <div class="modal" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
      <h3>저장 전 변경 확인 — {agent.name}</h3>
      <p class="fp">{diff.path}</p>
      {#if diff.changed}<DiffViewer {diff} />{:else}<p class="nochg">변경 사항이 없습니다.</p>{/if}
      <div class="modal-actions">
        <button onclick={() => (diff = null)}>취소</button>
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
    padding: 0.6rem;
    box-sizing: border-box;
  }
  .bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }
  .fmt {
    font-size: 0.68rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    color: var(--accent-text);
    background: var(--accent-soft);
    border: 1px solid rgba(79, 142, 247, 0.3);
    border-radius: 999px;
    padding: 0.08rem 0.6rem;
  }
  .tools {
    display: flex;
    gap: 0.35rem;
  }
  .tools button {
    padding: 0.3rem 0.7rem;
    background: var(--bg-2);
    border: 1px solid var(--border-strong);
    color: var(--text-1);
    border-radius: var(--r-sm);
    cursor: pointer;
    font-size: 0.78rem;
    transition: background-color var(--t-fast), border-color var(--t-fast);
  }
  .tools button:hover:not(:disabled) {
    background: var(--bg-3);
  }
  .tools .rec {
    background: var(--deny-soft);
    border-color: rgba(248, 113, 113, 0.35);
    color: var(--deny);
    font-weight: 600;
  }
  .tools .rec:hover {
    background: rgba(248, 113, 113, 0.22);
  }
  .tools .primary {
    background: var(--bg-2);
    color: var(--text-3);
    font-weight: 600;
  }
  .tools .primary.dirty {
    background: var(--accent-strong);
    border-color: var(--accent-strong);
    color: white;
    box-shadow: 0 2px 12px rgba(37, 99, 235, 0.3);
  }
  .tools .primary:disabled {
    cursor: default;
  }
  .new {
    color: var(--ask);
    background: var(--ask-soft);
    border-radius: var(--r-sm);
    padding: 0.35rem 0.6rem;
    font-size: 0.78rem;
    margin: 0 0 0.5rem;
  }
  textarea {
    flex: 1;
    width: 100%;
    box-sizing: border-box;
    resize: none;
    background: var(--bg-1);
    color: var(--text-1);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    padding: 0.6rem;
    font-family: var(--font-mono);
    font-size: 0.8rem;
    line-height: 1.5;
    transition: border-color var(--t-fast);
  }
  textarea:focus {
    border-color: var(--accent);
    outline: none;
  }
  .muted {
    color: var(--text-3);
  }
  .err {
    color: var(--deny);
    font-size: 0.8rem;
    margin: 0.4rem 0 0;
  }
  .ok {
    color: var(--allow);
    font-size: 0.8rem;
    word-break: break-all;
    margin: 0.4rem 0 0;
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
