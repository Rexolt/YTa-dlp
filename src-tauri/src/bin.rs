use std::path::{Path, PathBuf};

use once_cell::sync::Lazy;

/// Target triple baked in by build.rs (e.g. "x86_64-unknown-linux-gnu").
pub const TARGET_TRIPLE: &str = env!("TARGET_TRIPLE");

#[derive(Debug, Clone, Copy)]
pub enum BinarySource {
    /// Found next to the app exe (packaged release).
    Packaged,
    /// Found in `src-tauri/binaries/<name>-<triple>` (dev mode).
    DevSidecar,
    /// Found on PATH.
    SystemPath,
    /// Not found anywhere — fall back to bare name.
    Missing,
}

#[derive(Debug, Clone)]
pub struct ResolvedBinary {
    pub name: &'static str,
    pub path: PathBuf,
    pub source: BinarySource,
}

impl ResolvedBinary {
    pub fn is_present(&self) -> bool {
        !matches!(self.source, BinarySource::Missing)
    }
}

static EXE_DIR: Lazy<Option<PathBuf>> = Lazy::new(|| {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
});

/// Resolve a sidecar binary by canonical name (e.g. "yt-dlp", "ffmpeg").
/// Order: packaged (next to exe) → dev sidecar (triple suffix) → system PATH.
pub fn resolve(name: &'static str) -> ResolvedBinary {
    let exe_name = exe_filename(name);

    // 1. Packaged: next to the current exe.
    if let Some(dir) = EXE_DIR.as_ref() {
        let p = dir.join(&exe_name);
        if p.is_file() {
            return ResolvedBinary { name, path: p, source: BinarySource::Packaged };
        }
    }

    // 2. Dev sidecar (the file Tauri expects in src-tauri/binaries/):
    //    binaries/<name>-<triple>[.exe]
    let dev_basename = format!("{}-{}{}", name, TARGET_TRIPLE, exe_suffix());
    // Walk up from current exe to find a `src-tauri/binaries/` dir (typical `target/debug/...`).
    if let Some(dir) = EXE_DIR.as_ref() {
        for ancestor in dir.ancestors() {
            let candidate = ancestor.join("src-tauri").join("binaries").join(&dev_basename);
            if candidate.is_file() {
                return ResolvedBinary { name, path: candidate, source: BinarySource::DevSidecar };
            }
        }
    }

    // 3. PATH fallback.
    if let Some(found) = which_on_path(&exe_name) {
        return ResolvedBinary { name, path: found, source: BinarySource::SystemPath };
    }

    // 4. Missing — return bare name so spawning gives a clean NotFound error.
    ResolvedBinary {
        name,
        path: PathBuf::from(&exe_name),
        source: BinarySource::Missing,
    }
}

#[inline]
fn exe_suffix() -> &'static str {
    if cfg!(windows) { ".exe" } else { "" }
}

fn exe_filename(name: &str) -> String {
    format!("{}{}", name, exe_suffix())
}

fn which_on_path(filename: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(filename);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Convenience: directory containing the resolved ffmpeg, suitable for
/// `yt-dlp --ffmpeg-location <dir>`.
pub fn ffmpeg_location() -> Option<PathBuf> {
    let f = resolve("ffmpeg");
    if !f.is_present() {
        return None;
    }
    f.path.parent().map(|p| p.to_path_buf())
}

#[allow(dead_code)]
pub fn path_for(name: &'static str) -> PathBuf {
    resolve(name).path
}

#[allow(dead_code)]
pub fn is_under(_p: &Path) -> bool {
    true
}
