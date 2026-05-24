# YTa-dlp

Premium desktop YouTube downloader built on **yt-dlp**, with a Rust + Tauri v2 backend and a SvelteKit + Tailwind + shadcn-svelte frontend.

## Status

Scaffolding stage. See the project tree in the design doc / chat history.

## Prerequisites

- Rust (stable) + `cargo`
- Node 20+ and `pnpm` (or npm)
- Either:
  - **Recommended:** let the postinstall script fetch bundled `yt-dlp` + `ffmpeg` sidecars (see below), or
  - install `yt-dlp` and `ffmpeg` system-wide and put them on `PATH`.

The in-app **Environment** pill (top-right) shows you which binaries are detected
and where they came from (`bundled` / `sidecar` / `system` / `missing`).

## Dev

```bash
pnpm install            # also runs `binaries:fetch` via postinstall
pnpm tauri dev
```

If the postinstall step was skipped or failed, re-run manually:

```bash
pnpm binaries:fetch       # fetch yt-dlp + ffmpeg for current platform
pnpm binaries:refresh     # force re-download
```

The script writes platform-suffixed files to `src-tauri/binaries/`, e.g.:

```
src-tauri/binaries/yt-dlp-x86_64-unknown-linux-gnu
src-tauri/binaries/ffmpeg-x86_64-unknown-linux-gnu
src-tauri/binaries/ffprobe-x86_64-unknown-linux-gnu
```

These are declared as `externalBin` in `src-tauri/tauri.conf.json` and get
bundled into the final installer / app image automatically.

## Build

```bash
pnpm tauri build
```

## Architecture

- **Frontend → Backend**: Tauri `invoke` commands (`start_download`, `cancel_download`, `probe_url`, ...).
- **Backend → Frontend**: Tauri events emitted per-download (`download://progress/<id>`, `download://log/<id>`, `download://done/<id>`).
- **Concurrency**: each download is a `tokio::task` that spawns `yt-dlp` via `tokio::process::Command` with `--newline --progress-template`, parses lines, and emits events. Cancellation via `kill_on_drop` + a `CancellationToken`.
