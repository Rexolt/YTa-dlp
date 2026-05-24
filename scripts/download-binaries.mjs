#!/usr/bin/env node
// Fetch yt-dlp + a static ffmpeg/ffprobe build for the current host platform,
// and copy them into `src-tauri/binaries/` with the target-triple suffix Tauri
// expects for sidecars. Idempotent: re-runs skip already-present binaries.
//
// Usage:
//   node scripts/download-binaries.mjs            # current host
//   node scripts/download-binaries.mjs --force    # re-download even if present
//
// Layout produced:
//   src-tauri/binaries/yt-dlp-<triple>[.exe]
//   src-tauri/binaries/ffmpeg-<triple>[.exe]
//   src-tauri/binaries/ffprobe-<triple>[.exe]

import { execSync, spawnSync } from 'node:child_process';
import { createWriteStream, existsSync, mkdirSync, chmodSync, copyFileSync, renameSync, rmSync } from 'node:fs';
import { mkdtemp, readdir, stat } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { pipeline } from 'node:stream/promises';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '..');
const OUT_DIR = join(ROOT, 'src-tauri', 'binaries');
const FORCE = process.argv.includes('--force');

// ---------- platform / triple detection ----------
const arch = process.arch;     // 'x64' | 'arm64' | ...
const platform = process.platform; // 'linux' | 'darwin' | 'win32'

function targetTriple() {
  if (platform === 'linux') {
    return arch === 'arm64' ? 'aarch64-unknown-linux-gnu' : 'x86_64-unknown-linux-gnu';
  }
  if (platform === 'darwin') {
    return arch === 'arm64' ? 'aarch64-apple-darwin' : 'x86_64-apple-darwin';
  }
  if (platform === 'win32') {
    return arch === 'arm64' ? 'aarch64-pc-windows-msvc' : 'x86_64-pc-windows-msvc';
  }
  throw new Error(`Unsupported platform: ${platform}/${arch}`);
}

const TRIPLE = targetTriple();
const EXE = platform === 'win32' ? '.exe' : '';

// ---------- yt-dlp URLs (official release assets) ----------
function ytDlpAsset() {
  if (platform === 'win32')  return arch === 'arm64'
    ? 'yt-dlp_x86.exe' // no native ARM build; use x86 under emulation
    : 'yt-dlp.exe';
  if (platform === 'darwin') return 'yt-dlp_macos';
  if (platform === 'linux')  return arch === 'arm64' ? 'yt-dlp_linux_aarch64' : 'yt-dlp_linux';
  throw new Error(`Unsupported yt-dlp platform: ${platform}`);
}

const YT_DLP_URL = `https://github.com/yt-dlp/yt-dlp/releases/latest/download/${ytDlpAsset()}`;

// ---------- ffmpeg URLs (BtbN static gpl builds) ----------
function ffmpegAsset() {
  // Repo: https://github.com/BtbN/FFmpeg-Builds — "gpl" non-shared zips/tarballs.
  if (platform === 'linux') {
    return {
      url: arch === 'arm64'
        ? 'https://github.com/BtbN/FFmpeg-Builds/releases/latest/download/ffmpeg-master-latest-linuxarm64-gpl.tar.xz'
        : 'https://github.com/BtbN/FFmpeg-Builds/releases/latest/download/ffmpeg-master-latest-linux64-gpl.tar.xz',
      kind: 'tar.xz'
    };
  }
  if (platform === 'win32') {
    return {
      url: 'https://github.com/BtbN/FFmpeg-Builds/releases/latest/download/ffmpeg-master-latest-win64-gpl.zip',
      kind: 'zip'
    };
  }
  if (platform === 'darwin') {
    // BtbN doesn't publish official macOS builds — recommend evermeet.cx zips.
    return {
      url: arch === 'arm64'
        ? 'https://www.osxexperts.net/ffmpeg711arm.zip'
        : 'https://www.osxexperts.net/ffmpeg711intel.zip',
      kind: 'zip'
    };
  }
  throw new Error(`Unsupported ffmpeg platform: ${platform}`);
}

// ---------- helpers ----------
async function download(url, destPath) {
  console.log(`  ↓ ${url}`);
  const res = await fetch(url, { redirect: 'follow' });
  if (!res.ok) throw new Error(`fetch ${url} failed: ${res.status} ${res.statusText}`);
  await pipeline(res.body, createWriteStream(destPath));
}

