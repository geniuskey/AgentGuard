<script lang="ts">
  import { app } from '$lib/state.svelte';
  import TreeNode from './TreeNode.svelte';
</script>

<div class="panel">
  <div class="head">
    <span class="ptitle">{app.projectName || 'Files'}</span>
    <button class="root" class:active={app.selectedPath === ''} onclick={() => (app.selectedPath = '')}>
      / root
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
  .panel {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
    padding: 0.55rem 0.7rem;
    border-bottom: 1px solid var(--border);
    font-size: 0.8rem;
  }
  .ptitle {
    font-weight: 600;
    letter-spacing: 0.02em;
    color: var(--text-2);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .root {
    font-size: 0.7rem;
    font-family: var(--font-mono);
    background: none;
    border: 1px solid var(--border-strong);
    color: var(--text-2);
    border-radius: 4px;
    padding: 0.12rem 0.45rem;
    cursor: pointer;
    flex-shrink: 0;
    transition: color var(--t-fast), border-color var(--t-fast), background-color var(--t-fast);
  }
  .root:hover {
    color: var(--text-1);
    background: var(--bg-2);
  }
  .root.active {
    border-color: rgba(79, 142, 247, 0.45);
    background: var(--accent-soft);
    color: var(--accent-text);
  }
  .tree {
    overflow: auto;
    padding: 0.4rem;
    flex: 1;
  }
  .muted {
    color: var(--text-3);
    font-size: 0.85rem;
  }
</style>
