<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import {
    appInfo,
    inTauri,
    listRecentProjects,
    loadSettings,
    openProject,
    pickFolder,
    type AppInfo,
    type ProjectRecord
  } from '$lib/ipc';
  import { setProject, refreshEffective } from '$lib/state.svelte';

  let info = $state<AppInfo | null>(null);
  let recent = $state<ProjectRecord[]>([]);
  let error = $state<string | null>(null);
  let busy = $state(false);

  onMount(async () => {
    try {
      info = await appInfo();
      recent = await listRecentProjects();
    } catch (e) {
      error = String(e);
    }
  });

  async function open(path: string | null) {
    if (!path) return;
    busy = true;
    error = null;
    try {
      const view = await openProject(path);
      const scoped = await loadSettings(view.project.path);
      setProject(view, scoped);
      await refreshEffective();
      await goto('/project');
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function onOpenClick() {
    if (!inTauri()) {
      error = '데스크톱 앱에서만 폴더를 열 수 있습니다 (npm run tauri dev).';
      return;
    }
    await open(await pickFolder());
  }
</script>

<main>
  <header>
    <h1>🛡️ Agent Guard</h1>
    <p class="tagline">
      코딩 에이전트가 프로젝트에서 접근할 수 있는 경계를 눈으로 보고 안전하게 설정하세요.
    </p>
  </header>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <section class="actions">
    <button class="primary" onclick={onOpenClick} disabled={busy}>
      {busy ? '여는 중…' : '📂 프로젝트 열기'}
    </button>
  </section>

  <section>
    <h2>최근 프로젝트</h2>
    {#if recent.length === 0}
      <p class="muted">아직 열어본 프로젝트가 없습니다.</p>
    {:else}
      <ul class="cards">
        {#each recent as p (p.id)}
          <li>
            <button class="card" onclick={() => open(p.path)} disabled={busy}>
              <div class="card-top">
                <span class="name">{p.name}</span>
                {#if p.riskLevel}
                  <span class="risk risk-{p.riskLevel.toLowerCase()}">{p.riskLevel}</span>
                {/if}
              </div>
              <div class="path">{p.path}</div>
              <div class="meta">
                {p.riskProfile ?? 'No profile'} · score {p.riskScore ?? '?'}
              </div>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  {#if info}
    <footer>{info.name} v{info.version} · db v{info.dbSchemaVersion} · {info.dataDir}</footer>
  {/if}
</main>

<style>
  main { max-width: 900px; margin: 0 auto; padding: 2.5rem 1.5rem; }
  h1 { margin: 0 0 0.25rem; font-size: 2rem; }
  .tagline { color: #94a3b8; margin-top: 0; }
  .actions { margin: 1.5rem 0; }
  button.primary {
    padding: 0.7rem 1.2rem; border-radius: 8px; border: 1px solid #2563eb;
    background: #2563eb; color: white; font-size: 1rem; cursor: pointer;
  }
  button.primary:disabled { opacity: 0.6; cursor: default; }
  h2 { font-size: 1.1rem; border-bottom: 1px solid #1e293b; padding-bottom: 0.35rem; }
  .cards { list-style: none; padding: 0; display: grid; gap: 0.6rem; }
  .card {
    width: 100%; text-align: left; padding: 0.8rem 1rem; border-radius: 10px;
    border: 1px solid #334155; background: #1e293b; color: #e2e8f0; cursor: pointer;
  }
  .card:hover { border-color: #475569; }
  .card-top { display: flex; justify-content: space-between; align-items: center; }
  .name { font-weight: 600; }
  .path { color: #64748b; font-size: 0.8rem; margin-top: 0.15rem; }
  .meta { color: #94a3b8; font-size: 0.8rem; margin-top: 0.35rem; }
  .risk { font-size: 0.72rem; padding: 0.1rem 0.5rem; border-radius: 999px; }
  .risk-high { background: #7f1d1d; color: #fecaca; }
  .risk-medium { background: #78350f; color: #fde68a; }
  .risk-low { background: #14532d; color: #bbf7d0; }
  .muted { color: #64748b; }
  .error { color: #f87171; }
  footer { margin-top: 2rem; font-size: 0.8rem; color: #475569; }
</style>
