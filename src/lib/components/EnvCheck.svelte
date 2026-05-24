<script lang="ts">
  import { onMount } from 'svelte';
  import { fade, scale } from 'svelte/transition';
  import { quintOut } from 'svelte/easing';
  import {
    CheckCircle2, AlertTriangle, RefreshCw, Download, Loader2,
    Terminal, Package, Cpu, X
  } from 'lucide-svelte';
  import { ipc, type EnvironmentReport, type ToolStatus } from '$lib/ipc';
  import { cn } from '$lib/utils';

  let report = $state<EnvironmentReport | null>(null);
  let loading = $state(true);
  let updating = $state(false);
  let dismissed = $state(false);
  let updateOutput = $state<string | null>(null);

  const tools = $derived(report ? [report.ytDlp, report.ffmpeg, report.ffprobe] : []);
  const allOk = $derived(tools.length > 0 && tools.every((t) => t.available));
  const ytDlpOk = $derived(!!report?.ytDlp.available);

  // Show overlay only if something critical (yt-dlp or ffmpeg) is missing.
  const blocking = $derived(
    !loading && report !== null && (!report.ytDlp.available || !report.ffmpeg.available)
  );

  async function refresh() {
    loading = true;
    try { report = await ipc.checkEnvironment(); }
    catch (e) { console.error(e); }
    finally { loading = false; }
  }

  async function updateYtDlp() {
    updating = true;
    updateOutput = null;
    try {
      const res = await ipc.updateYtDlp();
      updateOutput = res.output.trim() || (res.success ? 'Already up to date.' : 'Update failed.');
      await refresh();
    } catch (e) {
      updateOutput = String(e);
    } finally {
      updating = false;
    }
  }

  function sourceBadge(s: ToolStatus['source']) {
    switch (s) {
      case 'packaged':   return { label: 'bundled',  cls: 'bg-emerald-500/15 text-emerald-400 border-emerald-500/30' };
      case 'devSidecar': return { label: 'sidecar',  cls: 'bg-primary/15 text-primary border-primary/30' };
      case 'systemPath': return { label: 'system',   cls: 'bg-amber-500/15 text-amber-400 border-amber-500/30' };
      case 'missing':    return { label: 'missing',  cls: 'bg-destructive/15 text-destructive-foreground border-destructive/30' };
    }
  }

  function toolIcon(name: string) {
    if (name === 'yt-dlp') return Download;
    if (name === 'ffmpeg') return Cpu;
    return Terminal;
  }

  onMount(refresh);
</script>

