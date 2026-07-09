<script lang="ts">
  import { onMount } from 'svelte';
  import { appInfo, listRecentProjects, type AppInfo, type RecentProject } from '$lib/ipc';

  let info = $state<AppInfo | null>(null);
  let recent = $state<RecentProject[]>([]);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      info = await appInfo();
      recent = await listRecentProjects();
    } catch (e) {
      error = String(e);
    }
  });
</script>

<main>
  <header>
    <h1>🛡️ Agent Guard</h1>
    <p class="tagline">
      코딩 에이전트가 프로젝트에서 접근할 수 있는 경계를 눈으로 보고 안전하게 설정하세요.
    </p>
  </header>

  {#if error}
    <p class="error">초기화 오류: {error}</p>
  {/if}

  <section class="actions">
    <button disabled>프로젝트 열기 (다음 반복)</button>
    <button disabled>User settings 열기 (다음 반복)</button>
  </section>

  <section>
    <h2>최근 프로젝트</h2>
    {#if recent.length === 0}
      <p class="muted">아직 열어본 프로젝트가 없습니다.</p>
    {:else}
      <ul>
        {#each recent as p (p.projectPath)}
          <li>{p.projectName} — {p.riskLevel ?? '?'}</li>
        {/each}
      </ul>
    {/if}
  </section>

  {#if info}
    <footer>
      {info.name} v{info.version} · data: {info.dataDir} · db schema v{info.dbSchemaVersion}
    </footer>
  {/if}
</main>

<style>
  main {
    max-width: 900px;
    margin: 0 auto;
    padding: 2.5rem 1.5rem;
  }
  h1 {
    margin: 0 0 0.25rem;
    font-size: 2rem;
  }
  .tagline {
    color: #94a3b8;
    margin-top: 0;
  }
  .actions {
    display: flex;
    gap: 0.75rem;
    margin: 1.5rem 0;
  }
  button {
    padding: 0.6rem 1rem;
    border-radius: 8px;
    border: 1px solid #334155;
    background: #1e293b;
    color: #e2e8f0;
    cursor: not-allowed;
  }
  h2 {
    font-size: 1.1rem;
    border-bottom: 1px solid #1e293b;
    padding-bottom: 0.35rem;
  }
  .muted {
    color: #64748b;
  }
  .error {
    color: #f87171;
  }
  footer {
    margin-top: 2rem;
    font-size: 0.8rem;
    color: #475569;
  }
</style>
