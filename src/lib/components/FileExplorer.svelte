<script lang="ts">
  // Iteration 1: lazy project file tree with policy-state badges (ui-spec.md §4).
  let { projectRoot = '' }: { projectRoot?: string } = $props();
</script>

<div class="panel">
  <h3>File Explorer</h3>
  <p class="stub">{projectRoot || '프로젝트 미선택'} — 트리는 다음 반복에서 구현됩니다.</p>
</div>

<style>
  .panel { padding: 0.75rem; }
  .stub { color: #64748b; font-size: 0.85rem; }
</style>
