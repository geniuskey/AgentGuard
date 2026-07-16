<script lang="ts">
  import type { DiffView } from '$lib/ipc';

  let { diff }: { diff: DiffView } = $props();

  let showLineNumbers = $state(true);
  let diffOnly = $state(false);

  type Row = {
    type: 'equal' | 'add' | 'del' | 'mod';
    left: string | null;
    right: string | null;
    bnum: number | null;
    anum: number | null;
  };

  // LCS-based line diff → rows aligned side-by-side (Beyond Compare style),
  // pairing adjacent deletions/insertions into a single "modified" row. Both
  // sides of a row live in the same grid row, so they stay vertically aligned
  // even when a long line wraps.
  const rows = $derived.by<Row[]>(() => {
    const a = diff.before ? diff.before.split('\n') : [];
    const b = diff.after ? diff.after.split('\n') : [];
    const n = a.length;
    const m = b.length;

    // dp[i][j] = LCS length of a[i..] and b[j..].
    const dp: number[][] = Array.from({ length: n + 1 }, () => new Array(m + 1).fill(0));
    for (let i = n - 1; i >= 0; i--)
      for (let j = m - 1; j >= 0; j--)
        dp[i][j] = a[i] === b[j] ? dp[i + 1][j + 1] + 1 : Math.max(dp[i + 1][j], dp[i][j + 1]);

    const ops: { t: 'eq' | 'del' | 'add'; text: string }[] = [];
    let i = 0;
    let j = 0;
    while (i < n && j < m) {
      if (a[i] === b[j]) {
        ops.push({ t: 'eq', text: a[i] });
        i++;
        j++;
      } else if (dp[i + 1][j] >= dp[i][j + 1]) {
        ops.push({ t: 'del', text: a[i] });
        i++;
      } else {
        ops.push({ t: 'add', text: b[j] });
        j++;
      }
    }
    while (i < n) ops.push({ t: 'del', text: a[i++] });
    while (j < m) ops.push({ t: 'add', text: b[j++] });

    const out: Row[] = [];
    let bnum = 0;
    let anum = 0;
    let k = 0;
    while (k < ops.length) {
      if (ops[k].t === 'eq') {
        bnum++;
        anum++;
        out.push({ type: 'equal', left: ops[k].text, right: ops[k].text, bnum, anum });
        k++;
        continue;
      }
      // Gather a run of consecutive del/add and zip them into aligned rows.
      const dels: string[] = [];
      const adds: string[] = [];
      while (k < ops.length && ops[k].t !== 'eq') {
        if (ops[k].t === 'del') dels.push(ops[k].text);
        else adds.push(ops[k].text);
        k++;
      }
      const len = Math.max(dels.length, adds.length);
      for (let x = 0; x < len; x++) {
        const l = x < dels.length ? dels[x] : null;
        const r = x < adds.length ? adds[x] : null;
        if (l !== null) bnum++;
        if (r !== null) anum++;
        out.push({
          type: l !== null && r !== null ? 'mod' : l !== null ? 'del' : 'add',
          left: l,
          right: r,
          bnum: l !== null ? bnum : null,
          anum: r !== null ? anum : null
        });
      }
    }
    return out;
  });

  type DisplayItem = { kind: 'row'; row: Row } | { kind: 'gap'; count: number };

  // "Diff only": collapse runs of unchanged lines into a single gap marker.
  const display = $derived.by<DisplayItem[]>(() => {
    if (!diffOnly) return rows.map((row) => ({ kind: 'row', row }) as DisplayItem);
    const out: DisplayItem[] = [];
    let run = 0;
    for (const row of rows) {
      if (row.type === 'equal') {
        run++;
      } else {
        if (run > 0) {
          out.push({ kind: 'gap', count: run });
          run = 0;
        }
        out.push({ kind: 'row', row });
      }
    }
    if (run > 0) out.push({ kind: 'gap', count: run });
    return out;
  });

  const cols = $derived(showLineNumbers ? 'auto minmax(0, 1fr) auto minmax(0, 1fr)' : 'minmax(0, 1fr) minmax(0, 1fr)');
  const afterStart = $derived(showLineNumbers ? 3 : 2);
