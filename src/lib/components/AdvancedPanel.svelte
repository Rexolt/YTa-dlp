<script lang="ts">
  import { slide, fade } from 'svelte/transition';
  import { quintOut } from 'svelte/easing';
  import {
    Settings2, ChevronDown, Film, Music2, Subtitles, Shield, Network, ListVideo, Tags
  } from 'lucide-svelte';
  import { cn } from '$lib/utils';
  import type { DownloadOptions } from '$lib/types';
  import SubtitlePicker from './SubtitlePicker.svelte';
  import ChapterTimeline from './ChapterTimeline.svelte';
  import { probe } from '$lib/stores/probe';

  interface Props {
    options: DownloadOptions;
  }

  let { options = $bindable() }: Props = $props();

  let open = $state(false);
  let section = $state<string | null>('format');

  function toggleSection(id: string) {
    section = section === id ? null : id;
  }

  const sections = [
    { id: 'format',       label: 'Format & Quality',  icon: Film,      hint: () => options.kind === 'video' ? `${options.video?.maxHeight ?? 1080}p · ${options.videoContainer ?? 'mp4'}` : `${options.audio?.bitrateKbps ?? 192}k · ${options.audioContainer ?? 'mp3'}` },
    { id: 'audio',        label: 'Audio',             icon: Music2,    hint: () => options.audio?.normalize ? 'normalized' : 'standard' },
    { id: 'subtitles',    label: 'Subtitles',         icon: Subtitles, hint: () => {
      const langs = options.subtitles?.languages ?? [];
      if (langs.length === 0) return 'off';
      const mode = options.subtitles?.mode ?? 'sidecar';
      const head = langs.length > 3 ? `${langs.slice(0, 3).join(',')}+${langs.length - 3}` : langs.join(',');
      return `${head} · ${mode}`;
    } },
    { id: 'metadata',     label: 'Metadata',          icon: Tags,      hint: () => [options.metadata.embedThumbnail && 'thumb', options.metadata.keepChapters && 'chapters', options.metadata.embedMetadata && 'tags'].filter(Boolean).join(' · ') || 'off' },
    { id: 'sponsorblock', label: 'SponsorBlock',      icon: Shield,    hint: () => options.sponsorblock.mode },
    { id: 'network',      label: 'Network & Cookies', icon: Network,   hint: () => options.network.cookiesFromBrowser ?? (options.network.proxy ? 'proxy' : 'direct') },
    { id: 'playlist',     label: 'Playlist',          icon: ListVideo, hint: () => options.playlist.enabled ? (options.playlist.items ?? 'all') : 'off' }
  ];
</script>

