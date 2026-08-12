use crate::db::{self, Clip, DbState, Folder, Video};
use crate::github_api;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::{AppHandle, Manager, State};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(dead_code)]
struct Metadata {
    last_sync_timestamp: i64,
    device_id: String,
}

#[derive(Serialize)]
#[allow(dead_code)]
struct SyncFiles {
    videos: Vec<Video>,
    folders: Vec<Folder>,
    clips: Vec<Clip>,
    metadata: Metadata,
}

pub struct SyncEngine {
    token: String,
    owner: String,
    repo: String,
}

impl SyncEngine {
    pub fn new(token: String, repo_url: String) -> Option<Self> {
        let (owner, repo) = parse_repo_url(&repo_url)?;
        Some(Self { token, owner, repo })
    }

    pub async fn sync(&self, app: AppHandle) -> Result<String, String> {
        // 1. Fetch Remote Data (No DB lock needed)
        let remote_videos_res =
            github_api::get_file_content(&self.token, &self.owner, &self.repo, "videos.json").await;
        let remote_folders_res =
            github_api::get_file_content(&self.token, &self.owner, &self.repo, "folders.json")
                .await;
        let remote_clips_res =
            github_api::get_file_content(&self.token, &self.owner, &self.repo, "clips.json").await;

        let (remote_videos, v_sha): (Vec<Video>, Option<String>) = match remote_videos_res {
            Ok(Some((content, sha))) => (
                serde_json::from_str(&content).unwrap_or_default(),
                Some(sha),
            ),
            _ => (Vec::new(), None),
        };

        let (remote_folders, f_sha): (Vec<Folder>, Option<String>) = match remote_folders_res {
            Ok(Some((content, sha))) => (
                serde_json::from_str(&content).unwrap_or_default(),
                Some(sha),
            ),
            _ => (Vec::new(), None),
        };

        let (remote_clips, c_sha): (Vec<Clip>, Option<String>) = match remote_clips_res {
            Ok(Some((content, sha))) => (
                serde_json::from_str(&content).unwrap_or_default(),
                Some(sha),
            ),
            _ => (Vec::new(), None),
        };

        // 2. Fetch Local Data (Lock DB)
        let state: State<DbState> = app.state();

        let (local_videos, local_folders, local_clips) = {
            let conn = state.conn.lock().unwrap();
            let videos = db::get_all_videos_internal(&conn).map_err(|e| e.to_string())?;
            let folders = db::get_all_folders_internal(&conn).map_err(|e| e.to_string())?;
            let clips = db::get_all_clips_internal(&conn).map_err(|e| e.to_string())?;
            (videos, folders, clips)
        }; // Lock dropped here!

        // 3. Merge Strategies (No lock)
        let merged_videos = merge_entities(local_videos, remote_videos);
        let merged_folders = merge_entities(local_folders, remote_folders);
        let merged_clips = merge_entities_clips(local_clips, remote_clips);

        // 4. Update Local DB (Lock DB)
        {
            let conn = state.conn.lock().unwrap();
            for v in &merged_videos {
                db::upsert_video_internal(&conn, v).map_err(|e| e.to_string())?;
            }
            for f in &merged_folders {
                db::upsert_folder_internal(&conn, f).map_err(|e| e.to_string())?;
            }
            for c in &merged_clips {
                db::upsert_clip_internal(&conn, c).map_err(|e| e.to_string())?;
            }
        } // Lock dropped

        // 5. Push (Upload Merged Data) (No lock)
        let videos_json =
            serde_json::to_string_pretty(&merged_videos).map_err(|e| e.to_string())?;
        let folders_json =
            serde_json::to_string_pretty(&merged_folders).map_err(|e| e.to_string())?;
        let clips_json = serde_json::to_string_pretty(&merged_clips).map_err(|e| e.to_string())?;

        github_api::update_file(
            &self.token,
            &self.owner,
            &self.repo,
            "videos.json",
            &videos_json,
            v_sha,
            "Sync videos",
        )
        .await
        .map_err(|e| e.to_string())?;
        github_api::update_file(
            &self.token,
            &self.owner,
            &self.repo,
            "folders.json",
            &folders_json,
            f_sha,
            "Sync folders",
        )
        .await
        .map_err(|e| e.to_string())?;
        github_api::update_file(
            &self.token,
            &self.owner,
            &self.repo,
            "clips.json",
            &clips_json,
            c_sha,
            "Sync clips",
        )
        .await
        .map_err(|e| e.to_string())?;

        Ok("Sync completed".to_string())
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

trait Syncable {
    fn get_id(&self) -> String;
    fn get_updated_at(&self) -> i64;
}

impl Syncable for Video {
    fn get_id(&self) -> String {
        self.id.clone()
    }
    fn get_updated_at(&self) -> i64 {
        self.updated_at
    }
}

impl Syncable for Folder {
    fn get_id(&self) -> String {
        self.id.map(|i| i.to_string()).unwrap_or_default()
    }
    fn get_updated_at(&self) -> i64 {
        self.updated_at
    }
}

impl Syncable for Clip {
    fn get_id(&self) -> String {
        self.id.map(|i| i.to_string()).unwrap_or_default()
    }
    fn get_updated_at(&self) -> i64 {
        self.updated_at
    }
}

fn merge_entities<T: Syncable + Clone + std::fmt::Debug>(local: Vec<T>, remote: Vec<T>) -> Vec<T> {
    let mut map: HashMap<String, T> = HashMap::new();

    for item in local {
        map.insert(item.get_id(), item);
    }

    for item in remote {
        let id = item.get_id();
        if let Some(local_item) = map.get(&id) {
            if item.get_updated_at() > local_item.get_updated_at() {
                map.insert(id, item);
            }
        } else {
            map.insert(id, item);
        }
    }

    map.into_values().collect()
}

fn merge_entities_clips(local: Vec<Clip>, remote: Vec<Clip>) -> Vec<Clip> {
    merge_entities(local, remote)
}
