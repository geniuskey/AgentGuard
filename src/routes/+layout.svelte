<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { beforeNavigate } from '$app/navigation';
  import { initTheme } from '$lib/theme.svelte';
  import {
    confirmDiscardChanges,
    hasUnsavedChanges,
    protectBeforeUnload
  } from '$lib/unsaved';

  let { children } = $props();

  beforeNavigate(({ cancel }) => {
    if (!confirmDiscardChanges(hasUnsavedChanges(), window.confirm.bind(window))) cancel();
  });

  onMount(() => {
    initTheme();
    let allowClose = false;
    let disposed = false;
    let unlistenClose: (() => void) | undefined;

    const onBeforeUnload = (event: BeforeUnloadEvent) => {
      if (!allowClose) protectBeforeUnload(event, hasUnsavedChanges());
    };
    window.addEventListener('beforeunload', onBeforeUnload);

    // Tauri emits a close request before tearing down the webview. Handling it
    // explicitly keeps the same protection for the native title-bar close button.
    if ('__TAURI_INTERNALS__' in window) {
      import('@tauri-apps/api/window')
        .then(async ({ getCurrentWindow }) => {
          const appWindow = getCurrentWindow();
          const unlisten = await appWindow.onCloseRequested(async (event) => {
            if (allowClose || !hasUnsavedChanges()) return;
            event.preventDefault();
            if (confirmDiscardChanges(true, window.confirm.bind(window))) {
              allowClose = true;
              await appWindow.close();
            }
          });
          if (disposed) unlisten();
          else unlistenClose = unlisten;
        })
        .catch(() => {
          // Browser preview and older webviews still retain beforeunload protection.
        });
    }

    return () => {
      disposed = true;
      window.removeEventListener('beforeunload', onBeforeUnload);
      unlistenClose?.();
    };
  });
</script>

<div class="app">
  {@render children()}
</div>

<style>
  .app {
    min-height: 100vh;
  }
</style>