<div class="w-full max-w-2xl mx-auto">
  <button
    type="button"
    onclick={() => (open = !open)}
    class={cn(
      'group flex w-full items-center justify-between rounded-2xl glass px-4 py-3 transition-all',
      open && 'rounded-b-none'
    )}
    aria-expanded={open}
  >
    <span class="flex items-center gap-2.5 text-sm">
      <span class="grid h-7 w-7 place-items-center rounded-lg bg-muted text-muted-foreground">
        <Settings2 class="h-3.5 w-3.5" />
      </span>
      <span class="font-medium">Advanced settings</span>
      <span class="hidden sm:inline text-xs text-muted-foreground">
        {options.kind === 'video' ? 'Video' : 'Audio'} · {options.kind === 'video' ? (options.videoContainer ?? 'mp4') : (options.audioContainer ?? 'mp3')}
      </span>
    </span>
    <ChevronDown class={cn('h-4 w-4 text-muted-foreground transition-transform duration-300', open && 'rotate-180 text-foreground')} />
  </button>

  {#if open}
    <div
      transition:slide={{ duration: 280, easing: quintOut }}
      class="overflow-hidden rounded-b-2xl glass border-t-0"
    >
      <!-- Top tabs: Video / Audio -->
      <div class="flex items-center gap-1 p-2 border-b border-border/60">
        {#each ['video', 'audio'] as kind}
          <button
            onclick={() => (options.kind = kind as 'video' | 'audio')}
            class={cn(
              'flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-medium transition',
              options.kind === kind
                ? 'bg-primary text-primary-foreground shadow-sm'
                : 'text-muted-foreground hover:text-foreground hover:bg-muted'
            )}
          >
            {#if kind === 'video'}<Film class="h-3 w-3" />{:else}<Music2 class="h-3 w-3" />{/if}
            <span class="capitalize">{kind}</span>
          </button>
        {/each}
      </div>

      <div class="divide-y divide-border/50">
        {#each sections as s (s.id)}
          <div>
            <button
              type="button"
              onclick={() => toggleSection(s.id)}
              class="flex w-full items-center justify-between px-4 py-3 text-left hover:bg-muted/40 transition"
              aria-expanded={section === s.id}
            >
              <span class="flex items-center gap-3 text-sm">
                <span class="grid h-7 w-7 place-items-center rounded-lg bg-muted/60 text-muted-foreground">
                  <s.icon class="h-3.5 w-3.5" />
                </span>
                <span class="font-medium">{s.label}</span>
              </span>
              <span class="flex items-center gap-2 text-xs text-muted-foreground">
                <span class="rounded-md bg-secondary/60 px-2 py-0.5 font-mono">{s.hint()}</span>
                <ChevronDown class={cn('h-3.5 w-3.5 transition-transform', section === s.id && 'rotate-180')} />
              </span>
            </button>

            {#if section === s.id}
              <div transition:slide={{ duration: 220, easing: quintOut }} class="px-4 pb-4 pt-1">
                <div in:fade={{ duration: 160 }}>
                  {#if s.id === 'format'}
                    {#if options.kind === 'video'}
                      <div class="grid grid-cols-2 gap-3">
                        <label class="block">
                          <span class="block text-[11px] uppercase tracking-wide text-muted-foreground mb-1">Container</span>
                          <div class="flex gap-1">
                            {#each ['mp4','mkv','webm'] as c}
                              <button onclick={() => (options.videoContainer = c as any)}
                                class={cn('flex-1 rounded-md border px-2 py-1.5 text-xs transition',
                                  options.videoContainer === c ? 'border-primary bg-primary/10 text-foreground' : 'border-border/60 text-muted-foreground hover:bg-muted')}>{c.toUpperCase()}</button>
                            {/each}
                          </div>
                        </label>
                        <label class="block">
                          <span class="block text-[11px] uppercase tracking-wide text-muted-foreground mb-1">Resolution</span>
                          <select
                            bind:value={() => options.video?.maxHeight ?? 1080, (v) => options.video = { ...(options.video ?? { fps: 60, maxHeight: 1080 }), maxHeight: Number(v) }}
                            class="w-full rounded-md border border-border/60 bg-secondary/50 px-2 py-1.5 text-xs"
                          >
                            {#each ($probe.status === 'ready' && $probe.data?.availableHeights?.length ? $probe.data.availableHeights : [4320, 2160, 1440, 1080, 720, 480, 360]) as h}
                              <option value={h}>{h === 4320 ? '8K (4320p)' : h === 2160 ? '4K (2160p)' : h === 1440 ? '2K (1440p)' : `${h}p`}</option>
                            {/each}
                          </select>
                        </label>
                        <label class="block col-span-2">
                          <span class="block text-[11px] uppercase tracking-wide text-muted-foreground mb-1">Max FPS</span>
                          <div class="flex gap-1">
                            {#each ($probe.status === 'ready' && $probe.data?.availableFps?.length ? $probe.data.availableFps : [30, 60]) as f}
                              <button onclick={() => options.video = { ...(options.video ?? { maxHeight: 1080, fps: 60 }), fps: f }}
                                class={cn('flex-1 rounded-md border px-2 py-1.5 text-xs transition',
                                  options.video?.fps === f ? 'border-primary bg-primary/10' : 'border-border/60 text-muted-foreground hover:bg-muted')}>{f} fps</button>
                            {/each}
                          </div>
                        </label>
                      </div>
                    {:else}
                      <div class="grid grid-cols-2 gap-3">
                        <label class="block">
                          <span class="block text-[11px] uppercase tracking-wide text-muted-foreground mb-1">Container</span>
                          <div class="grid grid-cols-4 gap-1">
                            {#each ['mp3','m4a','flac','wav'] as c}
                              <button onclick={() => (options.audioContainer = c as any)}
                                class={cn('rounded-md border px-2 py-1.5 text-xs transition',
                                  options.audioContainer === c ? 'border-primary bg-primary/10' : 'border-border/60 text-muted-foreground hover:bg-muted')}>{c.toUpperCase()}</button>
                            {/each}
                          </div>
                        </label>
                        <label class="block">
                          <span class="block text-[11px] uppercase tracking-wide text-muted-foreground mb-1">Bitrate</span>
                          <div class="grid grid-cols-3 gap-1">
                            {#each [128, 192, 320] as b}
                              <button onclick={() => options.audio = { ...(options.audio ?? { bitrateKbps: 192, normalize: false }), bitrateKbps: b }}
                                class={cn('rounded-md border px-2 py-1.5 text-xs transition',
                                  options.audio?.bitrateKbps === b ? 'border-primary bg-primary/10' : 'border-border/60 text-muted-foreground hover:bg-muted')}>{b}k</button>
                            {/each}
                          </div>
                        </label>
                      </div>
                    {/if}
                  {:else if s.id === 'audio'}
                    <label class="flex items-center justify-between text-sm">
                      <span>
                        <span class="font-medium">Loudness normalization</span>
                        <span class="block text-xs text-muted-foreground">EBU R128 (-16 LUFS) via ffmpeg loudnorm.</span>
                      </span>
                      <input type="checkbox"
                        checked={options.audio?.normalize ?? false}
                        onchange={(e) => options.audio = { ...(options.audio ?? { bitrateKbps: 192, normalize: false }), normalize: (e.target as HTMLInputElement).checked }}
                        class="h-5 w-9 appearance-none rounded-full bg-muted before:hidden checked:bg-primary transition-colors"
                      />
                    </label>
                  {:else if s.id === 'subtitles'}
                    {#if !options.subtitles}
                      {void (options.subtitles = { languages: [], autoGenerated: false, mode: 'sidecar', format: 'srt' })}
                    {/if}
                    <SubtitlePicker bind:subtitles={options.subtitles!} container={options.videoContainer} />
                  {:else if s.id === 'metadata'}
                    <div class="grid grid-cols-3 gap-2 text-xs">
                      <label class="flex items-center gap-2">
                        <input type="checkbox" bind:checked={options.metadata.embedThumbnail} /> Embed thumbnail
                      </label>
                      <label class="flex items-center gap-2">
                        <input type="checkbox" bind:checked={options.metadata.keepChapters} /> Keep chapters
                      </label>
                      <label class="flex items-center gap-2">
                        <input type="checkbox" bind:checked={options.metadata.embedMetadata} /> Embed tags
                      </label>
                    </div>
                  {:else if s.id === 'sponsorblock'}
                    <div class="space-y-3 text-sm">
                      <div class="flex gap-1">
                        {#each ['off','mark','remove'] as m}
                          <button onclick={() => (options.sponsorblock.mode = m as any)}
                            class={cn('flex-1 rounded-md border px-2 py-1.5 text-xs capitalize transition',
                              options.sponsorblock.mode === m ? 'border-primary bg-primary/10' : 'border-border/60 text-muted-foreground hover:bg-muted')}>{m}</button>
                        {/each}
                      </div>
                      <div class="flex flex-wrap gap-1.5">
                        {#each ['sponsor','intro','outro','selfpromo','interaction','music_offtopic','preview','filler'] as cat}
                          {@const active = options.sponsorblock.categories.includes(cat)}
                          <button
                            onclick={() => options.sponsorblock.categories = active ? options.sponsorblock.categories.filter(c => c !== cat) : [...options.sponsorblock.categories, cat]}
                            class={cn('rounded-full border px-2.5 py-1 text-[11px] transition',
                              active ? 'border-primary bg-primary/15 text-foreground' : 'border-border/60 text-muted-foreground hover:bg-muted')}>{cat}</button>
                        {/each}
                      </div>
                      <ChapterTimeline
                        enabledCategories={options.sponsorblock.categories}
                        sbMode={options.sponsorblock.mode}
                      />
                    </div>
                  {:else if s.id === 'network'}
                    <div class="grid grid-cols-2 gap-3 text-sm">
                      <label class="block">
                        <span class="block text-[11px] uppercase tracking-wide text-muted-foreground mb-1">Cookies from browser</span>
                        <select
                          value={options.network.cookiesFromBrowser ?? ''}
                          onchange={(e) => options.network.cookiesFromBrowser = (e.target as HTMLSelectElement).value || null}
                          class="w-full rounded-md border border-border/60 bg-secondary/50 px-2 py-1.5 text-xs"
                        >
                          <option value="">None</option>
                          {#each ['chrome','firefox','edge','brave','safari','vivaldi','opera'] as b}
                            <option value={b}>{b}</option>
                          {/each}
                        </select>
                      </label>
                      <label class="block">
                        <span class="block text-[11px] uppercase tracking-wide text-muted-foreground mb-1">Rate limit</span>
                        <input
                          type="text" placeholder="e.g. 2M"
                          value={options.network.rateLimit ?? ''}
                          oninput={(e) => options.network.rateLimit = (e.target as HTMLInputElement).value || null}
                          class="w-full rounded-md border border-border/60 bg-secondary/50 px-2 py-1.5 text-xs"
                        />
                      </label>
                      <label class="block col-span-2">
                        <span class="block text-[11px] uppercase tracking-wide text-muted-foreground mb-1">Proxy</span>
                        <input
                          type="text" placeholder="http://127.0.0.1:8080"
                          value={options.network.proxy ?? ''}
                          oninput={(e) => options.network.proxy = (e.target as HTMLInputElement).value || null}
                          class="w-full rounded-md border border-border/60 bg-secondary/50 px-2 py-1.5 text-xs"
                        />
                      </label>
                      <label class="block col-span-2">
                        <span class="block text-[11px] uppercase tracking-wide text-muted-foreground mb-1">
                          Concurrent fragments: <span class="font-mono">{options.network.concurrentFragments}</span>
                        </span>
                        <input
                          type="range" min="1" max="16" step="1"
                          bind:value={options.network.concurrentFragments}
                          class="w-full accent-[hsl(var(--primary))]"
                        />
                      </label>
                    </div>
                  {:else if s.id === 'playlist'}
                    <div class="space-y-2 text-sm">
                      <label class="flex items-center gap-2">
                        <input type="checkbox" bind:checked={options.playlist.enabled} />
                        <span>Treat URL as playlist</span>
                      </label>
                      {#if options.playlist.enabled}
                        <label class="block">
                          <span class="block text-[11px] uppercase tracking-wide text-muted-foreground mb-1">Items (yt-dlp -I syntax)</span>
                          <input
                            type="text" placeholder="1-5,7,10-12"
                            value={options.playlist.items ?? ''}
                            oninput={(e) => options.playlist.items = (e.target as HTMLInputElement).value || null}
                            class="w-full rounded-md border border-border/60 bg-secondary/50 px-2.5 py-1.5 text-sm font-mono"
                          />
                        </label>
                      {/if}
                    </div>
                  {/if}
                </div>
              </div>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>
