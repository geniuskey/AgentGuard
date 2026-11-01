<script lang="ts">
  // Raw JSON editor for one scope (req §8.8). Validate / Format / Save.
  // NOTE: a lightweight textarea editor with live JSON validation. Monaco (the
  // documented choice in docs/tech-stack.md) is deferred as a visual polish item —
  // it cannot be verified in this headless environment and needs CSP worker-src
  // tuning; the functional contract (validate/format/save/diff/restore) is complete.
  import type { ScopeName } from '$lib/ipc';
  import { readRawSettings, saveRawSettings, validateJson } from '$lib/ipc';
  import { app } from '$lib/state.svelte';
  import UnsavedMarker from '$lib/components/UnsavedMarker.svelte';
  import { confirmDiscardChanges } from '$lib/unsaved';
  import { toast } from '$lib/toast.svelte';

  // `lockScope` pins the editor to one scope (used by the project-less User settings
  // page) and hides the scope switcher.
  let { lockScope }: { lockScope?: ScopeName } = $props();

  let picked = $state<ScopeName>('project');
  const scope = $derived(lockScope ?? picked);
  let text = $state('');
  let onDisk = $state('');
  let error = $state<string | null>(null);
  let loading = $state(false);
  let saving = $state(false);
  let loadSequence = 0;

  const dirty = $derived(!loading && text !== onDisk);
  const unsavedId = $derived(`raw-settings-${lockScope ?? 'project'}`);

  const scopes: { id: ScopeName; label: string }[] = [
    { id: 'user', label: '사용자' },
    { id: 'project', label: '프로젝트' },
    { id: 'local', label: '로컬' }
  ];
  const scopeLabel = (s: ScopeName) => scopes.find((x) => x.id === s)?.label ?? s;

  async function load(target: ScopeName) {
    const sequence = ++loadSequence;
    loading = true;
    error = null;
    try {
      const loaded = await readRawSettings(app.projectRoot, target);
      if (sequence !== loadSequence) return;
      text = loaded;
      onDisk = loaded;
    } catch (e) {
      error = String(e);
    } finally {
      if (sequence === loadSequence) loading = false;
    }
  }

  $effect(() => {
    // Reload whenever the scope changes (or on first mount).
    const target = scope;
    load(target);
  });

  function selectScope(next: ScopeName) {
    if (next === scope) return;
    if (!confirmDiscardChanges(dirty, window.confirm.bind(window))) return;
    picked = next;
  }

  async function validate() {
    error = await validateJson(text);
    if (!error) toast('유효한 JSON입니다.');
  }

  function format() {
    try {
      text = JSON.stringify(JSON.parse(text || '{}'), null, 2) + '\n';
      error = null;
      toast('JSON을 정렬했습니다.', 'info');
    } catch (e) {
      error = String(e);
    }
  }

  async function save() {
    const v = await validateJson(text);
    if (v) {
      error = v;
      return;
    }
    saving = true;
    try {
      const res = await saveRawSettings({
        projectRoot: app.projectRoot,
        projectId: app.projectId,
        scope,
        text,
        projectName: app.projectName
      });
      toast(`저장됨 → ${res.written}`);
      error = null;
      onDisk = text;
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<UnsavedMarker id={unsavedId} when={dirty} />

<div class="panel">
  <div class="bar">
    {#if lockScope}
      <span class="chip chip-accent scope-label">{scopeLabel(lockScope)} 범위</span>
    {:else}
      <div class="segmented" role="group" aria-label="설정 범위">
        {#each scopes as s (s.id)}
          <button aria-pressed={scope === s.id} onclick={() => selectScope(s.id)}>{s.label}</button>
        {/each}
      </div>
    {/if}
    <div class="tools">
      <button class="btn btn-sm" onclick={validate} disabled={loading}>검사</button>
      <button class="btn btn-sm" onclick={format} disabled={loading}>정렬</button>
      <button class="btn btn-sm btn-primary" onclick={save} disabled={!dirty || saving}>
        {saving ? '저장 중…' : dirty ? '저장' : '저장됨'}
      </button>
    </div>
  </div>

  {#if loading}
    <p class="muted">불러오는 중…</p>
  {:else}
    <textarea bind:value={text} spellcheck="false" placeholder="{'{}'}" aria-label="{scopeLabel(scope)} 범위 설정 JSON"></textarea>
  {/if}

  {#if error}<p class="callout callout-error err" role="alert">JSON 오류: {error}</p>{/if}
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 0.5rem;
    box-sizing: border-box;
  }
  .bar {
    display: flex;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }
  .scope-label {
    align-self: center;
  }
  .tools {
    display: flex;
    gap: 0.35rem;
    min-width: 0;
    max-width: 100%;
    overflow-x: auto;
  }
  .tools .btn {
    flex-shrink: 0;
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
    font-size: 0.78rem;
    line-height: 1.5;
    transition: border-color var(--t-fast);
  }
  textarea:focus {
    border-color: var(--accent);
    outline: none;
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .err {
    margin-top: 0.4rem;
    font-size: var(--fs-sm);
    overflow-wrap: anywhere;
  }
</style>
