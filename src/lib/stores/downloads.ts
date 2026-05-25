import { writable, derived, get } from 'svelte/store';
import type {
  DownloadFinished,
  DownloadItem,
  DownloadRecord,
  DownloadStarted,
  ProgressUpdate
} from '$lib/types';
import { ipc } from '$lib/ipc';

function recordToItem(r: DownloadRecord): DownloadItem {
  return {
    id: r.id,
    url: r.url,
    title: r.title,
    status: (r.status as DownloadItem['status']) ?? 'queued',
    percent: r.percent,
    downloaded: r.downloaded,
    total: r.total,
    speedBps: null,
    etaSecs: null,
    error: r.error,
    attempts: r.attempts,
    queuePosition: r.queuePosition,
    startedAt: r.startedAt ? Date.parse(r.startedAt) : Date.parse(r.createdAt),
    outputPath: r.outputPath ?? null
  };
}

function createStore() {
  const items = writable<Map<string, DownloadItem>>(new Map());

  // Coalesce rapid-fire progress updates to a single store write per RAF.
  const pendingProgress = new Map<string, Partial<DownloadItem>>();
  let rafScheduled = false;
  function scheduleFlush() {
    if (rafScheduled) return;
    rafScheduled = true;
    requestAnimationFrame(() => {
      rafScheduled = false;
      if (pendingProgress.size === 0) return;
      items.update((m) => {
        for (const [id, patch] of pendingProgress) {
          const cur = m.get(id);
          if (cur) m.set(id, { ...cur, ...patch });
        }
        pendingProgress.clear();
        return new Map(m);
      });
    });
  }

  function applyPatch(id: string, patch: Partial<DownloadItem>) {
    items.update((m) => {
      const cur = m.get(id);
      if (cur) m.set(id, { ...cur, ...patch });
      return new Map(m);
    });
  }

  return {
    subscribe: items.subscribe,

    list: derived(items, ($m) =>
      [...$m.values()].sort((a, b) => {
        const order = ['downloading', 'queued', 'paused', 'error', 'finished', 'canceled'];
        const oa = order.indexOf(a.status);
        const ob = order.indexOf(b.status);
        if (oa !== ob) return oa - ob;
        if (a.status === 'queued' || a.status === 'paused') return a.queuePosition - b.queuePosition;
        return b.startedAt - a.startedAt;
      })
    ),

    active: derived(items, ($m) =>
      [...$m.values()].filter((i) => i.status === 'downloading' || i.status === 'paused')
    ),
    queued: derived(items, ($m) =>
      [...$m.values()].filter((i) => i.status === 'queued').sort((a, b) => a.queuePosition - b.queuePosition)
    ),
    history: derived(items, ($m) =>
      [...$m.values()]
        .filter((i) => i.status === 'finished' || i.status === 'error' || i.status === 'canceled')
        .sort((a, b) => b.startedAt - a.startedAt)
    ),

    async hydrate() {
      const recs = await ipc.listDownloads(500);
      const m = new Map<string, DownloadItem>();
      for (const r of recs) m.set(r.id, recordToItem(r));
      items.set(m);
    },

    /**
     * Insert/refresh a record from the backend's `queue://queued` event,
     * fired immediately after enqueue. Without this the UI would only
     * see the download once the dispatcher picks it up (which feels like
     * "nothing happened" when clicking Download).
     */
    handleQueued(r: DownloadRecord) {
      items.update((m) => {
        const cur = m.get(r.id);
        const seed = recordToItem(r);
        m.set(r.id, cur ? { ...cur, ...seed } : seed);
        return new Map(m);
      });
    },

    handleStarted(p: DownloadStarted) {
      const cur = get(items).get(p.id);
      const seed: DownloadItem = cur ?? {
        id: p.id,
        url: p.url,
        title: null,
        status: 'downloading',
        percent: 0,
        downloaded: 0,
        total: null,
        speedBps: null,
        etaSecs: null,
        error: null,
        attempts: 0,
        queuePosition: 0,
        startedAt: Date.now(),
        outputPath: null
      };
      applyPatch(p.id, { ...seed, status: 'downloading', startedAt: Date.now() });
    },

    handleProgress(p: ProgressUpdate) {
      pendingProgress.set(p.id, {
        ...(pendingProgress.get(p.id) ?? {}),
        title: p.title ?? undefined,
        percent: p.percent ?? 0,
        downloaded: p.downloaded,
        total: p.total,
        speedBps: p.speedBps,
        etaSecs: p.etaSecs,
        status: p.status === 'finished' ? 'finished' : 'downloading'
      });
      scheduleFlush();
    },

    handleFinished(p: DownloadFinished) {
      applyPatch(p.id, {
        status: p.canceled ? 'canceled' : p.success ? 'finished' : 'error',
        error: p.error,
        speedBps: null,
        etaSecs: null,
        outputPath: p.outputPath ?? undefined,
        percent: p.success ? 100 : undefined as unknown as number
      });
    },

    remove(id: string) {
      items.update((m) => { m.delete(id); return new Map(m); });
    }
  };
}

export const downloads = createStore();
