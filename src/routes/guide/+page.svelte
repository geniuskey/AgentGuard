<script lang="ts">
  // In-app guide rendered from docs/user-guide.md — a single source of truth that
  // can be reused as-is on GitHub Pages later. The file is inlined at build time.
  //
  // A table of contents (built from the H2 headings) makes every help topic
  // browsable at a glance, and contextual links (?s=<key>) scroll straight to the
  // section relevant to the screen the user came from.
  import { onMount } from 'svelte';
  import { afterNavigate } from '$app/navigation';
  import { marked } from 'marked';
  import { page } from '$app/stores';
  import guideMd from '../../../docs/user-guide.md?raw';

  const html = marked.parse(guideMd, { async: false }) as string;

  // TOC entries from the H2 headings, in document order. The rendered <h2>
  // elements get matching ids (sec-0, sec-1, …) on mount.
  const sections = guideMd
    .split('\n')
    .filter((l) => /^##\s/.test(l))
    .map((l, i) => ({ id: `sec-${i}`, title: cleanTitle(l.replace(/^##\s/, '')), raw: l }));

  // Contextual deep-link keys → a substring of the target heading.
  const KEY_TO_HEADING: Record<string, string> = {
    home: '홈 화면',
    project: '프로젝트 화면',
    simulator: '정책 시뮬레이터',
    mcp: 'MCP 서버',
    tools: '상단 도구',
    save: '저장 · 백업',
    env: 'Bedrock',
    user: '사용자(전역)',
    general: '일반 설정',
    agent: '에이전트 설정',
    gitignore: '.gitignore',
    risk: '리스크 점수',
    shortcuts: '키보드 단축키'
  };

  function cleanTitle(s: string): string {
    return s.replace(/`/g, '').split(' — ')[0].trim();
  }

  let article = $state<HTMLElement | null>(null);
  let activeId = $state('');

  function scrollToId(id: string) {
    document.getElementById(id)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
    activeId = id;
  }

  function targetForKey(key: string | null): string | null {
    if (!key) return null;
    const needle = KEY_TO_HEADING[key];
    if (!needle) return null;
    const idx = sections.findIndex((s) => s.raw.includes(needle));
    return idx >= 0 ? sections[idx].id : null;
  }

  // Assign ids (sec-0, sec-1, …) to the rendered H2s, in document order, so the
  // TOC and deep-links can target them. Idempotent — safe to call more than once.
  function ensureIds(): HTMLElement[] {
    const heads = article ? [...article.querySelectorAll('h2')] : [];
    heads.forEach((el, i) => {
      if (!el.id) el.id = `sec-${i}`;
    });
    return heads;
  }

  onMount(() => {
    const heads = ensureIds();
    if (heads[0]) activeId = heads[0].id;

    const obs = new IntersectionObserver(
      (entries) => {
        for (const e of entries) if (e.isIntersecting) activeId = (e.target as HTMLElement).id;
      },
      { rootMargin: '0px 0px -82% 0px' }
    );
    heads.forEach((h) => obs.observe(h));
    return () => obs.disconnect();
  });

  // Deep-link scroll runs in afterNavigate (not onMount) so it lands *after*
  // SvelteKit's own scroll-to-top on navigation, which would otherwise clobber it.
  afterNavigate(() => {
    ensureIds();
    const id = targetForKey($page.url.searchParams.get('s'));
    if (id) requestAnimationFrame(() => scrollToId(id));
  });

  function back() {
    history.back();
  }
</script>

<div class="page">
  <nav class="toc" aria-label="도움말 목차">
    <button class="back" onclick={back}>← 뒤로</button>
    <div class="toc-title">도움말 목차</div>
    <ul>
      {#each sections as s (s.id)}
        <li>
          <button class="toc-link" class:active={activeId === s.id} onclick={() => scrollToId(s.id)}>
            {s.title}
          </button>
        </li>
      {/each}
    </ul>
  </nav>

  <main>
    <!-- eslint-disable-next-line svelte/no-at-html-tags -- our own bundled markdown -->
    <article class="md" bind:this={article}>{@html html}</article>
    <p class="src">출처: <code>docs/user-guide.md</code> · 로컬에서만 처리됩니다.</p>
  </main>
</div>

<style>
  .page {
    max-width: 1120px;
    margin: 0 auto;
    padding: 1.4rem 1.5rem 3rem;
    display: grid;
    grid-template-columns: 236px 1fr;
    gap: 1.8rem;
    align-items: start;
  }

  /* ── 목차 ── */
  .toc {
    position: sticky;
    top: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    max-height: calc(100vh - 2rem);
    overflow: auto;
  }
  .back {
    align-self: flex-start;
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
  .toc-title {
    font-size: 0.7rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-3);
    margin-top: 0.4rem;
  }
  .toc ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .toc-link {
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    border-left: 2px solid transparent;
    color: var(--text-3);
    padding: 0.32rem 0.6rem;
    cursor: pointer;
    font-size: 0.8rem;
    line-height: 1.35;
    border-radius: 0 var(--r-sm) var(--r-sm) 0;
    transition: color var(--t-fast), background-color var(--t-fast), border-color var(--t-fast);
  }
  .toc-link:hover {
    color: var(--text-1);
    background: var(--bg-2);
  }
  .toc-link.active {
    color: var(--accent-text);
    border-left-color: var(--accent);
    background: var(--accent-soft);
    font-weight: 600;
  }

  .src {
    margin-top: 2rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border);
    color: var(--text-3);
    font-size: 0.74rem;
  }
  .src code {
    background: var(--bg-1);
    border: 1px solid var(--border);
    padding: 0 0.3rem;
    border-radius: 4px;
  }

  @media (max-width: 820px) {
    .page {
      grid-template-columns: 1fr;
      gap: 1rem;
    }
    .toc {
      position: static;
      max-height: none;
    }
    .toc ul {
      flex-direction: row;
      flex-wrap: wrap;
      gap: 0.3rem;
    }
    .toc-link {
      border: 1px solid var(--border);
      border-radius: 999px;
      padding: 0.24rem 0.7rem;
    }
    .toc-link.active {
      border-color: var(--accent);
    }
  }

  /* Rendered-markdown styling ({@html} output is unscoped -> :global under .md). */
  .md {
    line-height: 1.7;
    font-size: 0.92rem;
  }
  .md :global(h1) {
    font-size: 1.45rem;
    letter-spacing: -0.02em;
    margin: 0.2rem 0 0.8rem;
  }
  .md :global(h2) {
    font-size: 1.02rem;
    letter-spacing: -0.01em;
    color: var(--accent-text);
    margin: 1.8rem 0 0.5rem;
    padding-bottom: 0.3rem;
    border-bottom: 1px solid var(--border);
    scroll-margin-top: 1rem;
  }
  .md :global(h3) {
    font-size: 0.9rem;
    color: var(--text-1);
    margin: 1.2rem 0 0.4rem;
    scroll-margin-top: 1rem;
  }
  .md :global(p) {
    margin: 0.5rem 0;
    color: var(--text-1);
  }
  .md :global(blockquote) {
    margin: 0.6rem 0;
    padding: 0.1rem 0.9rem;
    border-left: 3px solid var(--accent);
    background: var(--accent-soft);
    border-radius: 0 var(--r-sm) var(--r-sm) 0;
    color: var(--text-2);
  }
  .md :global(ul),
  .md :global(ol) {
    margin: 0.4rem 0;
    padding-left: 1.3rem;
  }
  .md :global(li) {
    margin: 0.25rem 0;
  }
  .md :global(code) {
    background: var(--bg-1);
    border: 1px solid var(--border);
    padding: 0.08rem 0.35rem;
    border-radius: 4px;
    font-size: 0.84em;
  }
  .md :global(kbd) {
    background: var(--bg-3);
    border: 1px solid var(--border-strong);
    border-bottom-width: 2px;
    border-radius: 5px;
    padding: 0.05rem 0.4rem;
    font-size: 0.78em;
    color: var(--text-2);
  }
  .md :global(table) {
    border-collapse: collapse;
    margin: 0.6rem 0;
    width: 100%;
    font-size: 0.85rem;
  }
  .md :global(th),
  .md :global(td) {
    border: 1px solid var(--border);
    padding: 0.4rem 0.6rem;
    text-align: left;
    vertical-align: top;
  }
  .md :global(th) {
    background: var(--bg-2);
    color: var(--text-2);
    font-size: 0.76rem;
    letter-spacing: 0.03em;
  }
  .md :global(tr:nth-child(even) td) {
    background: var(--bg-1);
  }
  .md :global(a) {
    color: var(--accent-text);
  }
  .md :global(hr) {
    border: none;
    border-top: 1px solid var(--border);
    margin: 1.4rem 0;
  }
</style>