<!-- Compact header pill (always rendered when report is loaded) -->
{#if report && !blocking}
  <button
    type="button"
    onclick={() => (dismissed = false)}
    class={cn(
      'flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-[11px] transition',
      allOk
        ? 'border-emerald-500/30 bg-emerald-500/10 text-emerald-400 hover:bg-emerald-500/20'
        : 'border-amber-500/30 bg-amber-500/10 text-amber-400 hover:bg-amber-500/20'
    )}
    title="Environment status"
  >
    {#if allOk}
      <CheckCircle2 class="h-3 w-3" />
      <span>Ready</span>
    {:else}
      <AlertTriangle class="h-3 w-3" />
      <span>{tools.filter((t) => !t.available).length} missing</span>
    {/if}
  </button>
{/if}

<!-- Full-screen overlay when something critical is missing or user opens it -->
{#if (blocking && !dismissed) || (report && dismissed === false && !allOk && !blocking)}
  <div
    transition:fade={{ duration: 200 }}
    class="fixed inset-0 z-50 flex items-center justify-center bg-background/70 px-4 backdrop-blur-md"
  >
    <div
      in:scale={{ duration: 240, easing: quintOut, start: 0.96 }}
      class="relative w-full max-w-xl rounded-2xl glass p-6 shadow-[0_30px_120px_-20px_rgba(0,0,0,0.6)]"
    >
      {#if !blocking}
        <button
          onclick={() => (dismissed = true)}
          class="absolute right-3 top-3 grid h-7 w-7 place-items-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground transition"
          aria-label="Dismiss"
        ><X class="h-3.5 w-3.5" /></button>
      {/if}

      <div class="mb-5 flex items-center gap-3">
        <div class={cn(
          'grid h-10 w-10 place-items-center rounded-xl',
          allOk ? 'bg-emerald-500/15 text-emerald-400' : 'bg-amber-500/15 text-amber-400'
        )}>
          {#if loading}
            <Loader2 class="h-5 w-5 animate-spin" />
          {:else if allOk}
            <CheckCircle2 class="h-5 w-5" />
          {:else}
            <Package class="h-5 w-5" />
          {/if}
        </div>
        <div>
          <h2 class="text-base font-semibold tracking-tight">Environment check</h2>
          <p class="text-xs text-muted-foreground">
            YTa-dlp needs a few helper binaries to do its job.
          </p>
        </div>
      </div>

      <ul class="space-y-2">
        {#each tools as t (t.name)}
          {@const badge = sourceBadge(t.source)}
          {@const Icon = toolIcon(t.name)}
          <li class="flex items-center gap-3 rounded-xl border border-border/60 bg-secondary/30 p-3">
            <div class={cn(
              'grid h-9 w-9 place-items-center rounded-lg',
              t.available ? 'bg-emerald-500/10 text-emerald-400' : 'bg-destructive/10 text-destructive'
            )}>
              <Icon class="h-4 w-4" />
            </div>
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <span class="text-sm font-medium">{t.name}</span>
                <span class={cn('rounded-md border px-1.5 py-px text-[10px] uppercase tracking-wide', badge.cls)}>
                  {badge.label}
                </span>
              </div>
              {#if t.available}
                <div class="truncate text-[11px] text-muted-foreground">
                  {t.version ?? '—'}{#if t.path} · <span class="font-mono">{t.path}</span>{/if}
                </div>
              {:else}
                <div class="text-[11px] text-destructive-foreground">Not found on PATH or sidecar dir.</div>
              {/if}
            </div>
            {#if t.available}
              <CheckCircle2 class="h-4 w-4 text-emerald-400 shrink-0" />
            {:else}
              <AlertTriangle class="h-4 w-4 text-destructive shrink-0" />
            {/if}
          </li>
        {/each}
      </ul>

      <div class="mt-5 flex flex-wrap items-center justify-between gap-2">
        <p class="text-[11px] text-muted-foreground">
          Run <code class="font-mono text-foreground">pnpm binaries:fetch</code> to install bundled sidecars,
          or place yt-dlp/ffmpeg on your system PATH.
        </p>
        <div class="flex items-center gap-2">
          <button
            onclick={refresh}
            disabled={loading}
            class="flex items-center gap-1.5 rounded-lg border border-border/60 bg-secondary/50 px-3 py-1.5 text-xs hover:bg-secondary transition disabled:opacity-50"
          >
            <RefreshCw class={cn('h-3 w-3', loading && 'animate-spin')} /> Re-check
          </button>
          <button
            onclick={updateYtDlp}
            disabled={!ytDlpOk || updating}
            class="flex items-center gap-1.5 rounded-lg bg-primary px-3 py-1.5 text-xs text-primary-foreground hover:brightness-110 transition disabled:opacity-50"
          >
            {#if updating}<Loader2 class="h-3 w-3 animate-spin" />{:else}<Download class="h-3 w-3" />{/if}
            Update yt-dlp
          </button>
        </div>
      </div>

      {#if updateOutput}
        <pre transition:fade
          class="mt-3 max-h-32 overflow-auto rounded-lg border border-border/60 bg-background/50 p-2 font-mono text-[10px] text-muted-foreground"
        >{updateOutput}</pre>
      {/if}
    </div>
  </div>
{/if}
