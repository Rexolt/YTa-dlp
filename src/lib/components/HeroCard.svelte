<script lang="ts">
  import { slide, fade } from 'svelte/transition';
  import { quintOut } from 'svelte/easing';
  import {
    Calendar, Eye, ThumbsUp, Languages, BookOpen, Shield, Clock, ChevronDown, ChevronUp, ExternalLink
  } from 'lucide-svelte';
  import { cn } from '$lib/utils';
  import type { ProbeResult } from '$lib/ipc';

  interface Props {
    data: ProbeResult;
  }

  let { data }: Props = $props();

  let descExpanded = $state(false);

  // Format views and likes (e.g. 1.2M, 34.5K)
  function formatNumber(num: number | null): string {
    if (num === null || num === undefined) return '—';
    if (num >= 1_000_000) {
      return (num / 1_000_000).toFixed(1).replace(/\.0$/, '') + 'M';
    }
    if (num >= 1_000) {
      return (num / 1_000).toFixed(1).replace(/\.0$/, '') + 'K';
    }
    return num.toLocaleString();
  }

  // Format duration in seconds to hh:mm:ss or mm:ss
  function formatDuration(secs: number | null): string {
    if (secs === null || secs === undefined) return '—:—';
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    const s = Math.floor(secs % 60);
    const pad = (n: number) => n.toString().padStart(2, '0');
    if (h > 0) {
      return `${h}:${pad(m)}:${pad(s)}`;
    }
    return `${m}:${pad(s)}`;
  }

  // Format date from YYYYMMDD to YYYY-MM-DD
  function formatDate(rawDate: string | null): string {
    if (!rawDate) return '—';
    if (/^\d{8}$/.test(rawDate)) {
      return `${rawDate.slice(0, 4)}-${rawDate.slice(4, 6)}-${rawDate.slice(6, 8)}`;
    }
    return rawDate;
  }
</script>

<div
  in:fade={{ duration: 300 }}
  class="w-full max-w-2xl mx-auto overflow-hidden rounded-2xl glass p-4 sm:p-5 flex flex-col gap-4 shadow-xl transition-all duration-300 hover:shadow-2xl hover:border-primary/30"
