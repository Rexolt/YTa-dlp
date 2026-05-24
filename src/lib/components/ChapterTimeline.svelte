<script lang="ts">
  import { fade } from 'svelte/transition';
  import { Clock, Shield, Loader2 } from 'lucide-svelte';
  import { probe } from '$lib/stores/probe';
  import type { SponsorBlockMode } from '$lib/types';
  import { cn, formatEta } from '$lib/utils';

  interface Props {
    /** Selected SponsorBlock categories (highlighted on the timeline). */
    enabledCategories: string[];
    /** SponsorBlock mode (off/mark/remove) — purely visual hint. */
    sbMode: SponsorBlockMode;
  }
  let { enabledCategories, sbMode }: Props = $props();

  const chapters = $derived($probe.data?.chapters ?? []);
  const segments = $derived($probe.data?.sponsorSegments ?? []);
  const duration = $derived(
    $probe.data?.durationSecs ??
      // Fallback: use the largest segment end / chapter end.
      Math.max(
        0,
        ...segments.map((s) => s.end),
        ...chapters.map((c) => c.end)
      )
  );

  // Stable color per category.
  const CATEGORY_COLORS: Record<string, string> = {
    sponsor:        'bg-emerald-500',
    intro:          'bg-blue-500',
    outro:          'bg-indigo-500',
    selfpromo:      'bg-yellow-500',
    interaction:    'bg-pink-500',
    music_offtopic: 'bg-orange-500',
    preview:        'bg-purple-500',
    filler:         'bg-rose-500'
  };

  function color(cat: string) { return CATEGORY_COLORS[cat] ?? 'bg-muted-foreground'; }

  function pct(v: number) {
    if (!duration || !Number.isFinite(duration) || duration <= 0) return 0;
    return Math.max(0, Math.min(100, (v / duration) * 100));
  }

  let hover = $state<{ kind: 'chapter' | 'sb'; idx: number; x: number } | null>(null);

  function onLeave() { hover = null; }
</script>

<div class="space-y-2">
  {#if $probe.status === 'idle'}
    <div class="rounded-lg border border-dashed border-border/60 px-3 py-4 text-center text-[11px] text-muted-foreground">
      Paste a YouTube URL to preview chapters and SponsorBlock segments.
    </div>
  {:else if $probe.status === 'loading'}
    <div class="flex items-center gap-2 rounded-lg border border-border/60 bg-secondary/30 px-3 py-3 text-[11px] text-muted-foreground">
      <Loader2 class="h-3 w-3 animate-spin" /> Probing timeline…
    </div>
  {:else if $probe.status === 'error'}
    <div class="rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-[11px] text-destructive-foreground">
      Probe failed: {$probe.error}
    </div>
  {:else if duration > 0}
    <!-- Header summary -->
    <div class="flex items-center justify-between text-[11px] text-muted-foreground">
      <div class="flex items-center gap-3">
        <span class="inline-flex items-center gap-1">
          <Clock class="h-3 w-3" /> {chapters.length} chapter{chapters.length === 1 ? '' : 's'}
        </span>
        <span class="inline-flex items-center gap-1">
          <Shield class="h-3 w-3" /> {segments.length} SponsorBlock segment{segments.length === 1 ? '' : 's'}
        </span>
      </div>
      <span class="font-mono tabular-nums">{formatEta(duration)}</span>
    </div>

    <!-- Timeline -->
    <div
      class="relative h-10 select-none overflow-hidden rounded-lg border border-border/60 bg-secondary/30"
      onmouseleave={onLeave}
      role="presentation"
    >
      <!-- Chapters band (top half) -->
      <div class="absolute inset-x-0 top-0 h-1/2">
        {#each chapters as ch, i (i)}
          <div
            class="absolute inset-y-0 border-r border-border/40 hover:bg-foreground/5 transition-colors"
            style="left: {pct(ch.start)}%; width: {pct(ch.end - ch.start)}%;"
            onmouseenter={(e) => (hover = { kind: 'chapter', idx: i, x: (e as MouseEvent).offsetX })}
            role="presentation"
          ></div>
        {/each}
      </div>

      <!-- SponsorBlock band (bottom half) -->
      <div class="absolute inset-x-0 bottom-0 h-1/2">
        {#each segments as seg, i (seg.uuid)}
          {@const active = enabledCategories.includes(seg.category)}
          <div
            class={cn(
              'absolute inset-y-0 transition-all',
              color(seg.category),
              active ? 'opacity-90' : 'opacity-30',
              sbMode === 'remove' && active && 'ring-1 ring-inset ring-destructive/50'
            )}
            style="left: {pct(seg.start)}%; width: {pct(seg.end - seg.start)}%;"
            onmouseenter={(e) => (hover = { kind: 'sb', idx: i, x: (e as MouseEvent).offsetX })}
            role="presentation"
            title={`${seg.category} · ${formatEta(seg.start)}–${formatEta(seg.end)}`}
          ></div>
        {/each}
      </div>

      <!-- Mid divider -->
      <div class="pointer-events-none absolute inset-x-0 top-1/2 h-px bg-border/60"></div>

      <!-- Hover tooltip -->
      {#if hover}
        {#if hover.kind === 'chapter' && chapters[hover.idx]}
          {@const ch = chapters[hover.idx]}
          <div
            in:fade={{ duration: 80 }}
            class="pointer-events-none absolute -top-9 z-10 rounded-md border border-border/60 bg-background/95 px-2 py-1 text-[10px] shadow-sm"
            style="left: {pct((ch.start + ch.end) / 2)}%; transform: translateX(-50%);"
          >
            <div class="font-medium text-foreground">{ch.title ?? `Chapter ${hover.idx + 1}`}</div>
            <div class="font-mono tabular-nums text-muted-foreground">
              {formatEta(ch.start)} → {formatEta(ch.end)}
            </div>
          </div>
        {:else if hover.kind === 'sb' && segments[hover.idx]}
          {@const seg = segments[hover.idx]}
          <div
            in:fade={{ duration: 80 }}
            class="pointer-events-none absolute -bottom-9 z-10 rounded-md border border-border/60 bg-background/95 px-2 py-1 text-[10px] shadow-sm"
            style="left: {pct((seg.start + seg.end) / 2)}%; transform: translateX(-50%);"
          >
            <div class="flex items-center gap-1.5">
              <span class={cn('inline-block h-2 w-2 rounded-full', color(seg.category))}></span>
              <span class="font-medium capitalize text-foreground">{seg.category.replace('_', ' ')}</span>
            </div>
            <div class="font-mono tabular-nums text-muted-foreground">
              {formatEta(seg.start)} → {formatEta(seg.end)}
            </div>
          </div>
        {/if}
      {/if}
    </div>

    <!-- Legend -->
    {#if segments.length > 0}
      <div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-[10px] text-muted-foreground">
        {#each Array.from(new Set(segments.map((s) => s.category))) as cat (cat)}
          <span class="inline-flex items-center gap-1">
            <span class={cn('h-2 w-2 rounded-full', color(cat))}></span>
            <span class="capitalize">{cat.replace('_', ' ')}</span>
          </span>
        {/each}
      </div>
    {/if}
  {/if}
</div>
