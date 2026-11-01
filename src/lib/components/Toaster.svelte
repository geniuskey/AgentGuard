<script lang="ts">
  import { dismissToast, toasts } from '$lib/toast.svelte';
</script>

<div class="ag-toasts" aria-live="polite">
  {#each toasts as t (t.id)}
    <div class="ag-toast callout callout-{t.kind}" role={t.kind === 'error' ? 'alert' : 'status'}>
      <span>{t.message}</span>
      {#if t.action}
        <button
          class="btn btn-sm"
          onclick={() => {
            t.action?.run();
            dismissToast(t.id);
          }}>{t.action.label}</button
        >
      {/if}
      <button class="btn btn-ghost btn-sm btn-icon" aria-label="알림 닫기" onclick={() => dismissToast(t.id)}>
        <svg viewBox="0 0 24 24" width="13" height="13" fill="none" aria-hidden="true">
          <path d="M6 6l12 12M18 6 6 18" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
        </svg>
      </button>
    </div>
  {/each}
</div>