>
  <div class="flex flex-col sm:flex-row gap-4 items-start">
    <!-- Left column: Thumbnail with aspect ratio 16:9 and duration overlay -->
    <div class="relative w-full sm:w-[240px] shrink-0 aspect-video rounded-xl overflow-hidden bg-black/40 border border-border/40 shadow-inner group">
      {#if data.thumbnail}
        <img
          src={data.thumbnail}
          alt={data.title ?? 'Video thumbnail'}
          class="w-full h-full object-cover transition-transform duration-500 group-hover:scale-105"
          loading="lazy"
        />
      {:else}
        <div class="w-full h-full flex items-center justify-center text-muted-foreground text-xs">
          No image
        </div>
      {/if}
      <div class="absolute bottom-2 right-2 flex items-center gap-1 rounded bg-black/75 px-1.5 py-0.5 font-mono text-[10px] font-medium text-white tracking-wide shadow backdrop-blur-sm">
        <Clock class="h-2.5 w-2.5" />
        {formatDuration(data.durationSecs)}
      </div>
    </div>

    <!-- Right column: Metadata details -->
    <div class="flex-1 min-w-0 flex flex-col justify-between self-stretch py-0.5">
      <div class="space-y-1.5">
        <h2 class="text-base sm:text-lg font-bold leading-snug tracking-tight text-foreground line-clamp-2 hover:line-clamp-none transition-all duration-200">
          {data.title ?? 'Untitled Video'}
        </h2>

        <div class="flex items-center gap-1.5 flex-wrap">
          {#if data.uploader}
            {#if data.uploaderUrl}
              <a
                href={data.uploaderUrl}
                target="_blank"
                rel="noopener noreferrer"
                class="text-xs font-semibold text-primary hover:text-primary/80 transition flex items-center gap-0.5"
              >
                {data.uploader}
                <ExternalLink class="h-2.5 w-2.5 inline" />
              </a>
            {:else}
              <span class="text-xs font-semibold text-muted-foreground">{data.uploader}</span>
            {/if}
          {/if}
          {#if data.channel && data.channel !== data.uploader}
            <span class="text-[10px] text-muted-foreground/80 px-1.5 py-0.5 rounded bg-muted/50">
              {data.channel}
            </span>
          {/if}
        </div>
      </div>

      <!-- Statistics bar -->
      <div class="flex items-center gap-4 text-[11px] text-muted-foreground/90 border-t border-border/40 pt-3 mt-3 sm:mt-0 flex-wrap">
        <span class="flex items-center gap-1">
          <Eye class="h-3 w-3 text-muted-foreground/60" />
          <span>{formatNumber(data.viewCount)} views</span>
        </span>
        {#if data.likeCount}
          <span class="flex items-center gap-1">
            <ThumbsUp class="h-3 w-3 text-muted-foreground/60" />
            <span>{formatNumber(data.likeCount)} likes</span>
          </span>
        {/if}
        {#if data.uploadDate}
          <span class="flex items-center gap-1">
            <Calendar class="h-3 w-3 text-muted-foreground/60" />
            <span>{formatDate(data.uploadDate)}</span>
          </span>
        {/if}
      </div>
    </div>
  </div>

  <!-- Styled Badge Summary of formats and features -->
  <div class="flex flex-wrap gap-1.5 border-t border-border/40 pt-3 text-[10px] sm:text-xs">
    {#if data.subtitles && data.subtitles.length > 0}
      {@const manuals = data.subtitles.filter(s => s.kind === 'manual').length}
      {@const autos = data.subtitles.filter(s => s.kind === 'auto').length}
      <div class="flex items-center gap-1 rounded-full bg-blue-500/10 border border-blue-500/20 px-2.5 py-1 text-blue-400 font-medium">
        <Languages class="h-3.5 w-3.5" />
        <span>{manuals > 0 ? `${manuals} manual` : ''}{manuals > 0 && autos > 0 ? ' + ' : ''}{autos > 0 ? `${autos} auto` : ''} subtitles</span>
      </div>
    {/if}

    {#if data.chapters && data.chapters.length > 0}
      <div class="flex items-center gap-1 rounded-full bg-amber-500/10 border border-amber-500/20 px-2.5 py-1 text-amber-400 font-medium">
        <BookOpen class="h-3.5 w-3.5" />
        <span>{data.chapters.length} chapters</span>
      </div>
    {/if}

    {#if data.sponsorSegments && data.sponsorSegments.length > 0}
      <div class="flex items-center gap-1 rounded-full bg-emerald-500/10 border border-emerald-500/20 px-2.5 py-1 text-emerald-400 font-medium">
        <Shield class="h-3.5 w-3.5" />
        <span>{data.sponsorSegments.length} SponsorBlock segments</span>
      </div>
    {/if}
  </div>

  <!-- Expandable Description Drawer -->
  {#if data.description}
    <div class="border-t border-border/40 pt-2 text-[11px] sm:text-xs">
      <button
        onclick={() => descExpanded = !descExpanded}
        class="flex items-center justify-between w-full py-1 text-muted-foreground hover:text-foreground font-medium transition"
      >
        <span>Description Preview</span>
        <span class="flex items-center gap-1">
          {descExpanded ? 'Collapse' : 'Expand'}
          {#if descExpanded}
            <ChevronUp class="h-3.5 w-3.5" />
          {:else}
            <ChevronDown class="h-3.5 w-3.5" />
          {/if}
        </span>
      </button>
      {#if descExpanded}
        <div
          transition:slide={{ duration: 200, easing: quintOut }}
          class="mt-1.5 text-muted-foreground leading-relaxed whitespace-pre-wrap max-h-48 overflow-y-auto bg-secondary/35 p-3 rounded-lg border border-border/30 font-mono text-[10px] sm:text-[11px]"
        >
          {data.description.trim()}
        </div>
      {/if}
    </div>
  {/if}
</div>
