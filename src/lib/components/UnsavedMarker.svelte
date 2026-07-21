<script lang="ts">
  import { clearUnsavedSource, setUnsavedSource } from '$lib/unsaved';

  let { id, when }: { id: string; when: boolean } = $props();

  $effect(() => {
    const source = id;
    setUnsavedSource(source, when);
    return () => clearUnsavedSource(source);
  });
</script>
