<script lang="ts">
  import type { DiffView } from '$lib/ipc';

  let { diff }: { diff: DiffView } = $props();
  const uid = $props.id();
  const scrollId = `diff-scroll-${uid}`;

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

  // --- Minimap: whole-file overview bar (Beyond Compare style) -------------
  // A compressed vertical strip on the left showing every changed row as a
  // red (before/deleted) / green (after/added) mark, click-or-drag to jump.
  let scrollEl: HTMLDivElement | undefined = $state();
  let minimapEl: HTMLDivElement | undefined = $state();
  let canvasEl: HTMLCanvasElement | undefined = $state();
  let viewTop = $state(0);
  let viewSize = $state(1);

  const DEL_COLOR = 'rgba(248, 113, 113, 0.85)';
  const ADD_COLOR = 'rgba(52, 211, 153, 0.85)';

  function drawMinimap() {
    const wrap = minimapEl;
    const canvas = canvasEl;
    if (!wrap || !canvas) return;
    const w = wrap.clientWidth;
    const h = wrap.clientHeight;
    if (!w || !h) return;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.round(w * dpr);
    canvas.height = Math.round(h * dpr);
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);

    const items = display;
    const total = items.length || 1;
    const unit = h / total;
    const barH = Math.max(unit, 1.5);
    const half = w / 2;

    items.forEach((it, idx) => {
      if (it.kind === 'gap') return;
      const r = it.row;
      const y = idx * unit;
      if (r.type === 'del' || r.type === 'mod') {
        ctx.fillStyle = DEL_COLOR;
        ctx.fillRect(0, y, r.type === 'mod' ? half : w, barH);
      }
      if (r.type === 'add' || r.type === 'mod') {
        ctx.fillStyle = ADD_COLOR;
        ctx.fillRect(r.type === 'mod' ? half : 0, y, r.type === 'mod' ? half : w, barH);
      }
    });
  }

  function updateViewport() {
    const el = scrollEl;
    if (!el) return;
    const sh = el.scrollHeight || 1;
    viewTop = el.scrollTop / sh;
    viewSize = Math.min(1, el.clientHeight / sh);
  }

  // Center the clicked/dragged position in the scroll viewport.
  function jumpTo(clientY: number) {
    const wrap = minimapEl;
    const el = scrollEl;
    if (!wrap || !el) return;
    const rect = wrap.getBoundingClientRect();
    const fraction = Math.min(1, Math.max(0, (clientY - rect.top) / rect.height));
    const max = el.scrollHeight - el.clientHeight;
    el.scrollTop = Math.max(0, Math.min(max, fraction * el.scrollHeight - el.clientHeight / 2));
  }

  function onMinimapDown(e: MouseEvent) {
    jumpTo(e.clientY);
    const onMove = (ev: MouseEvent) => jumpTo(ev.clientY);
    const onUp = () => {
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
    };
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  }

  function onMinimapKey(e: KeyboardEvent) {
    const el = scrollEl;
    if (!el) return;
    if (e.key === 'ArrowDown' || e.key === 'PageDown') {
      el.scrollTop += e.key === 'PageDown' ? el.clientHeight : 40;
      e.preventDefault();
    } else if (e.key === 'ArrowUp' || e.key === 'PageUp') {
      el.scrollTop -= e.key === 'PageUp' ? el.clientHeight : 40;
      e.preventDefault();
    } else if (e.key === 'Home') {
      el.scrollTop = 0;
      e.preventDefault();
    } else if (e.key === 'End') {
      el.scrollTop = el.scrollHeight;
      e.preventDefault();
    }
  }

  $effect(() => {
    display;
    drawMinimap();
  });

  $effect(() => {
    const el = scrollEl;
    const wrap = minimapEl;
    if (!el) return;
    const onScroll = () => updateViewport();
    el.addEventListener('scroll', onScroll, { passive: true });
    updateViewport();
    const ro = new ResizeObserver(() => {
      updateViewport();
      drawMinimap();
    });
    ro.observe(el);
    if (wrap) ro.observe(wrap);
    return () => {
      el.removeEventListener('scroll', onScroll);
      ro.disconnect();
    };
  });
</script>

<div class="diff">
  <div class="toolbar">
    <label class="tg"><input type="checkbox" bind:checked={showLineNumbers} /> 줄 번호</label>
    <label class="tg"><input type="checkbox" bind:checked={diffOnly} /> 변경된 줄만</label>
    <span class="legend"><i class="sw del"></i>삭제<i class="sw add"></i>추가</span>
  </div>

  <div class="body">
    <!-- Whole-file overview bar: red = removed(before), green = added(after). -->
    <div
      class="minimap"
      bind:this={minimapEl}
      onmousedown={onMinimapDown}
      onkeydown={onMinimapKey}
      role="scrollbar"
      aria-orientation="vertical"
      aria-controls={scrollId}
      aria-valuenow={Math.round(viewTop * 100)}
      aria-valuemin={0}
      aria-valuemax={100}
      tabindex="0"
      title="변경 위치 미리보기 — 클릭/드래그로 이동"
    >
      <canvas bind:this={canvasEl}></canvas>
      <div class="mm-view" style="top: {viewTop * 100}%; height: {Math.max(viewSize * 100, 3)}%"></div>
    </div>

    <!-- Single scroll container → both sides scroll together (Beyond Compare). -->
    <div class="scroll" id={scrollId} bind:this={scrollEl}>
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
  .legend {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    margin-left: auto;
    font-size: 0.68rem;
    color: var(--text-3);
  }
  .sw {
    display: inline-block;
    width: 0.6rem;
    height: 0.6rem;
    border-radius: 2px;
    margin-right: 0.15rem;
  }
  .sw:not(:first-child) {
    margin-left: 0.5rem;
  }
  .sw.del {
    background: rgba(248, 113, 113, 0.85);
  }
  .sw.add {
    background: rgba(52, 211, 153, 0.85);
  }
  .body {
    display: flex;
    align-items: stretch;
  }
  .minimap {
    position: relative;
    flex: 0 0 16px;
    width: 16px;
    background: var(--bg-1);
    border-right: 1px solid var(--border);
    cursor: pointer;
  }
  .minimap:focus-visible {
    outline: 2px solid var(--accent-strong);
    outline-offset: -2px;
  }
  .minimap canvas {
    display: block;
    width: 100%;
    height: 100%;
  }
  .mm-view {
    position: absolute;
    left: 0;
    right: 0;
    pointer-events: none;
    background: color-mix(in srgb, var(--text-1) 12%, transparent);
    border-top: 1px solid var(--border-strong);
    border-bottom: 1px solid var(--border-strong);
  }
  .scroll {
    flex: 1;
    min-width: 0;
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
