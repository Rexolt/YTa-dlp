<script lang="ts">
  import { fly } from 'svelte/transition';
  import {
    Pause, Play, RotateCw, X, Trash2, CheckCircle2, AlertTriangle, Loader2, Clock, FolderOpen
  } from 'lucide-svelte';
  import type { DownloadItem } from '$lib/types';
  import { ipc } from '$lib/ipc';
  import { cn, formatBytes, formatEta, formatSpeed } from '$lib/utils';

  interface Props { item: DownloadItem; }
  let { item }: Props = $props();

  const statusMeta = $derived.by(() => {
    switch (item.status) {
      case 'downloading': return { label: 'Downloading',  color: 'text-primary',           Icon: Loader2,        spin: true };
      case 'queued':      return { label: 'Queued',       color: 'text-muted-foreground',  Icon: Clock,          spin: false };
      case 'paused':      return { label: 'Paused',       color: 'text-amber-400',         Icon: Pause,          spin: false };
      case 'finished':    return { label: 'Finished',     color: 'text-emerald-400',       Icon: CheckCircle2,   spin: false };
      case 'error':       return { label: 'Failed',       color: 'text-destructive',       Icon: AlertTriangle,  spin: false };
      case 'canceled':    return { label: 'Canceled',     color: 'text-muted-foreground',  Icon: X,              spin: false };
      default:            return { label: item.status,    color: 'text-muted-foreground',  Icon: Clock,          spin: false };
    }
  });
</script>

<li in:fly={{ y: 8, duration: 220 }} class="glass rounded-2xl p-4">
  <div class="flex items-start justify-between gap-4">
    <div class="min-w-0 flex-1">
      <div class="flex items-center gap-2">
        <span class={cn('inline-flex items-center gap-1.5 text-[11px] font-medium', statusMeta.color)}>
          <statusMeta.Icon class={cn('h-3 w-3', statusMeta.spin && 'animate-spin')} />
          {statusMeta.label}
          {#if item.attempts > 1}
            <span class="text-muted-foreground">· attempt {item.attempts}</span>
          {/if}
        </span>
      </div>
      <div class="mt-0.5 truncate text-sm font-medium">{item.title ?? item.url}</div>
      <div class="truncate text-[11px] text-muted-foreground">{item.url}</div>
    </div>

    <div class="text-right text-[11px] tabular-nums text-muted-foreground">
      <div>{formatSpeed(item.speedBps)}</div>
      <div>ETA {formatEta(item.etaSecs)}</div>
    </div>
  </div>

  <div class="mt-3">
    <div class="relative h-1.5 w-full overflow-hidden rounded-full bg-muted">
      <div
        class={cn(
          'absolute inset-y-0 left-0 rounded-full transition-[width] duration-200',
          item.status === 'error'    && 'bg-destructive',
          item.status === 'finished' && 'bg-emerald-500',
          (item.status === 'downloading' || item.status === 'queued') && 'bg-gradient-to-r from-primary to-fuchsia-500',
          item.status === 'paused'   && 'bg-amber-500',
          item.status === 'canceled' && 'bg-muted-foreground/40'
        )}
        style="width: {Math.max(0, Math.min(100, item.percent ?? 0))}%"
      ></div>
      {#if item.status === 'downloading'}
        <div class="absolute inset-0 animate-shimmer bg-[linear-gradient(90deg,transparent,hsl(var(--primary)/0.18),transparent)] bg-[length:200%_100%]"></div>
      {/if}
    </div>
    <div class="mt-1.5 flex items-center justify-between text-[11px] text-muted-foreground tabular-nums">
      <span>
        {formatBytes(item.downloaded)} / {item.total ? formatBytes(item.total) : '—'}
      </span>
      <span>{Math.round(item.percent ?? 0)}%</span>
    </div>
  </div>

  <div class="mt-3 flex items-center justify-end gap-1.5">
    {#if item.status === 'downloading'}
      <button onclick={() => ipc.pauseDownload(item.id)}
        class="flex items-center gap-1 rounded-md border border-border/60 bg-secondary/50 px-2.5 py-1 text-[11px] hover:bg-secondary transition">
        <Pause class="h-3 w-3" /> Pause
      </button>
      <button onclick={() => ipc.cancelDownload(item.id)}
        class="flex items-center gap-1 rounded-md border border-destructive/40 bg-destructive/10 px-2.5 py-1 text-[11px] text-destructive-foreground hover:bg-destructive/20 transition">
        <X class="h-3 w-3" /> Cancel
      </button>
    {:else if item.status === 'paused' || item.status === 'queued'}
      <button onclick={() => ipc.resumeDownload(item.id)}
        class="flex items-center gap-1 rounded-md border border-primary/40 bg-primary/10 px-2.5 py-1 text-[11px] text-foreground hover:bg-primary/20 transition">
        <Play class="h-3 w-3" /> {item.status === 'paused' ? 'Resume' : 'Start now'}
      </button>
      <button onclick={() => ipc.cancelDownload(item.id)}
        class="flex items-center gap-1 rounded-md border border-border/60 bg-secondary/50 px-2.5 py-1 text-[11px] hover:bg-secondary transition">
        <X class="h-3 w-3" /> Cancel
      </button>
    {:else if item.status === 'error' || item.status === 'canceled'}
      <button onclick={() => ipc.retryDownload(item.id)}
        class="flex items-center gap-1 rounded-md border border-primary/40 bg-primary/10 px-2.5 py-1 text-[11px] hover:bg-primary/20 transition">
        <RotateCw class="h-3 w-3" /> Retry
      </button>
      <button onclick={() => ipc.deleteDownload(item.id)}
        class="flex items-center gap-1 rounded-md border border-border/60 bg-secondary/50 px-2.5 py-1 text-[11px] hover:bg-secondary transition">
        <Trash2 class="h-3 w-3" /> Remove
      </button>
    {:else if item.status === 'finished'}
      {#if item.outputPath}
        <button onclick={() => ipc.revealInFileManager(item.outputPath!)}
          class="flex items-center gap-1 rounded-md border border-primary/40 bg-primary/10 px-2.5 py-1 text-[11px] text-foreground hover:bg-primary/20 transition">
          <FolderOpen class="h-3 w-3" /> Open folder
        </button>
      {/if}
      <button onclick={() => ipc.deleteDownload(item.id)}
        class="flex items-center gap-1 rounded-md border border-border/60 bg-secondary/50 px-2.5 py-1 text-[11px] hover:bg-secondary transition">
        <Trash2 class="h-3 w-3" /> Remove
      </button>
    {/if}
  </div>

  {#if item.error}
    <div class="mt-2 rounded-md border border-destructive/30 bg-destructive/10 px-2.5 py-1.5 text-[11px] text-destructive-foreground">
      {item.error}
    </div>
  {/if}
</li>
