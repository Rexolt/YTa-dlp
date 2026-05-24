import { writable } from 'svelte/store';
import { browser } from '$app/environment';
import { ipc } from '$lib/ipc';

export interface Settings {
  maxConcurrent: number;
  theme: 'dark' | 'light' | 'system';
  outputDir: string;
}

const DEFAULTS: Settings = {
  maxConcurrent: 2,
  theme: 'dark',
  outputDir: '~/Downloads/YTa-dlp'
};

const KEY = 'ytadlp.settings.v1';

function load(): Settings {
  if (!browser) return DEFAULTS;
  try {
    const raw = localStorage.getItem(KEY);
    return raw ? { ...DEFAULTS, ...JSON.parse(raw) } : DEFAULTS;
  } catch {
    return DEFAULTS;
  }
}

function createSettings() {
  const store = writable<Settings>(load());

  if (browser) {
    store.subscribe((s) => {
      try { localStorage.setItem(KEY, JSON.stringify(s)); } catch { /* ignore */ }
    });
  }

  return {
    subscribe: store.subscribe,
    update: store.update,
    set: store.set,
    async syncToBackend(s: Settings) {
      try { await ipc.setMaxConcurrent(s.maxConcurrent); } catch { /* ignore */ }
    }
  };
}

export const settings = createSettings();
