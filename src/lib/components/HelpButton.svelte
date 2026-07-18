<script lang="ts">
  // Shared help affordance. Opens the in-app guide, optionally jumping straight
  // to the section relevant to the current screen via ?s=<key>.
  import { goto } from '$app/navigation';

  let {
    section = '',
    compact = false,
    title = '사용 가이드 보기'
  }: { section?: string; compact?: boolean; title?: string } = $props();

  function open() {
    goto(section ? `/guide?s=${encodeURIComponent(section)}` : '/guide');
  }
</script>

<button class="help" class:compact onclick={open} {title} aria-label={title}>
  <svg viewBox="0 0 24 24" width="15" height="15" fill="none" aria-hidden="true">
    <circle cx="12" cy="12" r="9" stroke="currentColor" stroke-width="1.6" />
    <path
      d="M9.4 9.4a2.6 2.6 0 0 1 5 .8c0 1.7-2.4 2-2.4 3.6"
      stroke="currentColor"
      stroke-width="1.6"
      stroke-linecap="round"
    />
    <circle cx="12" cy="17" r="1" fill="currentColor" />
  </svg>
  {#if !compact}<span>도움말</span>{/if}
</button>

<style>
  .help {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    background: var(--bg-1);
    border: 1px solid var(--border-strong);
    color: var(--text-2);
    border-radius: 999px;
    padding: 0.34rem 0.8rem;
    cursor: pointer;
    font-size: 0.76rem;
    font-weight: 600;
    transition: color var(--t-fast), border-color var(--t-fast), background-color var(--t-fast);
  }
  .help:hover {
    color: var(--text-1);
    border-color: var(--accent);
    background: var(--bg-2);
  }
  .help.compact {
    padding: 0;
    width: 1.85rem;
    height: 1.85rem;
    justify-content: center;
    gap: 0;
  }
</style>
