<script lang="ts">
  // Iteration 1+: see docs/ui-spec.md.
  let { selectedPath = '' }: { selectedPath?: string } = $props();
</script>

<div class="panel">
  <h3>PolicyEditor</h3>
  <p class="stub">{selectedPath || '—'} · 다음 반복에서 구현됩니다.</p>
</div>

<style>
  .panel { padding: 0.75rem; }
  .stub { color: #64748b; font-size: 0.85rem; }
</style>
