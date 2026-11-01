<script lang="ts">
  // One row of a structured settings form: question-style label on the left, the
  // control that fits the value on the right (switch / segmented / select / preset
  // chips / number with unit / text). `null` from onchange removes the key.
  import type { Snippet } from 'svelte';
  import { sameValue, toggleState, type Choice, type FieldSpec } from '$lib/setting-field';
  import { tooltip } from '$lib/tooltip';

  let {
    field: f,
    value,
    onchange,
    keyLabel,
    extra
  }: {
    field: FieldSpec;
    value: unknown;
    onchange: (v: unknown) => void;
    /** Raw key shown on hover (defaults to field.key). */
    keyLabel?: string;
    /** Extra content under the control (e.g. "시스템 값" hint, remove button). */
    extra?: Snippet;
  } = $props();

  const id = $derived(`f-${f.key.replace(/[^A-Za-z0-9_-]/g, '_')}`);
  const unset = $derived(value === undefined || value === null);
  const defaultChip = $derived(f.defaultText ? `기본값 · ${f.defaultText}` : '기본값');

  const choices = $derived(f.choices ?? []);
  const matched = $derived(choices.find((c) => sameValue(c.v, value)));
  const segmented = $derived(
    f.kind === 'choice' && choices.length <= 4 && choices.every((c) => c.label.length <= 12)
  );
  const hint = $derived.by(() => {
    if (f.kind === 'toggle') return undefined;
    if (unset) return undefined;
    return matched?.hint;
  });
  const risky = $derived(matched?.risky === true);

  function pick(c: Choice | null) {
    onchange(c ? c.v : null);
  }

  function onSelect(e: Event) {
    const i = (e.currentTarget as HTMLSelectElement).selectedIndex;
    if (i === 0) pick(null);
    else if (i <= choices.length) pick(choices[i - 1]);
  }

  function onText(e: Event) {
    const v = (e.currentTarget as HTMLInputElement).value.trim();
    if (v === (typeof value === 'string' ? value : value == null ? '' : String(value))) return;
    onchange(v === '' ? null : v);
  }

  const scale = $derived(f.scale ?? 1);
  const shownNumber = $derived.by(() => {
    if (unset) return '';
    const n = Number(value);
    return Number.isFinite(n) ? String(n / scale) : String(value);
  });

  function onNumber(e: Event) {
    const raw = (e.currentTarget as HTMLInputElement).value.trim();
    if (raw === '') return onchange(null);
    const n = Number(raw);
    if (!Number.isFinite(n) || (f.min !== undefined && n < f.min)) return;
    const stored = Math.round(n * scale);
    onchange(f.asString ? String(stored) : stored);
  }

  const tstate = $derived(f.kind === 'toggle' ? toggleState(f, value) : 'default');
  const tOn = $derived(f.on ?? true);
  const tOff = $derived(f.off === undefined ? false : f.off);
</script>

