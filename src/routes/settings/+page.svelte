<script lang="ts">
  import { onMount } from 'svelte';
  import { appInfo, diagnosticReport, inTauri, type AppInfo } from '$lib/ipc';
  import { prefs, setSidebarCollapsed, setTheme, type ThemePref } from '$lib/prefs.svelte';
  import { toast } from '$lib/toast.svelte';

  let info = $state<AppInfo | null>(null);
  let copying = $state(false);

  const themes: { value: ThemePref; label: string }[] = [
    { value: 'dark', label: '다크' },
    { value: 'light', label: '라이트' },
    { value: 'system', label: '시스템 설정 따르기' }
  ];

  const shortcuts: { keys: string[]; action: string; where: string }[] = [
    { keys: ['Ctrl', 'B'], action: '사이드바 접기/펼치기', where: '어디서나' },
    { keys: ['Ctrl', ','], action: '설정 열기', where: '어디서나' },
    { keys: ['Ctrl', 'S'], action: '변경 사항 저장 (변경 확인 후)', where: '프로젝트 · 사용자 설정' },
    { keys: ['A'], action: '선택한 경로를 Allow', where: '프로젝트' },
    { keys: ['K'], action: '선택한 경로를 Ask', where: '프로젝트' },
    { keys: ['D'], action: '선택한 경로를 Deny', where: '프로젝트' },
    { keys: ['Esc'], action: '메뉴 · 대화상자 닫기', where: '어디서나' }
  ];

  onMount(async () => {
    try {
      info = await appInfo();
    } catch (e) {
      toast(String(e), 'error');
    }
  });

  async function copyDiagnostics() {
    copying = true;
    try {
      await navigator.clipboard.writeText(await diagnosticReport());
      toast('비밀값을 제외한 진단 정보를 클립보드에 복사했습니다.');
    } catch (e) {
      toast(`진단 정보를 복사하지 못했습니다: ${e}`, 'error');
    } finally {
      copying = false;
    }
  }
</script>

<main>
  <header>
    <h1>설정</h1>
    <p class="muted">앱 화면과 동작에 관한 설정입니다. 변경 즉시 적용되고 이 PC에 저장됩니다.</p>
  </header>

  <section>
    <h2>화면</h2>
    <div class="row">
      <div class="row-text">
        <span id="theme-label">테마</span>
        <span class="muted">시스템 설정을 따르면 Windows 다크/라이트 모드에 맞춰 바뀝니다.</span>
      </div>
      <div class="segmented" role="radiogroup" aria-labelledby="theme-label">
        {#each themes as t (t.value)}
          <button role="radio" aria-checked={prefs.theme === t.value} onclick={() => setTheme(t.value)}>
            {t.label}
          </button>
        {/each}
      </div>
    </div>
    <div class="row">
      <div class="row-text">
        <span id="sidebar-label">사이드바</span>
        <span class="muted">접으면 아이콘만 표시합니다. <kbd>Ctrl</kbd> + <kbd>B</kbd> 로도 전환할 수 있습니다.</span>
      </div>
      <div class="segmented" role="radiogroup" aria-labelledby="sidebar-label">
        <button role="radio" aria-checked={!prefs.sidebarCollapsed} onclick={() => setSidebarCollapsed(false)}>펼침</button>
        <button role="radio" aria-checked={prefs.sidebarCollapsed} onclick={() => setSidebarCollapsed(true)}>접힘</button>
      </div>
    </div>
  </section>

  <section>
    <h2>단축키</h2>
    <table>
      <tbody>
        {#each shortcuts as s (s.action)}
          <tr>
            <td class="keys">
              {#each s.keys as k, i (k)}{#if i > 0}<span class="plus">+</span>{/if}<kbd>{k}</kbd>{/each}
            </td>
            <td>{s.action}</td>
            <td class="muted where">{s.where}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </section>

  <section>
    <h2>문제 해결</h2>
    <div class="row">
      <div class="row-text">
        <span>진단 정보 복사</span>
        <span class="muted">버전, 경로, 설정 파일 상태를 비밀값 없이 복사합니다. 문제 보고 시 붙여 넣어 주세요.</span>
      </div>
      <button class="btn" onclick={copyDiagnostics} disabled={copying || !inTauri()}>
        {copying ? '복사 중…' : '복사'}
      </button>
    </div>
  </section>

  <section>
    <h2>정보</h2>
    {#if info}
      <dl>
        <dt>버전</dt>
        <dd>{info.version}</dd>
        <dt>데이터베이스</dt>
        <dd>스키마 v{info.dbSchemaVersion}</dd>
        <dt>데이터 폴더</dt>
        <dd><code>{info.dataDir}</code></dd>
      </dl>
    {:else}
      <p class="muted">불러오는 중…</p>
    {/if}
  </section>
</main>

<style>
  main {
    max-width: 760px;
    margin: 0 auto;
    padding: 2rem 1.75rem 3rem;
  }
  header {
    margin-bottom: 1.5rem;
  }
  h1 {
    margin: 0 0 0.3rem;
    font-size: 1.4rem;
    letter-spacing: -0.01em;
  }
  h2 {
    margin: 0 0 0.4rem;
    padding-bottom: 0.45rem;
    border-bottom: 1px solid var(--border);
    color: var(--text-2);
    font-size: var(--fs-lg);
  }
  section {
    margin-bottom: 1.75rem;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1.25rem;
    padding: 0.7rem 0;
    border-bottom: 1px solid var(--border);
  }
  .row:last-child {
    border-bottom: 0;
  }
  .row-text {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    min-width: 0;
    font-size: 0.88rem;
  }
  .row-text .muted {
    font-size: var(--fs-sm);
  }
  .row > :last-child {
    flex-shrink: 0;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }
  td {
    padding: 0.5rem 0.4rem;
    border-bottom: 1px solid var(--border);
  }
  tr:last-child td {
    border-bottom: 0;
  }
  .keys {
    width: 8.5rem;
    white-space: nowrap;
  }
  .plus {
    margin: 0 0.25rem;
    color: var(--text-3);
    font-size: var(--fs-sm);
  }
  .where {
    text-align: right;
    font-size: var(--fs-sm);
  }
  dl {
    display: grid;
    grid-template-columns: 8rem 1fr;
    gap: 0.5rem 1rem;
    margin: 0.6rem 0 0;
    font-size: 0.85rem;
  }
  dt {
    color: var(--text-3);
  }
  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
</style>
