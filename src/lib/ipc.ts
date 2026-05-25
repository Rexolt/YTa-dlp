import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  ChapterInfo,
  DownloadFinished,
  DownloadLog,
  DownloadOptions,
  DownloadRecord,
  DownloadStarted,
  ProgressUpdate,
  QueueChanged,
  SponsorSegment,
  SubtitleTrack
} from './types';

export const Events = {
  Started: 'download://started',
  Progress: 'download://progress',
  Log: 'download://log',
  Finished: 'download://finished',
  QueueChanged: 'queue://changed',
  Queued: 'queue://queued'
} as const;

export interface FormatInfo {
  formatId: string;
  ext: string | null;
  height: number | null;
  width: number | null;
  fps: number | null;
  vcodec: string | null;
  acodec: string | null;
  tbrKbps: number | null;
  abrKbps: number | null;
  filesize: number | null;
  formatNote: string | null;
  kind: 'video' | 'audio' | 'video+audio';
}

export interface ProbeResult {
  title: string | null;
  uploader: string | null;
  uploaderUrl: string | null;
  channel: string | null;
  description: string | null;
  durationSecs: number | null;
  thumbnail: string | null;
  thumbnails: string[];
  viewCount: number | null;
  likeCount: number | null;
  uploadDate: string | null;
  ageLimit: number | null;
  liveStatus: string | null;
  webpageUrl: string | null;
  isPlaylist: boolean;
  playlistCount: number | null;
  chapters: ChapterInfo[];
  subtitles: SubtitleTrack[];
  sponsorSegments: SponsorSegment[];
  formats: FormatInfo[];
  availableHeights: number[];
  availableFps: number[];
  availableAudioBitrates: number[];
  hasVideo: boolean;
  hasAudio: boolean;
}

export interface ToolStatus {
  name: string;
  available: boolean;
  source: 'userInstalled' | 'packaged' | 'devSidecar' | 'systemPath' | 'missing';
  path: string | null;
  version: string | null;
}

export interface EnvironmentReport {
  ytDlp: ToolStatus;
  ffmpeg: ToolStatus;
  ffprobe: ToolStatus;
  ffmpegHasWebp: boolean;
}

export interface UpdateResult {
  success: boolean;
  output: string;
}

export const ipc = {
  probeUrl: (url: string) => invoke<ProbeResult>('probe_url', { url }),
  startDownload: (options: DownloadOptions) => invoke<string>('start_download', { options }),
  cancelDownload: (id: string) => invoke<boolean>('cancel_download', { id }),
  pauseDownload: (id: string) => invoke<boolean>('pause_download', { id }),
  resumeDownload: (id: string) => invoke<boolean>('resume_download', { id }),
  retryDownload: (id: string) => invoke<boolean>('retry_download', { id }),
  deleteDownload: (id: string) => invoke<void>('delete_download', { id }),
  listDownloads: (limit?: number) => invoke<DownloadRecord[]>('list_downloads', { limit }),
  clearHistory: () => invoke<number>('clear_history'),
  setMaxConcurrent: (n: number) => invoke<void>('set_max_concurrent', { n }),
  reorderDownload: (id: string, position: number) =>
    invoke<void>('reorder_download', { id, position }),
  version: () => invoke<string>('ytdlp_version'),
  checkEnvironment: () => invoke<EnvironmentReport>('check_environment'),
  updateYtDlp: () => invoke<UpdateResult>('update_ytdlp'),
  pickOutputDirectory: () => invoke<string | null>('pick_output_directory'),
  defaultDownloadDir: () => invoke<string>('default_download_dir'),
  revealInFileManager: (path: string) => invoke<void>('reveal_in_file_manager', { path }),

  onStarted: (cb: (p: DownloadStarted) => void): Promise<UnlistenFn> =>
    listen<DownloadStarted>(Events.Started, (e) => cb(e.payload)),
  onProgress: (cb: (p: ProgressUpdate) => void): Promise<UnlistenFn> =>
    listen<ProgressUpdate>(Events.Progress, (e) => cb(e.payload)),
  onLog: (cb: (p: DownloadLog) => void): Promise<UnlistenFn> =>
    listen<DownloadLog>(Events.Log, (e) => cb(e.payload)),
  onFinished: (cb: (p: DownloadFinished) => void): Promise<UnlistenFn> =>
    listen<DownloadFinished>(Events.Finished, (e) => cb(e.payload)),
  onQueueChanged: (cb: (p: QueueChanged) => void): Promise<UnlistenFn> =>
    listen<QueueChanged>(Events.QueueChanged, (e) => cb(e.payload)),
  onQueued: (cb: (p: DownloadRecord) => void): Promise<UnlistenFn> =>
    listen<DownloadRecord>(Events.Queued, (e) => cb(e.payload))
};
