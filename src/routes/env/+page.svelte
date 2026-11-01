<script lang="ts">
  import { onMount } from 'svelte';
  import { getEnvStatus, type EnvStatus } from '$lib/ipc';
  import HelpButton from '$lib/components/HelpButton.svelte';

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
  <div class="head">
    <h1>환경 변수 · Bedrock</h1>
    <HelpButton section="env" compact />
  </div>
  <p class="tagline">읽기 전용 상태입니다. Agent Guard는 Secret 값을 저장하거나 전송하지 않습니다.</p>

  {#if error}<p class="callout callout-error notice" role="alert">{error}</p>{/if}

  {#if status}
    {#if status.hasSecretInEnv}
      <div class="callout callout-warn notice">
        환경에 AWS Secret 값이 감지되었습니다. 가능하면 <b>AWS_PROFILE</b> 또는 사내 인증 체계를
        사용하고, <code>settings.json</code>에 Secret을 직접 저장하지 마세요.
      </div>
    {/if}
    {#if status.usesProfile}
      <div class="callout callout-ok notice">AWS_PROFILE 사용 중 — 권장 구성입니다.</div>
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
                  <span class="pill pill-off"><span aria-hidden="true">—</span><span class="sr-only">설정 안 됨</span></span>
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
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.6rem;
    margin-bottom: 0.25rem;
  }
  h1 {
    font-size: 1.45rem;
    letter-spacing: -0.02em;
    margin: 0;
  }
  .tagline {
    color: var(--text-2);
    margin-top: 0;
    font-size: 0.85rem;
  }
  .notice {
    margin: 0.6rem 0;
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
    border: 1px solid var(--allow-line);
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