</script>

<div class="diff">
  <div class="toolbar">
    <label class="tg"><input type="checkbox" bind:checked={showLineNumbers} /> 줄 번호</label>
    <label class="tg"><input type="checkbox" bind:checked={diffOnly} /> 변경된 줄만</label>
  </div>

  <!-- Single scroll container → both sides scroll together (Beyond Compare). -->
  <div class="scroll">
    <div class="grid" style="grid-template-columns: {cols}">
      <div class="hcell h-before" style="grid-column: 1 / {afterStart}">Before</div>
      <div class="hcell h-after sep" style="grid-column: {afterStart} / -1">After</div>

      {#each display as d, i (i)}
        {#if d.kind === 'gap'}
          <div class="gap" style="grid-column: 1 / -1">⋯ 변경 없는 {d.count}줄</div>
        {:else}
          {@const r = d.row}
          {#if showLineNumbers}
            <div class="ln" class:hl-del={r.type === 'del' || r.type === 'mod'} class:empty={r.left === null}>{r.bnum ?? ''}</div>
          {/if}
          <div class="tx" class:hl-del={r.type === 'del' || r.type === 'mod'} class:empty={r.left === null}>{r.left ?? ''}</div>
          {#if showLineNumbers}
            <div class="ln rt sep" class:hl-add={r.type === 'add' || r.type === 'mod'} class:empty={r.right === null}>{r.anum ?? ''}</div>
          {/if}
          <div class="tx rt" class:sep={!showLineNumbers} class:hl-add={r.type === 'add' || r.type === 'mod'} class:empty={r.right === null}>{r.right ?? ''}</div>
        {/if}
      {/each}
    </div>
  </div>
</div>

<style>
  .diff {
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    overflow: hidden;
    background: var(--bg-0);
  }
  .toolbar {
    display: flex;
    gap: 0.9rem;
    padding: 0.35rem 0.6rem;
    background: var(--bg-1);
    border-bottom: 1px solid var(--border);
  }
  .tg {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.72rem;
    color: var(--text-2);
    cursor: pointer;
    user-select: none;
  }
  .tg input {
    accent-color: var(--accent);
  }
  .scroll {
    overflow: auto;
    max-height: 45vh;
  }
  .grid {
    display: grid;
    align-content: start;
    font-family: var(--font-mono);
    font-size: 0.72rem;
    line-height: 1.5;
    min-width: fit-content;
  }
  .hcell {
    position: sticky;
    top: 0;
    z-index: 1;
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
  .sep {
    border-left: 1px solid var(--border-strong);
  }
  .ln {
    padding: 0 0.5rem;
    text-align: right;
    color: var(--text-3);
    user-select: none;
    white-space: nowrap;
    -webkit-user-select: none;
  }
  .tx {
    padding: 0 0.45rem;
    white-space: pre-wrap;
    word-break: break-word;
    color: var(--text-1);
  }
  .hl-del {
    background: var(--deny-soft);
  }
  .hl-add {
    background: var(--allow-soft);
  }
  .tx.hl-del {
    box-shadow: inset 2px 0 0 rgba(248, 113, 113, 0.6);
  }
  .tx.hl-add {
    box-shadow: inset 2px 0 0 rgba(52, 211, 153, 0.6);
  }
  /* A side with no corresponding line — render a faint placeholder. */
  .empty {
    background: repeating-linear-gradient(
      45deg,
      var(--bg-1),
      var(--bg-1) 4px,
      transparent 4px,
      transparent 8px
    );
    opacity: 0.5;
  }
  .gap {
    padding: 0.2rem 0.6rem;
    background: var(--bg-1);
    border-top: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
    color: var(--text-3);
    font-size: 0.66rem;
    text-align: center;
    letter-spacing: 0.04em;
  }
</style>
