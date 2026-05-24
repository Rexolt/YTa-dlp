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
