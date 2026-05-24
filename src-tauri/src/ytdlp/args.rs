use super::*;

/// A unique progress template we can reliably parse from stdout.
/// Fields are tab-separated to avoid filename collision with spaces/colons.
pub const PROGRESS_PREFIX: &str = "__YTADLP_PROGRESS__";
pub const PROGRESS_TEMPLATE: &str = "__YTADLP_PROGRESS__\t%(progress.status)s\t%(progress.downloaded_bytes)s\t%(progress.total_bytes)s\t%(progress.total_bytes_estimate)s\t%(progress.speed)s\t%(progress.eta)s\t%(info.title)s";

/// Marker yt-dlp prints (via `--print after_move:...`) so we can capture the
/// final filepath of the merged/post-processed output.
pub const FILEPATH_PREFIX: &str = "__YTADLP_FILEPATH__";

/// Expand a leading `~` to the user's home directory. Leaves any other path
/// (absolute or relative) untouched.
fn expand_home(p: &str) -> String {
    if let Some(rest) = p.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
            let mut out = std::path::PathBuf::from(home);
            out.push(rest);
            return out.to_string_lossy().into_owned();
        }
    }
    if p == "~" {
        if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
            return home.to_string_lossy().into_owned();
        }
    }
    p.to_string()
}

/// Convert structured options into a yt-dlp argv vector.
pub fn build_args(opts: &DownloadOptions) -> Vec<String> {
    let mut a: Vec<String> = Vec::with_capacity(48);

    // Stable, line-buffered progress output that we can parse.
    a.push("--newline".into());
    a.push("--no-colors".into());
    a.push("--progress".into());
    a.push("--progress-template".into());
    a.push(format!("download:{}", PROGRESS_TEMPLATE));
    a.push("--ignore-errors".into());

    // Output template
    a.push("-P".into());
    a.push(expand_home(&opts.output_dir));
    a.push("-o".into());
    a.push("%(title)s [%(id)s].%(ext)s".into());

    // Have yt-dlp print the *final* output filepath (after merge/postprocess)
    // on its own line so we can capture it for "Open folder" UX.
    a.push("--print".into());
    a.push("after_move:__YTADLP_FILEPATH__\t%(filepath)s".into());
    a.push("--no-simulate".into());

    // Format / quality
    match opts.kind {
        MediaKind::Video => apply_video(&mut a, opts),
        MediaKind::Audio => apply_audio(&mut a, opts),
    }

    // Subtitles — three modes, each maps to a different yt-dlp/ffmpeg pipeline.
    if let Some(s) = &opts.subtitles {
        if !s.languages.is_empty() {
            a.push("--sub-langs".into());
            a.push(s.languages.join(","));
            // We need yt-dlp to fetch them; the difference is in what it does after.
            a.push("--write-subs".into());
            if s.auto_generated {
                a.push("--write-auto-subs".into());
            }

            match s.mode {
                SubtitleMode::Sidecar => {
                    // Keep as separate file in the requested format.
                    a.push("--sub-format".into());
                    a.push(s.format.clone());
                    a.push("--convert-subs".into());
                    a.push(s.format.clone());
                }
                SubtitleMode::Embed => {
                    // Soft track inside the container (mp4/mkv supported by yt-dlp's
                    // SubtitleEmbedder postprocessor; webm cannot embed text subs).
                    a.push("--embed-subs".into());
                    a.push("--convert-subs".into());
                    a.push("srt".into());
                }
                SubtitleMode::BurnIn => {
                    // Hardcode into the video stream. Two steps:
                    //  1. Force srt sidecar so ffmpeg's `subtitles=` filter can read it.
                    //  2. Pass an ffmpeg filter that overlays the first sub track.
                    a.push("--sub-format".into());
                    a.push("srt".into());
                    a.push("--convert-subs".into());
                    a.push("srt".into());
                    // The actual burn-in is done by an explicit ffmpeg pass on the
                    // resulting video; we hint at it via `--postprocessor-args`.
                    // (The ffmpeg subtitles filter reads from the sidecar file.)
                    a.push("--postprocessor-args".into());
                    a.push(
                        "VideoConvertor+ffmpeg:-vf subtitles=%(filepath)s.%(ext)s"
                            .to_string(),
                    );
                }
            }
        }
    }

    // Metadata
    if opts.metadata.embed_thumbnail {
        a.push("--embed-thumbnail".into());
    }
    if opts.metadata.embed_metadata {
        a.push("--embed-metadata".into());
    }
    if opts.metadata.keep_chapters {
        a.push("--embed-chapters".into());
    }

    // SponsorBlock
    match opts.sponsorblock.mode {
        SponsorBlockMode::Off => {}
        SponsorBlockMode::Mark => {
            a.push("--sponsorblock-mark".into());
            a.push(opts.sponsorblock.categories.join(","));
        }
        SponsorBlockMode::Remove => {
            a.push("--sponsorblock-remove".into());
            a.push(opts.sponsorblock.categories.join(","));
        }
    }

    // Network
    if let Some(p) = &opts.network.proxy {
        a.push("--proxy".into());
        a.push(p.clone());
    }
    if let Some(r) = &opts.network.rate_limit {
        a.push("-r".into());
        a.push(r.clone());
    }
    if let Some(b) = &opts.network.cookies_from_browser {
        a.push("--cookies-from-browser".into());
        a.push(b.clone());
    }
    if opts.network.concurrent_fragments > 1 {
        a.push("-N".into());
        a.push(opts.network.concurrent_fragments.to_string());
    }

    // Playlist
    if opts.playlist.enabled {
        a.push("--yes-playlist".into());
        if let Some(items) = &opts.playlist.items {
            a.push("-I".into());
            a.push(items.clone());
        }
    } else {
        a.push("--no-playlist".into());
    }

    // Finally the URL.
    a.push(opts.url.clone());
    a
}

