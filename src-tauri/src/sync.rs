use crate::db::DbState;
use crate::sync_engine::SyncEngine;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn start_github_sync(
    app: AppHandle,
    _state: State<'_, DbState>,
    token: String,
    repo_url: String,
) -> Result<String, String> {
    let engine = SyncEngine::new(token, repo_url).ok_or("Invalid Repo URL".to_string())?;

    engine.sync(app).await
}
