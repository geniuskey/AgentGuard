<script lang="ts">
  let { children } = $props();
</script>

<div class="app">
  {@render children()}
</div>

<style>
  :global(body) {
    margin: 0;
    font-family: system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif;
    background: #0f172a;
    color: #e2e8f0;
  }
  .app {
    min-height: 100vh;
  }
</style>