fn apply_video(a: &mut Vec<String>, opts: &DownloadOptions) {
    let q = opts.video.as_ref().cloned().unwrap_or(VideoQuality { max_height: 1080, fps: 60 });
    let container = opts
        .video_container
        .as_ref()
        .cloned()
        .unwrap_or(VideoContainer::Mp4);

    // Best video up to height/fps + best audio, merged into the requested container.
    let fmt = format!(
        "bv*[height<={h}][fps<={fps}]+ba/b[height<={h}][fps<={fps}]/bv*+ba/b",
        h = q.max_height,
        fps = q.fps
    );
    a.push("-f".into());
    a.push(fmt);

    a.push("--merge-output-format".into());
    a.push(match container {
        VideoContainer::Mp4 => "mp4".into(),
        VideoContainer::Mkv => "mkv".into(),
        VideoContainer::Webm => "webm".into(),
    });
}

fn apply_audio(a: &mut Vec<String>, opts: &DownloadOptions) {
    let q = opts
        .audio
        .as_ref()
        .cloned()
        .unwrap_or(AudioQuality { bitrate_kbps: 192, normalize: false });
    let container = opts
        .audio_container
        .as_ref()
        .cloned()
        .unwrap_or(AudioContainer::Mp3);

    a.push("-f".into());
    a.push("ba/b".into());

    a.push("-x".into());
    a.push("--audio-format".into());
    a.push(match container {
        AudioContainer::Mp3 => "mp3".into(),
        AudioContainer::Flac => "flac".into(),
        AudioContainer::Wav => "wav".into(),
        AudioContainer::M4a => "m4a".into(),
    });
    a.push("--audio-quality".into());
    a.push(format!("{}K", q.bitrate_kbps));

    if q.normalize {
        // ffmpeg loudnorm filter via postprocessor-args
        a.push("--postprocessor-args".into());
        a.push("ffmpeg:-af loudnorm=I=-16:TP=-1.5:LRA=11".into());
    }
}
