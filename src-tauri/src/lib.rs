mod bin;
mod commands;
mod db;
mod downloader;
mod error;
mod queue;
mod sponsorblock;
mod ytdlp;

use tauri::Manager;
use tracing_subscriber::EnvFilter;

use crate::db::{default_db_path, Db};
use crate::queue::QueueManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Workarounds for WebKitGTK on modern Wayland compositors (Hyprland,
    // GNOME 46+, KDE 6) and NVIDIA proprietary drivers, where the DMABUF
    // renderer and the compositing path frequently abort with
    // `Error 71 (Protocol error) dispatching to Wayland display`.
    //
    // These env vars must be set before GTK/WebKit initialises. We only
    // set them if the user hasn't already overridden them.
    #[cfg(target_os = "linux")]
    apply_linux_webkit_workarounds();

    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .try_init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let handle = app.handle().clone();
            // Bootstrap DB + Queue on a tokio task before exposing them via state.
            tauri::async_runtime::block_on(async move {
                let db = Db::open(default_db_path())
                    .await
                    .expect("failed to open database");
                // Reconcile orphan 'downloading' rows from a previous crash.
                for rec in db.list(1000).await.unwrap_or_default() {
                    if rec.status == "downloading" {
                        let _ = db.mark_status(&rec.id, "paused", None).await;
                    }
                }
                let queue = QueueManager::new(db, handle.clone());
                handle.manage(queue);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::probe_url,
            commands::start_download,
            commands::cancel_download,
            commands::pause_download,
            commands::resume_download,
            commands::retry_download,
            commands::list_downloads,
            commands::delete_download,
            commands::clear_history,
            commands::set_max_concurrent,
            commands::reorder_download,
            commands::ytdlp_version,
            commands::check_environment,
            commands::update_ytdlp,
            commands::pick_output_directory,
            commands::default_download_dir,
            commands::reveal_in_file_manager,
        ])
        .run(tauri::generate_context!())
        .expect("error while running YTa-dlp");
}

/// Apply environment workarounds for known WebKitGTK + Wayland breakages.
///
/// Symptom this addresses:
///   `GDK-Message: ... Error 71 (Protocol error) dispatching to Wayland display.`
///
/// Root causes (any combination):
///   * WebKitGTK 2.42+ enabled a DMABUF renderer that crashes with NVIDIA
///     and some Mesa builds.
///   * GNOME 46+/KDE 6/Hyprland's wlroots-based compositors are stricter
///     about protocol violations from WebKit's compositing path.
///   * The "compositing mode" in WebKit aggressively uses GL on a Wayland
///     surface that may not be ready yet.
///
/// We only set vars the user hasn't already configured, so power users
/// can opt out by exporting them to a non-empty value.
#[cfg(target_os = "linux")]
fn apply_linux_webkit_workarounds() {
    fn set_default(key: &str, value: &str) {
        if std::env::var_os(key).is_none() {
            // SAFETY: single-threaded at program start, before any GTK init.
            std::env::set_var(key, value);
        }
    }

    set_default("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    set_default("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
    // Some NVIDIA + Wayland combos still need GTK to fall back to X11/XWayland
    // for stability. We only force this when XWayland is actually available
    // (i.e. DISPLAY is set), otherwise leave the user's choice intact.
    if std::env::var_os("DISPLAY").is_some()
        && std::env::var_os("GDK_BACKEND").is_none()
        && std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("wayland")
    {
        std::env::set_var("GDK_BACKEND", "x11");
        tracing::info!(
            "Wayland session detected with XWayland available; forcing GDK_BACKEND=x11 \
             to avoid WebKitGTK protocol errors. Override by exporting GDK_BACKEND."
        );
    }
}
