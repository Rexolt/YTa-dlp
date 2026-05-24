<script lang="ts">
  import { fly, scale } from 'svelte/transition';
  import { quintOut } from 'svelte/easing';
  import { Link2, Loader2, Sparkles, ArrowRight, X } from 'lucide-svelte';
  import { cn } from '$lib/utils';

  interface Props {
    onSubmit: (url: string) => void | Promise<void>;
    onInput?: (url: string) => void;
    loading?: boolean;
    disabled?: boolean;
  }

  let { onSubmit, onInput, loading = false, disabled = false }: Props = $props();

  let url = $state('');
  let focused = $state(false);
  let inputEl: HTMLInputElement | undefined = $state();

  // Notify parent on every keystroke (probe debounce lives in the store).
  $effect(() => { onInput?.(url); });

  const isValid = $derived(
    /^https?:\/\/(www\.)?(youtube\.com|youtu\.be|m\.youtube\.com|music\.youtube\.com)\/.+/i.test(
      url.trim()
    )
  );
  const hasValue = $derived(url.trim().length > 0);

  async function handleSubmit(e?: Event) {
    e?.preventDefault();
    if (!isValid || loading || disabled) return;
    await onSubmit(url.trim());
  }

  async function pasteFromClipboard() {
    try {
      const text = await navigator.clipboard.readText();
      if (text) {
        url = text.trim();
        inputEl?.focus();
      }
    } catch {
      // ignore
    }
  }

  function clear() {
    url = '';
    inputEl?.focus();
  }
</script>

<form
  onsubmit={handleSubmit}
  class="relative w-full max-w-2xl mx-auto"
  in:fly={{ y: 16, duration: 600, easing: quintOut }}
>
  <!-- Halo / aura behind input -->
  <div
    class={cn(
      'pointer-events-none absolute -inset-6 rounded-[2rem] blur-2xl transition-opacity duration-500',
      focused || hasValue ? 'opacity-100' : 'opacity-0'
    )}
    aria-hidden="true"
    style="background: radial-gradient(60% 60% at 50% 50%, hsl(var(--primary) / 0.35), transparent 70%);"
  ></div>

  <div
    class={cn(
      'focus-ring-conic relative flex items-center gap-2 rounded-2xl glass px-4 py-3 transition-all duration-300',
      focused && 'ring-2 ring-primary/40 shadow-[0_20px_70px_-20px_hsl(var(--primary)/0.45)]',
      !focused && 'shadow-[0_10px_40px_-15px_rgba(0,0,0,0.5)]'
    )}
  >
    <div
      class={cn(
        'flex h-9 w-9 shrink-0 items-center justify-center rounded-xl transition-colors',
        focused ? 'bg-primary/15 text-primary' : 'bg-muted text-muted-foreground'
      )}
    >
      <Link2 class="h-4 w-4" />
    </div>

    <input
      bind:this={inputEl}
      bind:value={url}
      onfocus={() => (focused = true)}
      onblur={() => (focused = false)}
      type="url"
      inputmode="url"
      autocomplete="off"
      autocorrect="off"
      spellcheck="false"
      placeholder="Paste a YouTube link…  (https://youtube.com/watch?v=…)"
      class="peer flex-1 bg-transparent text-base md:text-[15px] tracking-tight outline-none placeholder:text-muted-foreground/70 disabled:opacity-50"
      {disabled}
    />

    {#if hasValue}
      <button
        type="button"
        onclick={clear}
        in:scale={{ duration: 150, start: 0.85 }}
        class="grid h-7 w-7 place-items-center rounded-lg text-muted-foreground hover:bg-muted hover:text-foreground transition"
        aria-label="Clear"
      >
        <X class="h-3.5 w-3.5" />
      </button>
    {:else}
      <button
        type="button"
        onclick={pasteFromClipboard}
        class="hidden sm:flex items-center gap-1.5 rounded-lg border border-border/60 bg-secondary/60 px-2.5 py-1 text-xs text-muted-foreground hover:text-foreground hover:bg-secondary transition"
      >
        <Sparkles class="h-3 w-3" /> Paste
      </button>
    {/if}

    <button
      type="submit"
      disabled={!isValid || loading || disabled}
      class={cn(
        'group flex h-10 items-center gap-2 rounded-xl px-4 text-sm font-medium transition-all',
        isValid && !loading
          ? 'bg-primary text-primary-foreground hover:brightness-110 shadow-[0_8px_24px_-8px_hsl(var(--primary)/0.7)] animate-pulse-ring'
          : 'bg-muted text-muted-foreground cursor-not-allowed'
      )}
    >
      {#if loading}
        <Loader2 class="h-4 w-4 animate-spin" />
        <span>Analyzing…</span>
      {:else}
        <span>Download</span>
        <ArrowRight class="h-4 w-4 transition-transform group-hover:translate-x-0.5" />
      {/if}
    </button>
  </div>

  <!-- Validation / hint row -->
  <div class="mt-2 flex h-5 items-center justify-between px-2 text-[11px] text-muted-foreground">
    {#if hasValue && !isValid}
      <span in:fly={{ y: -4, duration: 180 }} class="text-destructive">
        That doesn't look like a YouTube URL.
      </span>
    {:else if isValid}
      <span in:fly={{ y: -4, duration: 180 }} class="text-emerald-400/90">
        Valid URL — press Enter to continue.
      </span>
    {:else}
      <span>Tip: ⌘V / Ctrl+V to paste from clipboard.</span>
    {/if}
    <kbd class="rounded-md border border-border/60 bg-secondary/50 px-1.5 py-0.5 font-mono">↵</kbd>
  </div>
</form>