function ensureDir(p) {
  if (!existsSync(p)) mkdirSync(p, { recursive: true });
}

function makeExecutable(p) {
  if (platform !== 'win32') chmodSync(p, 0o755);
}

function which(cmd) {
  const r = spawnSync(platform === 'win32' ? 'where' : 'which', [cmd], { encoding: 'utf8' });
  return r.status === 0 ? r.stdout.split(/\r?\n/)[0].trim() : null;
}

async function findInDir(dir, name) {
  const stack = [dir];
  while (stack.length) {
    const d = stack.pop();
    const entries = await readdir(d, { withFileTypes: true });
    for (const e of entries) {
      const full = join(d, e.name);
      if (e.isDirectory()) stack.push(full);
      else if (e.name === name) return full;
    }
  }
  return null;
}

async function extract(archivePath, kind, intoDir) {
  ensureDir(intoDir);
  if (kind === 'zip') {
    if (platform === 'win32') {
      execSync(`powershell -Command "Expand-Archive -Force '${archivePath}' '${intoDir}'"`, { stdio: 'inherit' });
    } else {
      execSync(`unzip -o "${archivePath}" -d "${intoDir}"`, { stdio: 'inherit' });
    }
  } else if (kind === 'tar.xz') {
    execSync(`tar -xJf "${archivePath}" -C "${intoDir}"`, { stdio: 'inherit' });
  } else {
    throw new Error(`Unknown archive kind: ${kind}`);
  }
}

// ---------- main ----------
async function fetchYtDlp() {
  const target = join(OUT_DIR, `yt-dlp-${TRIPLE}${EXE}`);
  if (existsSync(target) && !FORCE) {
    console.log(`✓ yt-dlp already at ${target}`);
    return;
  }
  console.log(`→ Downloading yt-dlp (${TRIPLE}) ...`);
  await download(YT_DLP_URL, target);
  makeExecutable(target);
  console.log(`✓ yt-dlp → ${target}`);
}

async function fetchFfmpeg() {
  const ffmpegTarget  = join(OUT_DIR, `ffmpeg-${TRIPLE}${EXE}`);
  const ffprobeTarget = join(OUT_DIR, `ffprobe-${TRIPLE}${EXE}`);
  if (existsSync(ffmpegTarget) && existsSync(ffprobeTarget) && !FORCE) {
    console.log(`✓ ffmpeg/ffprobe already at ${ffmpegTarget}`);
    return;
  }

  const { url, kind } = ffmpegAsset();
  console.log(`→ Downloading ffmpeg bundle (${TRIPLE}) ...`);

  const tmp = await mkdtemp(join(tmpdir(), 'ytadlp-ffmpeg-'));
  try {
    const archivePath = join(tmp, kind === 'zip' ? 'ffmpeg.zip' : 'ffmpeg.tar.xz');
    await download(url, archivePath);
    await extract(archivePath, kind, tmp);

    const ffmpegBin  = await findInDir(tmp, `ffmpeg${EXE}`);
    const ffprobeBin = await findInDir(tmp, `ffprobe${EXE}`);
    if (!ffmpegBin)  throw new Error('ffmpeg binary not found in archive');
    if (!ffprobeBin) console.warn('! ffprobe binary not found in archive — continuing without it');

    copyFileSync(ffmpegBin, ffmpegTarget);
    makeExecutable(ffmpegTarget);
    if (ffprobeBin) {
      copyFileSync(ffprobeBin, ffprobeTarget);
      makeExecutable(ffprobeTarget);
    }
    console.log(`✓ ffmpeg → ${ffmpegTarget}`);
    if (ffprobeBin) console.log(`✓ ffprobe → ${ffprobeTarget}`);
  } finally {
    try { rmSync(tmp, { recursive: true, force: true }); } catch { /* ignore */ }
  }
}

async function main() {
  ensureDir(OUT_DIR);
  console.log(`Target triple: ${TRIPLE}`);
  console.log(`Output dir:    ${OUT_DIR}`);
  await fetchYtDlp();
  await fetchFfmpeg();
  console.log('Done.');
}

main().catch((err) => {
  console.error('✗ Binary setup failed:', err.message);
  process.exit(1);
});
