<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { beforeNavigate, goto } from '$app/navigation';
  import { initPrefs, toggleSidebar } from '$lib/prefs.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import Toaster from '$lib/components/Toaster.svelte';
  import { page } from '$app/stores';
  import { app } from '$lib/state.svelte';
  import {
    confirmDiscardChanges,
    hasUnsavedChanges,
    protectBeforeUnload
  } from '$lib/unsaved';

  let { children } = $props();

  // Remount the project page when another project is opened from the sidebar so
  // its file watcher and per-project UI state start fresh.
  const projectKey = $derived($page.url.pathname === '/project' ? app.projectRoot : '');

  beforeNavigate(({ cancel }) => {
    if (!confirmDiscardChanges(hasUnsavedChanges(), window.confirm.bind(window))) cancel();
  });

  function onKeydown(e: KeyboardEvent) {
    if (!(e.ctrlKey || e.metaKey) || e.altKey || e.shiftKey) return;
    if (e.key === 'b' || e.key === 'B') {
      e.preventDefault();
      toggleSidebar();
    } else if (e.key === ',') {
      e.preventDefault();
      goto('/settings');
    }
  }

  onMount(() => {
    const disposePrefs = initPrefs();
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
      disposePrefs();
      window.removeEventListener('beforeunload', onBeforeUnload);
      unlistenClose?.();
    };
  });
</script>

<svelte:window onkeydown={onKeydown} />

<div class="shell">
  <Sidebar />
  <div class="shell-content">
    {#key projectKey}
      {@render children()}
    {/key}
  </div>
</div>
<Toaster />

<style>
  .shell {
    display: flex;
    min-height: 100vh;
  }
  .shell-content {
    flex: 1;
    min-width: 0;
  }
</style>
