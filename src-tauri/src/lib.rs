use tauri::Manager;

mod db;
mod github_api;
mod sync;
pub mod sync_engine;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let db_state = db::init_db(app.handle())?;
            app.manage(db_state);
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
            db::import_from_yt_renamer,
            db::fetch_video_oembed,
            sync::start_github_sync
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
