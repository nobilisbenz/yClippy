use tauri::Manager;

mod db;
mod github_api;
mod oplog;
pub mod play;
mod sync;
pub mod sync_engine;
pub mod wire;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let initial_args: Vec<String> = std::env::args().collect();

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    let builder = tauri::Builder::default().plugin(tauri_plugin_single_instance::init(
        |app, argv, _cwd| {
            if let Some(play::Command::Play(req)) = play::parse_play_args(&argv) {
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = play::process_play_request(&app_handle, req).await {
                        eprintln!("Failed to process CLI play request: {e}");
                    }
                });
            }
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.unminimize();
                let _ = win.set_focus();
            }
        },
    ));

    #[cfg(any(target_os = "android", target_os = "ios"))]
    let builder = tauri::Builder::default();

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(move |app| {
            let db_state = db::init_db(app.handle())?;
            app.manage(db_state);
            app.manage(play::PendingPlay::default());

            if let Some(play::Command::Play(req)) = play::parse_play_args(&initial_args) {
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = play::process_play_request(&app_handle, req).await {
                        eprintln!("Failed to process CLI play request: {e}");
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            db::get_videos,
            db::save_video,
            db::get_clips,
            db::save_clip,
            db::delete_clip,
            db::get_folders,
            db::save_folder,
            db::delete_folder,
            db::delete_video,
            db::rename_folder,
            db::rename_video,
            db::update_video_metadata,
            db::rename_clip,
            db::update_video_folder,
            db::update_folder_parent,
            db::export_db,
            db::import_db,
            db::update_clip,
            db::get_db_path,
            db::set_db_path,
            db::update_sort_order,
            db::update_clip_sort_order,
            db::restore_video,
            db::restore_folder,
            db::restore_clip,
            db::import_from_yt_renamer,
            db::fetch_video_oembed,
            db::get_github_config,
            db::set_github_config,
            db::clear_github_token,
            sync::start_github_sync,
            play::list_videos_for_picker,
            play::take_pending_play
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
