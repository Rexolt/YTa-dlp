<script lang="ts">
  import { fade, scale } from 'svelte/transition';
  import { quintOut } from 'svelte/easing';
  import { Download, Loader2, CheckCircle2 } from 'lucide-svelte';
  import { downloads } from '$lib/stores/downloads';
  import { cn } from '$lib/utils';

  const active = downloads.active;
  const queued = downloads.queued;

  const activeCount = $derived($active.filter((i) => i.status === 'downloading').length);
  const queuedCount = $derived($queued.length);
  const totalBusy = $derived(activeCount + queuedCount);

  // Aggregate percent across active downloads (mean, for a single visual bar).
  const avgPercent = $derived(
    activeCount === 0
      ? 0
      : Math.round(
          $active
            .filter((i) => i.status === 'downloading')
            .reduce((sum, i) => sum + (i.percent || 0), 0) / activeCount
        )
  );

  // Briefly flash a "done" state when a download just finished and nothing is busy.
  let lastBusy = $state(false);
  let justFinished = $state(false);
  let flashTimeout: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    const isBusy = totalBusy > 0;
    if (lastBusy && !isBusy) {
      justFinished = true;
      if (flashTimeout) clearTimeout(flashTimeout);
      flashTimeout = setTimeout(() => (justFinished = false), 2200);
    }
    lastBusy = isBusy;
  });
</script>

{#if totalBusy > 0}
  <div
    in:scale={{ duration: 220, start: 0.85, easing: quintOut }}
    out:fade={{ duration: 180 }}
    class="relative flex items-center gap-2 rounded-full border border-primary/30 bg-primary/10 pl-2.5 pr-3 py-1 text-[11px] text-primary shadow-[0_8px_24px_-12px_hsl(var(--primary)/0.6)]"
    title={`${activeCount} downloading · ${queuedCount} queued`}
  >
    <span class="relative grid h-5 w-5 place-items-center">
      {#if activeCount > 0}
        <Loader2 class="h-3.5 w-3.5 animate-spin" />
      {:else}
        <Download class="h-3.5 w-3.5" />
      {/if}
    </span>
    <span class="font-medium tabular-nums">
      {activeCount > 0 ? `${activeCount} downloading` : `${queuedCount} queued`}
    </span>
    {#if activeCount > 0}
      <span class="font-mono text-[10px] opacity-80">· {avgPercent}%</span>
    {/if}
    {#if queuedCount > 0 && activeCount > 0}
      <span class="rounded-full bg-primary-foreground/15 px-1.5 py-px text-[10px] tabular-nums">
        +{queuedCount}
      </span>
    {/if}

    <!-- Mini progress bar at the bottom of the pill -->
    {#if activeCount > 0}
      <span
        aria-hidden="true"
        class="pointer-events-none absolute inset-x-2 bottom-0.5 h-[2px] overflow-hidden rounded-full bg-primary/15"
      >
        <span
          class={cn('block h-full bg-primary transition-[width] duration-300 ease-out')}
          style={`width: ${Math.max(2, avgPercent)}%`}
        ></span>
      </span>
    {/if}
  </div>
{:else if justFinished}
  <div
    in:scale={{ duration: 200, start: 0.9 }}
    out:fade={{ duration: 250 }}
    class="flex items-center gap-1.5 rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2.5 py-1 text-[11px] text-emerald-400"
  >
    <CheckCircle2 class="h-3 w-3" />
    <span>Done</span>
  </div>
{/if}
