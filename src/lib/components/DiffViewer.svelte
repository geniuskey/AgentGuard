<script lang="ts">
  import type { DiffView } from '$lib/ipc';

  let { diff }: { diff: DiffView } = $props();

  // Minimal line-level diff: mark lines that differ between before/after.
  const beforeLines = $derived(diff.before ? diff.before.split('\n') : []);
  const afterLines = $derived(diff.after ? diff.after.split('\n') : []);
</script>

<div class="diff">
  <div class="col">
    <div class="h h-before">Before</div>
    <pre>{#each beforeLines as l, i (i)}<span class="ln" class:rm={!afterLines.includes(l) && l.trim()}>{l}</span>{/each}</pre>
  </div>
  <div class="col">
    <div class="h h-after">After</div>
    <pre>{#each afterLines as l, i (i)}<span class="ln" class:add={!beforeLines.includes(l) && l.trim()}>{l}</span>{/each}</pre>
  </div>
</div>

<style>
  .diff {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
  }
  .col {
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    overflow: hidden;
    background: var(--bg-0);
  }
  .h {
    padding: 0.35rem 0.6rem;
    background: var(--bg-1);
    border-bottom: 1px solid var(--border);
    font-size: 0.68rem;
    font-weight: 700;
    letter-spacing: 0.07em;
    text-transform: uppercase;
  }
  .h-before {
    color: var(--deny);
  }
  .h-after {
    color: var(--allow);
  }
  pre {
    margin: 0;
    padding: 0.45rem;
    overflow: auto;
    max-height: 45vh;
    font-size: 0.72rem;
    line-height: 1.4;
  }
  .ln {
    display: block;
    white-space: pre-wrap;
    padding: 0 0.25rem;
    border-radius: 2px;
  }
  .ln.add {
    background: var(--allow-soft);
    box-shadow: inset 2px 0 0 rgba(52, 211, 153, 0.6);
  }
  .ln.rm {
    background: var(--deny-soft);
    box-shadow: inset 2px 0 0 rgba(248, 113, 113, 0.6);
  }
</style>
