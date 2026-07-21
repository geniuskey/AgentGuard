<script lang="ts">
  // Config editor for a single agent global file (JSON or TOML): a structured
  // settings form (AgentSettingsForm) plus a raw text view over the same text state.
  // Validates by format before saving, shows a pre-save diff, and always backs up
  // first. Used for Codex/OpenCode; Claude Code's permission editor lives at /user.
  import type { AgentGlobal, AgentSecItem, DiffView } from '$lib/ipc';
  import {
    agentSecurityStatus,
    inTauri,
    readAgentConfig,
    saveAgentConfig,
    securityBaseline,
    validateConfig
  } from '$lib/ipc';
  import DiffViewer from '$lib/components/DiffViewer.svelte';
  import AgentSettingsForm from '$lib/components/AgentSettingsForm.svelte';
  import UnsavedMarker from '$lib/components/UnsavedMarker.svelte';
  import { modalFocus } from '$lib/modal';

  let { agent }: { agent: AgentGlobal } = $props();

  let text = $state('');
  let onDisk = $state('');
  let loading = $state(false);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let status = $state<string | null>(null);
  let diff = $state<DiffView | null>(null);
  let secItems = $state<AgentSecItem[] | null>(null);
  let view = $state<'form' | 'raw'>('form');

  // Security summary of the editor text (debounced; hidden while unparseable).
  $effect(() => {
    const current = text;
    const id = agent.id;
    if (!inTauri()) return;
    const t = setTimeout(async () => {
      try {
        secItems = await agentSecurityStatus(id, current);
      } catch {
        secItems = null;
      }
    }, 350);
    return () => clearTimeout(t);
  });

  async function load() {
    if (!inTauri()) return;
    loading = true;
    error = null;
    status = null;
    try {
      onDisk = await readAgentConfig(agent.id);
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

  // Merge this agent's security baseline into the editor for review.
  async function applyBaseline() {
    error = null;
    try {
      text = await securityBaseline(agent.id, text);
      status = '보안 베이스라인을 추가했습니다. 검토 후 저장하세요.';
    } catch (e) {
      error = String(e);
    }
  }

  async function confirmSave() {
    if (!diff) return;
    saving = true;
    try {
      const res = await saveAgentConfig({
        text,
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

<UnsavedMarker id={`agent-config-${agent.id}`} when={dirty} />

<div class="panel">
  <div class="bar">
    <span class="views">
      <button class:active={view === 'form'} onclick={() => (view = 'form')}>설정</button>
      <button class:active={view === 'raw'} onclick={() => (view = 'raw')}>
        Raw {agent.format.toUpperCase()}
      </button>
    </span>
    <div class="tools">
      <button class="rec" onclick={applyBaseline} title="권장 보안 설정(공유·웹·자격증명 차단)을 현재 설정에 병합">
        보안 베이스라인
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

  {#if secItems}
    <div class="sec" aria-label="보안 설정 요약">
      {#each secItems as it (it.label)}
        <span
          class="sec-item"
          class:safe={it.ok === true}
          class:warn={it.ok === false}
          title={it.ok === false ? '주의가 필요한 설정입니다' : undefined}
        >
          {it.label} <b>{it.value}</b>
        </span>
      {/each}
    </div>
  {/if}

  {#if loading}
    <p class="muted">불러오는 중…</p>
  {:else if view === 'form'}
    <AgentSettingsForm agentId={agent.id} format={agent.format} {text} onchange={(t) => (text = t)} />
  {:else}
    <textarea bind:value={text} spellcheck="false" placeholder={agent.format === 'toml' ? '# TOML' : '{}'}></textarea>
  {/if}

  {#if error}<p class="err" role="alert">오류: {error}</p>{/if}
  {#if status}<p class="ok" role="status">{status}</p>{/if}
</div>

{#if diff}
  <div class="modal-bg" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) diff = null; }}>
    <div
      class="modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="agent-save-title"
      tabindex="-1"
      use:modalFocus={() => (diff = null)}
    >
      <h3 id="agent-save-title">저장 전 변경 확인 — {agent.name}</h3>
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
    padding: 0.6rem;
    box-sizing: border-box;
  }
  .bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }
  .views {
    display: flex;
    border: 1px solid var(--border-strong);
    border-radius: var(--r-sm);
    overflow: hidden;
  }
  .views button {
    background: transparent;
    border: none;
    color: var(--text-2);
    padding: 0.3rem 0.7rem;
    cursor: pointer;
    font-size: 0.76rem;
    white-space: nowrap;
    transition: background-color var(--t-fast), color var(--t-fast);
  }
  .views button.active {
    background: var(--accent-soft);
    color: var(--accent-text);
  }
  .tools {
    display: flex;
    gap: 0.35rem;
    min-width: 0;
    max-width: 100%;
    overflow-x: auto;
  }
  .tools button {
    flex-shrink: 0;
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
    background: var(--allow-soft);
    border-color: rgba(52, 211, 153, 0.35);
    color: var(--allow);
    font-weight: 600;
  }
  .tools .rec:hover {
    background: rgba(52, 211, 153, 0.22);
  }
  .sec {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
    margin-bottom: 0.5rem;
  }
  .sec-item {
    font-size: 0.7rem;
    color: var(--text-3);
    background: var(--bg-1);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0.12rem 0.55rem;
    white-space: nowrap;
  }
  .sec-item b {
    color: var(--text-2);
    font-weight: 600;
  }
  .sec-item.safe {
    color: var(--allow);
    background: var(--allow-soft);
    border-color: rgba(52, 211, 153, 0.3);
  }
  .sec-item.safe b {
    color: inherit;
  }
  .sec-item.warn {
    color: var(--ask);
    background: var(--ask-soft);
    border-color: rgba(251, 191, 36, 0.35);
  }
  .sec-item.warn b {
    color: inherit;
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
