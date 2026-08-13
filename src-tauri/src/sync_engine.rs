use crate::db::{self, DbState};
use crate::github_api;
use crate::oplog;
use tauri::{AppHandle, Manager};

pub struct SyncEngine {
    pub token: String,
    pub owner: String,
    pub repo: String,
}

impl SyncEngine {
    pub fn new(token: String, repo_url: String) -> Option<Self> {
        let (owner, repo) = parse_repo_url(&repo_url)?;
        Some(Self { token, owner, repo })
    }

    pub async fn sync(&self, app: AppHandle) -> Result<String, String> {
        self.push(&app).await?;
        self.pull(&app).await?;
        crate::oplog::compact_library(app.clone())?;
        Ok("Sync completed".to_string())
    }

    pub async fn push(&self, app: &AppHandle) -> Result<(), String> {
        let mut watermarks = oplog::read_watermarks(app);

        let (local_videos, local_folders, local_clips) = {
            let state: tauri::State<DbState> = app.state();
            let conn = state.conn.lock().unwrap();
            let v = db::get_all_videos_internal(&conn).map_err(|e| e.to_string())?;
            let f = db::get_all_folders_internal(&conn).map_err(|e| e.to_string())?;
            let c = db::get_all_clips_internal(&conn).map_err(|e| e.to_string())?;
            (v, f, c)
        };

        let payload = oplog::LibraryPayload {
            folders: local_folders,
            videos: local_videos,
            clips: local_clips,
            updated_at: oplog::current_time(),
        };

        let json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
        let existing = github_api::get_file_content(&self.token, &self.owner, &self.repo, "library.json").await;
        let sha = match existing {
            Ok(Some((_, sha))) => Some(sha),
            _ => None,
        };

        github_api::update_file(
            &self.token,
            &self.owner,
            &self.repo,
            "library.json",
            &json,
            sha,
            "Sync yClippy library",
        )
        .await
        .map_err(|e| e.to_string())?;

        watermarks.compacted_through.insert("library".to_string(), oplog::current_time());
        oplog::write_watermarks(app, &watermarks)?;

        Ok(())
    }

    pub async fn pull(&self, app: &AppHandle) -> Result<u64, String> {
        let content = github_api::get_file_content(&self.token, &self.owner, &self.repo, "library.json").await;
        let (json, _sha) = match content {
            Ok(Some((c, s))) => (c, s),
            _ => return Ok(0),
        };

        let payload: oplog::LibraryPayload = serde_json::from_str(&json).map_err(|e| e.to_string())?;

        let state: tauri::State<DbState> = app.state();
        let mut conn = state.conn.lock().unwrap();
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        let mut applied = 0u64;
        let now = oplog::current_time();

        for f in &payload.folders {
            if f.uid.is_none() {
                continue;
            }
            tx.execute(
                "INSERT INTO folders (uid, name, created_at, updated_at, deleted_at, sort_order, parent_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) ON CONFLICT(uid) DO UPDATE SET name = excluded.name, updated_at = excluded.updated_at, deleted_at = excluded.deleted_at, sort_order = excluded.sort_order, parent_id = excluded.parent_id",
                rusqlite::params![f.uid, f.name, f.created_at, now, f.deleted_at, f.sort_order, f.parent_id],
            ).map_err(|e| e.to_string())?;
            applied += 1;
        }

        for v in &payload.videos {
            let folder_id = if v.folder_id == Some(0) { None } else { v.folder_id };
            tx.execute(
                "INSERT INTO videos (id, title, thumbnail_url, duration, last_position, created_at, updated_at, deleted_at, folder_id, start_time, end_time, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12) ON CONFLICT(id) DO UPDATE SET title = excluded.title, thumbnail_url = excluded.thumbnail_url, updated_at = excluded.updated_at, deleted_at = excluded.deleted_at, folder_id = excluded.folder_id, start_time = excluded.start_time, end_time = excluded.end_time, sort_order = excluded.sort_order",
                rusqlite::params![v.id, v.title, v.thumbnail_url, v.duration, v.last_position, v.created_at, now, v.deleted_at, folder_id, v.start_time, v.end_time, v.sort_order],
            ).map_err(|e| e.to_string())?;
            applied += 1;
        }

        for c in &payload.clips {
            if c.uid.is_none() {
                continue;
            }
            if let Some(id) = c.id {
                tx.execute(
                    "INSERT INTO clips (id, uid, video_id, start_time, end_time, title, created_at, updated_at, deleted_at, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) ON CONFLICT(id) DO UPDATE SET uid = excluded.uid, video_id = excluded.video_id, start_time = excluded.start_time, end_time = excluded.end_time, title = excluded.title, updated_at = excluded.updated_at, deleted_at = excluded.deleted_at, sort_order = excluded.sort_order",
                    rusqlite::params![id, c.uid, c.video_id, c.start_time, c.end_time, c.title, c.created_at, now, c.deleted_at, c.sort_order],
                ).map_err(|e| e.to_string())?;
            } else {
                tx.execute(
                    "INSERT INTO clips (uid, video_id, start_time, end_time, title, created_at, updated_at, deleted_at, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?7, ?8)",
                    rusqlite::params![c.uid, c.video_id, c.start_time, c.end_time, c.title, c.created_at, c.deleted_at, c.sort_order],
                ).map_err(|e| e.to_string())?;
            }
            applied += 1;
        }

        tx.commit().map_err(|e| e.to_string())?;
        Ok(applied)
    }
}

fn parse_repo_url(url: &str) -> Option<(String, String)> {
    let url = url.trim_end_matches(".git");
    let parts: Vec<&str> = url.split('/').collect();
    if parts.len() >= 2 {
        let repo = parts[parts.len() - 1];
        let owner = parts[parts.len() - 2];
        if !repo.is_empty() && !owner.is_empty() {
            return Some((owner.to_string(), repo.to_string()));
        }
    }
    None
}