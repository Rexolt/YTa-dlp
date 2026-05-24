<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import { Layers, Clock, History, Trash2, SlidersHorizontal } from 'lucide-svelte';
  import DownloadCard from './DownloadCard.svelte';
  import { downloads } from '$lib/stores/downloads';
  import { settings } from '$lib/stores/settings';
  import { ipc } from '$lib/ipc';
  import { cn } from '$lib/utils';

  type Tab = 'active' | 'queued' | 'history';
  let tab: Tab = $state('active');

  const active = downloads.active;
  const queued = downloads.queued;
  const history = downloads.history;

  const tabs: Array<{ id: Tab; label: string; Icon: typeof Layers; count: () => number }> = [
    { id: 'active',  label: 'Active',  Icon: Layers,  count: () => $active.length },
    { id: 'queued',  label: 'Queued',  Icon: Clock,   count: () => $queued.length },
    { id: 'history', label: 'History', Icon: History, count: () => $history.length }
  ];

  async function onConcurrencyChange(e: Event) {
    const n = Number((e.target as HTMLInputElement).value);
    settings.update((s) => ({ ...s, maxConcurrent: n }));
    try { await ipc.setMaxConcurrent(n); } catch { /* ignore */ }
  }

  async function clearHistory() {
    await ipc.clearHistory();
    await downloads.hydrate();
  }
</script>

<section class="mt-12 mx-auto max-w-3xl" in:fade={{ duration: 250 }}>
  <header class="mb-3 flex items-center justify-between">
    <div class="flex items-center gap-1.5 rounded-xl glass p-1">
      {#each tabs as t}
        <button
          onclick={() => (tab = t.id)}
          class={cn(
            'flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-medium transition',
            tab === t.id
              ? 'bg-primary text-primary-foreground shadow-sm'
              : 'text-muted-foreground hover:text-foreground hover:bg-muted/60'
          )}
        >
          <t.Icon class="h-3 w-3" />
          <span>{t.label}</span>
          <span class={cn(
            'rounded-full px-1.5 py-px text-[10px] tabular-nums',
            tab === t.id ? 'bg-primary-foreground/15' : 'bg-secondary/60 text-muted-foreground'
          )}>{t.count()}</span>
        </button>
      {/each}
    </div>

    <div class="flex items-center gap-2">
      <label class="hidden sm:flex items-center gap-2 rounded-xl glass px-3 py-1.5 text-[11px] text-muted-foreground">
        <SlidersHorizontal class="h-3 w-3" />
        Parallel
        <input
          type="range" min="1" max="8" step="1"
          value={$settings.maxConcurrent}
          oninput={onConcurrencyChange}
          class="w-20 accent-[hsl(var(--primary))]"
        />
        <span class="w-4 text-right font-mono text-foreground">{$settings.maxConcurrent}</span>
      </label>
      {#if tab === 'history' && $history.length > 0}
        <button
          onclick={clearHistory}
          class="flex items-center gap-1 rounded-xl glass px-3 py-1.5 text-[11px] text-muted-foreground hover:text-foreground transition"
        >
          <Trash2 class="h-3 w-3" /> Clear all
        </button>
      {/if}
    </div>
  </header>

  {#if tab === 'active'}
    {#if $active.length === 0}
      {@render EmptyState({ message: 'No active downloads. Paste a link above to start one.' })}
    {:else}
      <ul class="grid gap-3">{#each $active as item (item.id)}<DownloadCard {item} />{/each}</ul>
    {/if}
  {:else if tab === 'queued'}
    {#if $queued.length === 0}
      {@render EmptyState({ message: 'The queue is empty.' })}
    {:else}
      <ul class="grid gap-3">{#each $queued as item (item.id)}<DownloadCard {item} />{/each}</ul>
    {/if}
  {:else}
    {#if $history.length === 0}
      {@render EmptyState({ message: 'No history yet — your finished downloads will live here.' })}
    {:else}
      <ul class="grid gap-3">{#each $history as item (item.id)}<DownloadCard {item} />{/each}</ul>
    {/if}
  {/if}
</section>

{#snippet EmptyState({ message }: { message: string })}
  <div in:fly={{ y: 8, duration: 220 }} class="rounded-2xl border border-dashed border-border/60 px-6 py-10 text-center text-sm text-muted-foreground">
    {message}
  </div>
{/snippet}
