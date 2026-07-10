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
  <button class="back" onclick={() => history.back()}>← Back</button>
  <h1>🔐 Bedrock / Environment</h1>
  <p class="tagline">읽기 전용 상태입니다. Agent Guard는 Secret 값을 저장하거나 전송하지 않습니다.</p>

  {#if error}<p class="err">{error}</p>{/if}

  {#if status}
    {#if status.hasSecretInEnv}
      <div class="warn">
        ⚠️ 환경에 AWS Secret 값이 감지되었습니다. 가능하면 <b>AWS_PROFILE</b> 또는 사내 인증 체계를
        사용하고, <code>settings.json</code>에 Secret을 직접 저장하지 마세요.
      </div>
    {/if}
    {#if status.usesProfile}
      <div class="ok">✅ AWS_PROFILE 사용 중 — 권장 구성입니다.</div>
    {/if}

    <table>
      <thead><tr><th>변수</th><th>상태</th><th>값</th></tr></thead>
      <tbody>
        {#each status.vars as v (v.name)}
          <tr class:secret={v.isSecret}>
            <td><code>{v.name}</code></td>
            <td>{v.present ? '설정됨' : '—'}</td>
            <td class="val">{v.display || ''}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else if !error}
    <p class="muted">불러오는 중…</p>
  {/if}
</main>

<style>
  main { max-width: 800px; margin: 0 auto; padding: 2rem 1.5rem; }
  .back { background: none; border: 1px solid #334155; color: #94a3b8; border-radius: 6px; padding: 0.3rem 0.6rem; cursor: pointer; }
  h1 { font-size: 1.5rem; margin: 1rem 0 0.25rem; }
  .tagline { color: #94a3b8; margin-top: 0; font-size: 0.85rem; }
  .warn { background: #422006; color: #fde68a; padding: 0.6rem 0.8rem; border-radius: 8px; margin: 1rem 0; font-size: 0.85rem; }
  .ok { background: #14532d; color: #bbf7d0; padding: 0.5rem 0.8rem; border-radius: 8px; margin: 0.5rem 0; font-size: 0.85rem; }
  .err { color: #f87171; }
  .muted { color: #64748b; }
  table { width: 100%; border-collapse: collapse; margin-top: 1rem; font-size: 0.85rem; }
  th, td { text-align: left; padding: 0.4rem 0.6rem; border-bottom: 1px solid #1e293b; }
  th { color: #94a3b8; font-weight: 600; }
  .val { color: #cbd5e1; font-family: ui-monospace, monospace; }
  tr.secret .val { color: #f87171; }
  code { background: #0b1220; padding: 0.1rem 0.35rem; border-radius: 4px; }
</style>
