<script lang="ts">
  // In-app guide rendered from docs/user-guide.md — a single source of truth that
  // can be reused as-is on GitHub Pages later. The file is inlined at build time.
  import { marked } from 'marked';
  import guideMd from '../../../docs/user-guide.md?raw';

  const html = marked.parse(guideMd, { async: false }) as string;

  function back() {
    history.back();
  }
</script>

<main>
  <div class="bar">
    <button class="back" onclick={back}>← 뒤로</button>
    <code class="src">docs/user-guide.md</code>
  </div>

  <article class="md">
    <!-- eslint-disable-next-line svelte/no-at-html-tags -- our own bundled markdown -->
    {@html html}
  </article>
</main>

<style>
  main {
    max-width: 820px;
    margin: 0 auto;
    padding: 1.4rem 1.5rem 3rem;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 0.6rem;
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
  .src {
    color: var(--text-3);
    font-size: 0.72rem;
  }

  /* Rendered-markdown styling ({@html} output is unscoped -> :global under .md). */
  .md {
    line-height: 1.7;
    font-size: 0.92rem;
  }
  .md :global(h1) {
    font-size: 1.45rem;
    letter-spacing: -0.02em;
    margin: 1.2rem 0 0.8rem;
  }
  .md :global(h2) {
    font-size: 1.02rem;
    letter-spacing: -0.01em;
    color: var(--accent-text);
    margin: 1.8rem 0 0.5rem;
    padding-bottom: 0.3rem;
    border-bottom: 1px solid var(--border);
  }
  .md :global(h3) {
    font-size: 0.9rem;
    color: var(--text-1);
    margin: 1.2rem 0 0.4rem;
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
