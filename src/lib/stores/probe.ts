import { writable } from 'svelte/store';
import { ipc, type ProbeResult } from '$lib/ipc';

export type ProbeStatus = 'idle' | 'loading' | 'ready' | 'error';

export interface ProbeState {
  status: ProbeStatus;
  url: string | null;
  data: ProbeResult | null;
  error: string | null;
}

const initial: ProbeState = { status: 'idle', url: null, data: null, error: null };

function createProbeStore() {
  const store = writable<ProbeState>(initial);
  let activeUrl: string | null = null;
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  const cache = new Map<string, ProbeResult>();

  async function runProbe(url: string) {
    activeUrl = url;
    // Use cache to avoid re-probing the same URL.
    const hit = cache.get(url);
    if (hit) {
      store.set({ status: 'ready', url, data: hit, error: null });
      return;
    }
    store.update((s) => ({ ...s, status: 'loading', url, error: null }));
    try {
      const data = await ipc.probeUrl(url);
      if (activeUrl !== url) return; // stale
      cache.set(url, data);
      store.set({ status: 'ready', url, data, error: null });
    } catch (e) {
      if (activeUrl !== url) return;
      store.set({ status: 'error', url, data: null, error: String(e) });
    }
  }

  return {
    subscribe: store.subscribe,

    /** Debounced probe — call freely while the user is typing. */
    request(url: string | null, delayMs = 350) {
      if (debounceTimer) clearTimeout(debounceTimer);
      if (!url) {
        activeUrl = null;
        store.set(initial);
        return;
      }
      debounceTimer = setTimeout(() => runProbe(url), delayMs);
    },

    clear() {
      activeUrl = null;
      store.set(initial);
    }
  };
}

export const probe = createProbeStore();