<div class="row">
  <div class="lbl-col">
    <label class="lbl" id="{id}-lbl" for={id} use:tooltip={keyLabel ?? f.key}>{f.label}</label>
    {#if f.help}<p class="help">{f.help}</p>{/if}
  </div>

  <div class="ctl-col">
    {#if f.kind === 'toggle' && tOff === null}
      <button
        {id}
        class="switch"
        role="switch"
        aria-checked={tstate === 'on'}
        onclick={() => onchange(tstate === 'on' ? null : tOn)}
      >
        <span class="knob" aria-hidden="true"></span>
        <span class="switch-text">{tstate === 'on' ? '켬' : '끔'}</span>
      </button>
    {:else if f.kind === 'toggle'}
      <div class="segmented" role="radiogroup" aria-labelledby="{id}-lbl">
        <button {id} role="radio" aria-checked={tstate === 'default'} onclick={() => onchange(null)}>
          {defaultChip}
        </button>
        <button role="radio" aria-checked={tstate === 'on'} onclick={() => onchange(tOn)}>켬</button>
        <button role="radio" aria-checked={tstate === 'off'} onclick={() => onchange(tOff)}>끔</button>
      </div>
    {:else if f.kind === 'choice' && segmented}
      <div class="segmented" role="radiogroup" aria-labelledby="{id}-lbl">
        <button {id} role="radio" aria-checked={unset} onclick={() => pick(null)}>{defaultChip}</button>
        {#each choices as c (String(c.v))}
          <button
            role="radio"
            class:risky={c.risky}
            aria-checked={!unset && sameValue(c.v, value)}
            onclick={() => pick(c)}>{c.label}</button
          >
        {/each}
      </div>
    {:else if f.kind === 'choice'}
      <select class="input" {id} class:risky onchange={onSelect}>
        <option selected={unset}>{defaultChip}</option>
        {#each choices as c (String(c.v))}
          <option selected={!unset && sameValue(c.v, value)}>{c.label}</option>
        {/each}
        {#if !unset && !matched}
          <option selected disabled>현재 값: {String(value)}</option>
        {/if}
      </select>
    {:else if f.kind === 'combo'}
      <div class="combo">
        <div class="chips" role="radiogroup" aria-labelledby="{id}-lbl">
          <button class="pchip" role="radio" aria-checked={unset} onclick={() => pick(null)}>
            {defaultChip}
          </button>
          {#each choices as c (String(c.v))}
            <button
              class="pchip"
              role="radio"
              aria-checked={!unset && sameValue(c.v, value)}
              onclick={() => pick(c)}>{c.label}</button
            >
          {/each}
        </div>
        <input
          class="input mono"
          {id}
          type="text"
          value={!unset && !matched ? String(value) : ''}
          placeholder={f.placeholder ?? '직접 입력'}
          spellcheck="false"
          onchange={onText}
        />
      </div>
    {:else if f.kind === 'number'}
      <div class="num">
        <input
          class="input mono"
          {id}
          type="number"
          min={f.min}
          value={shownNumber}
          placeholder={f.defaultText ? `기본 ${f.defaultText}` : f.placeholder}
          onchange={onNumber}
        />
        {#if f.unit}<span class="unit">{f.unit}</span>{/if}
      </div>
    {:else}
      <input
        class="input mono"
        {id}
        type="text"
        value={unset ? '' : String(value)}
        placeholder={f.placeholder ?? (f.defaultText ? `기본: ${f.defaultText}` : '')}
        spellcheck="false"
        onchange={onText}
      />
    {/if}

    {#if hint}<p class="hint" class:risky>{hint}</p>{/if}
    {#if f.caution && f.caution.when(value)}
      <p class="callout callout-warn caution" role="note">{f.caution.text}</p>
    {/if}
    {#if extra}{@render extra()}{/if}
  </div>
</div>

<style>
  .row {
    display: grid;
    grid-template-columns: minmax(10rem, 15rem) minmax(0, 1fr);
    align-items: start;
    gap: 0.35rem 1rem;
    padding: 0.55rem 0;
    border-bottom: 1px solid var(--border);
  }
  .row:last-child {
    border-bottom: none;
  }
  .lbl-col {
    min-width: 0;
    padding-top: 0.3rem;
  }
  .lbl {
    font-size: var(--fs-md);
    font-weight: 500;
    color: var(--text-1);
    cursor: default;
  }
  .help {
    color: var(--text-3);
    font-size: var(--fs-xs);
    line-height: 1.45;
    margin: 0.15rem 0 0;
  }
  .ctl-col {
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.3rem;
  }
  .segmented {
    flex-wrap: wrap;
  }
  .segmented > button {
    font-size: var(--fs-sm);
    padding: 0.28rem 0.65rem;
  }
  .segmented > button.risky[aria-checked='true'],
  select.risky {
    color: var(--ask);
  }
  .input {
    font-size: var(--fs-sm);
    padding: 0.32rem 0.5rem;
    max-width: 26rem;
  }
  .mono {
    font-family: var(--font-mono);
  }
  .mono::placeholder {
    font-family: var(--font-sans);
    color: var(--text-3);
  }
  .switch {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    background: none;
    border: none;
    padding: 0.2rem 0;
    cursor: pointer;
    color: var(--text-2);
    font-size: var(--fs-sm);
  }
  .knob {
    position: relative;
    width: 2.1rem;
    height: 1.2rem;
    border-radius: 999px;
    background: var(--bg-3);
    border: 1px solid var(--border-strong);
    transition: background-color var(--t-fast);
  }
  .knob::after {
    content: '';
    position: absolute;
    top: 2px;
    left: 2px;
    width: calc(1.2rem - 6px);
    height: calc(1.2rem - 6px);
    border-radius: 999px;
    background: var(--text-2);
    transition: transform var(--t-fast), background-color var(--t-fast);
  }
  .switch[aria-checked='true'] .knob {
    background: var(--accent-strong);
    border-color: var(--accent-strong);
  }
  .switch[aria-checked='true'] .knob::after {
    transform: translateX(0.9rem);
    background: var(--on-accent);
  }
  .switch[aria-checked='true'] .switch-text {
    color: var(--text-1);
  }
  .switch:focus-visible .knob {
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .combo {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    width: 100%;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
  }
  .pchip {
    border: 1px solid var(--border-strong);
    background: var(--bg-1);
    color: var(--text-2);
    border-radius: 999px;
    padding: 0.18rem 0.65rem;
    font-size: var(--fs-sm);
    cursor: pointer;
    transition: background-color var(--t-fast), color var(--t-fast), border-color var(--t-fast);
  }
  .pchip:hover {
    color: var(--text-1);
  }
  .pchip[aria-checked='true'] {
    background: var(--accent-soft);
    border-color: var(--accent-line);
    color: var(--accent-text);
  }
  .num {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
  }
  .num .input {
    width: 8rem;
  }
  .unit {
    color: var(--text-2);
    font-size: var(--fs-sm);
  }
  .hint {
    margin: 0;
    color: var(--text-2);
    font-size: var(--fs-xs);
  }
  .hint.risky {
    color: var(--ask);
  }
  .caution {
    font-size: var(--fs-xs);
    padding: 0.25rem 0.5rem;
  }
  @container (max-width: 34rem) {
    .row {
      grid-template-columns: minmax(0, 1fr);
    }
    .lbl-col {
      padding-top: 0;
    }
  }
</style>
