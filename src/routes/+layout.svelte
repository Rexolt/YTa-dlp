<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { downloads } from '$lib/stores/downloads';
  import { settings } from '$lib/stores/settings';
  import { ipc } from '$lib/ipc';
  import { get } from 'svelte/store';

  let { children } = $props();

  onMount(() => {
    // Hydrate from persisted DB.
    downloads.hydrate().catch(() => { /* ignore */ });

    // Push current settings to backend (max concurrent).
    settings.syncToBackend(get(settings)).catch(() => { /* ignore */ });

    const offs: Promise<() => void>[] = [
      ipc.onStarted((p) => downloads.handleStarted(p)),
      ipc.onProgress((p) => downloads.handleProgress(p)),
      ipc.onFinished((p) => downloads.handleFinished(p)),
      ipc.onQueueChanged(() => { /* could update a header indicator */ }),
      ipc.onLog(() => { /* attach log viewer later */ })
    ];

    return () => { offs.forEach((u) => u.then((fn) => fn())); };
  });
</script>

<div class="relative min-h-screen">
  <div class="ambient-orbs"></div>
  <div class="relative z-10">
    {@render children?.()}
  </div>
</div>
