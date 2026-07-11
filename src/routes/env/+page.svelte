<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { getEnvStatus, type EnvStatus } from '$lib/ipc';

  let status = $state<EnvStatus | null>(null);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      status = await getEnvStatus();
    } catch (e) {
      error = String(e);
    }
  });
</script>

<main>
  <button class="back" onclick={() => history.back()}>← 뒤로</button>
  <h1>Bedrock / Environment</h1>
  <p class="tagline">읽기 전용 상태입니다. Agent Guard는 Secret 값을 저장하거나 전송하지 않습니다.</p>

  {#if error}<p class="err" role="alert">{error}</p>{/if}

  {#if status}
    {#if status.hasSecretInEnv}
      <div class="warn">
        환경에 AWS Secret 값이 감지되었습니다. 가능하면 <b>AWS_PROFILE</b> 또는 사내 인증 체계를
        사용하고, <code>settings.json</code>에 Secret을 직접 저장하지 마세요.
      </div>
    {/if}
    {#if status.usesProfile}
      <div class="ok">AWS_PROFILE 사용 중 — 권장 구성입니다.</div>
    {/if}

    <div class="table-wrap">
      <table>
        <thead><tr><th>변수</th><th>상태</th><th>값</th></tr></thead>
        <tbody>
          {#each status.vars as v (v.name)}
            <tr class:secret={v.isSecret}>
              <td><code>{v.name}</code></td>
              <td>
                {#if v.present}
                  <span class="pill pill-on"><span class="dot" aria-hidden="true"></span>설정됨</span>
                {:else}
                  <span class="pill pill-off">—</span>
                {/if}
              </td>
              <td class="val">{v.display || ''}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else if !error}
    <p class="muted">불러오는 중…</p>
  {/if}
</main>

<style>
  main {
    max-width: 800px;
    margin: 0 auto;
    padding: 2rem 1.5rem;
  }
  .back {
    background: none;
    border: 1px solid var(--border-strong);
    color: var(--text-2);
    border-radius: var(--r-sm);
    padding: 0.28rem 0.6rem;
    cursor: pointer;
    font-size: 0.8rem;
    transition: color var(--t-fast), background-color var(--t-fast);
  }
  .back:hover {
    color: var(--text-1);
    background: var(--bg-2);
  }
  h1 {
    font-size: 1.45rem;
    letter-spacing: -0.02em;
    margin: 1.2rem 0 0.25rem;
  }
  .tagline {
    color: var(--text-2);
    margin-top: 0;
    font-size: 0.85rem;
  }
  .warn {
    background: var(--ask-soft);
    border: 1px solid rgba(251, 191, 36, 0.3);
    color: #fde68a;
    padding: 0.65rem 0.85rem;
    border-radius: var(--r-sm);
    margin: 1rem 0;
    font-size: 0.85rem;
    line-height: 1.5;
  }
  .warn code {
    background: rgba(0, 0, 0, 0.25);
    padding: 0 0.3rem;
    border-radius: 3px;
  }
  .ok {
    background: var(--allow-soft);
    border: 1px solid rgba(52, 211, 153, 0.3);
    color: var(--allow);
    padding: 0.55rem 0.85rem;
    border-radius: var(--r-sm);
    margin: 0.5rem 0;
    font-size: 0.85rem;
  }
  .err {
    color: var(--deny);
    background: var(--deny-soft);
    border: 1px solid rgba(248, 113, 113, 0.3);
    border-radius: var(--r-sm);
    padding: 0.5rem 0.8rem;
    font-size: 0.85rem;
  }
  .muted {
    color: var(--text-3);
  }
  .table-wrap {
    margin-top: 1.2rem;
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    overflow-x: auto;
    background: var(--bg-1);
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.84rem;
  }
  th,
  td {
    text-align: left;
    padding: 0.5rem 0.8rem;
    border-bottom: 1px solid var(--border);
  }
  tbody tr:last-child td {
    border-bottom: none;
  }
  tbody tr {
    transition: background-color var(--t-fast);
  }
  tbody tr:hover {
    background: var(--bg-2);
  }
  th {
    color: var(--text-3);
    font-weight: 600;
    font-size: 0.7rem;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    background: var(--bg-2);
  }
  .val {
    color: var(--text-2);
    font-family: var(--font-mono);
    font-size: 0.8rem;
    word-break: break-all;
  }
  tr.secret .val {
    color: var(--deny);
  }
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.7rem;
    font-weight: 600;
    padding: 0.1rem 0.55rem;
    border-radius: 999px;
  }
  .pill-on {
    background: var(--allow-soft);
    color: var(--allow);
    border: 1px solid rgba(52, 211, 153, 0.3);
  }
  .pill-on .dot {
    width: 6px;
    height: 6px;
    border-radius: 999px;
    background: var(--allow);
  }
  .pill-off {
    color: var(--text-3);
  }
  code {
    background: var(--bg-0);
    border: 1px solid var(--border);
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
  }
</style>
