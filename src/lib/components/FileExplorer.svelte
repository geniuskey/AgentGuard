<script lang="ts">
  import { app } from '$lib/state.svelte';
  import TreeNode from './TreeNode.svelte';
</script>

<div class="panel">
  <div class="head">
    <span>{app.projectName || 'Files'}</span>
    <button class="root" class:active={app.selectedPath === ''} onclick={() => (app.selectedPath = '')}>
      / (root)
    </button>
  </div>
  <div class="tree">
    {#if app.view}
      {#each app.view.tree as entry (entry.path)}
        <TreeNode {entry} />
      {/each}
    {:else}
      <p class="muted">프로젝트가 열려 있지 않습니다.</p>
    {/if}
  </div>
</div>

<style>
  .panel { display: flex; flex-direction: column; height: 100%; }
  .head {
    display: flex; justify-content: space-between; align-items: center;
    padding: 0.5rem 0.6rem; border-bottom: 1px solid #1e293b; font-weight: 600; font-size: 0.85rem;
  }
  .root {
    font-size: 0.72rem; background: none; border: 1px solid #334155; color: #94a3b8;
    border-radius: 4px; padding: 0.1rem 0.4rem; cursor: pointer;
  }
  .root.active { border-color: #2563eb; color: #93c5fd; }
  .tree { overflow: auto; padding: 0.35rem; flex: 1; }
  .muted { color: #64748b; font-size: 0.85rem; }
</style>
