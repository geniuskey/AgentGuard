<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { initTheme } from '$lib/theme.svelte';

  let { children } = $props();

  onMount(initTheme);
</script>

<div class="app">
  {@render children()}
</div>

<style>
  .app {
    min-height: 100vh;
  }
</style>
