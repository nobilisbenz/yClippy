use crate::db::{self, DbState};
use crate::sync_engine::SyncEngine;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn start_github_sync(
    app: AppHandle,
    _state: State<'_, DbState>,
) -> Result<String, String> {
    let config = db::load_config_pub(&app);
    let token = config
        .github_token
        .ok_or_else(|| "GitHub token is not configured".to_string())?;
    let repo = config
        .github_repo
        .ok_or_else(|| "GitHub repo is not configured".to_string())?;
    let engine = SyncEngine::new(token, repo).ok_or("Invalid Repo URL".to_string())?;
    engine.sync(app).await
}
