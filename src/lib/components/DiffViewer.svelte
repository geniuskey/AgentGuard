<script lang="ts">
  import type { DiffView } from '$lib/ipc';

  let { diff }: { diff: DiffView } = $props();

  // Minimal line-level diff: mark lines that differ between before/after.
  const beforeLines = $derived(diff.before ? diff.before.split('\n') : []);
  const afterLines = $derived(diff.after ? diff.after.split('\n') : []);
</script>

<div class="diff">
  <div class="col">
    <div class="h">Before</div>
    <pre>{#each beforeLines as l, i (i)}<span class="ln" class:rm={!afterLines.includes(l) && l.trim()}>{l}</span>{/each}</pre>
  </div>
  <div class="col">
    <div class="h">After</div>
    <pre>{#each afterLines as l, i (i)}<span class="ln" class:add={!beforeLines.includes(l) && l.trim()}>{l}</span>{/each}</pre>
  </div>
</div>

<style>
  .diff { display: grid; grid-template-columns: 1fr 1fr; gap: 0.5rem; }
  .col { border: 1px solid #1e293b; border-radius: 6px; overflow: hidden; }
  .h { padding: 0.3rem 0.5rem; background: #0b1220; font-size: 0.75rem; color: #94a3b8; }
  pre { margin: 0; padding: 0.4rem; overflow: auto; max-height: 45vh; font-size: 0.72rem; line-height: 1.35; }
  .ln { display: block; white-space: pre-wrap; }
  .ln.add { background: #14532d55; }
  .ln.rm { background: #7f1d1d55; }
</style>
