use crate::db::{self, Clip, DbState, Folder, Video};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum Op {
    #[serde(rename = "upsert")]
    Upsert {
        entity: String,
        uid: String,
        device: String,
        at: i64,
        fields: serde_json::Value,
    },
    #[serde(rename = "delete")]
    Delete {
        entity: String,
        uid: String,
        device: String,
        at: i64,
    },
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Watermarks {
    pub compacted_through: HashMap<String, i64>,
}

pub fn vault_dir(app: &AppHandle) -> Option<PathBuf> {
    let notes = app
        .path()
        .app_config_dir()
        .ok()
        .or_else(|| app.path().app_data_dir().ok())?
        .parent()
        .map(|p| p.join(".notes"))
        .or_else(|| {
            app.path()
                .home_dir()
                .ok()
                .map(|h| h.join(".notes"))
        })?;
    Some(notes.join("yclippy"))
}

pub fn ensure_vault_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = vault_dir(app).ok_or_else(|| "Could not determine vault directory".to_string())?;
    fs::create_dir_all(dir.join("devices")).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn library_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = ensure_vault_dir(app)?;
    Ok(dir.join("library.json"))
}

pub fn watermarks_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = ensure_vault_dir(app)?;
    Ok(dir.join("watermarks.json"))
}

pub fn device_log_path(app: &AppHandle, device: &str) -> Result<PathBuf, String> {
    let dir = ensure_vault_dir(app)?;
    Ok(dir.join("devices").join(format!("{device}.jsonl")))
}

pub fn write_op(app: &AppHandle, op: &Op) -> Result<(), String> {
    let (device, at) = match op {
        Op::Upsert { device, at, .. } => (device.clone(), *at),
        Op::Delete { device, at, .. } => (device.clone(), *at),
    };
    let path = device_log_path(app, &device)?;
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    let line = serde_json::to_string(op).map_err(|e| e.to_string())?;
    writeln!(f, "{line}").map_err(|e| e.to_string())?;
    let _ = at;
    Ok(())
}

#[allow(dead_code)]
pub fn append_upsert(
    app: &AppHandle,
    entity: &str,
    uid: &str,
    fields: serde_json::Value,
) -> Result<(), String> {
    let device = current_device(app)?;
    let at = current_time();
    let op = Op::Upsert {
        entity: entity.to_string(),
        uid: uid.to_string(),
        device: device.clone(),
        at,
        fields,
    };
    write_op(app, &op)
}

#[allow(dead_code)]
pub fn append_delete(app: &AppHandle, entity: &str, uid: &str) -> Result<(), String> {
    let device = current_device(app)?;
    let at = current_time();
    let op = Op::Delete {
        entity: entity.to_string(),
        uid: uid.to_string(),
        device,
        at,
    };
    write_op(app, &op)
}

pub fn current_device(app: &AppHandle) -> Result<String, String> {
    let state: tauri::State<DbState> = app.state();
    let conn = state.conn.lock().unwrap();
    let device: String = conn
        .query_row(
            "SELECT value FROM sync_metadata WHERE key = 'device_id'",
            [],
            |row| row.get::<_, String>(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(device)
}

pub fn current_time() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

pub fn read_watermarks(app: &AppHandle) -> Watermarks {
    let path = match watermarks_path(app) {
        Ok(p) => p,
        Err(_) => return Watermarks::default(),
    };
    if !path.exists() {
        return Watermarks::default();
    }
    let content = fs::read_to_string(&path).unwrap_or_default();
    serde_json::from_str(&content).unwrap_or_default()
}

pub fn write_watermarks(app: &AppHandle, wm: &Watermarks) -> Result<(), String> {
    let path = watermarks_path(app)?;
    let content = serde_json::to_string_pretty(wm).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())
}

#[allow(dead_code)]
pub fn load_library(app: &AppHandle) -> Result<LibraryPayload, String> {
    let path = library_path(app)?;
    if !path.exists() {
        return Ok(LibraryPayload::default());
    }
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

pub fn write_library(app: &AppHandle, payload: &LibraryPayload) -> Result<(), String> {
    let path = library_path(app)?;
    let content = serde_json::to_string_pretty(payload).map_err(|e| e.to_string())?;
    write_atomic(&path, content.as_bytes())
}

pub fn write_atomic(path: &Path, content: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, content).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct LibraryPayload {
    pub folders: Vec<Folder>,
    pub videos: Vec<Video>,
    pub clips: Vec<Clip>,
    pub updated_at: i64,
}

pub fn pull_ops_from_remote(app: &AppHandle) -> Result<u64, String> {
    let _dir = ensure_vault_dir(app)?;
    let watermarks = read_watermarks(app);
    let devices_dir = vault_dir(app)
        .ok_or_else(|| "vault dir missing".to_string())?
        .join("devices");

    if !devices_dir.exists() {
        return Ok(0);
    }

    let mut applied = 0u64;
    let entries = fs::read_dir(&devices_dir).map_err(|e| e.to_string())?;
    let mut device_files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|e| e == "jsonl").unwrap_or(false))
        .collect();
    device_files.sort();

    let state: tauri::State<DbState> = app.state();
    let mut conn = state.conn.lock().unwrap();
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    for path in device_files {
        let device = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let watermark = watermarks
            .compacted_through
            .get(&device)
            .copied()
            .unwrap_or(0);

        let file = match fs::File::open(&path) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let reader = BufReader::new(file);

        let mut last_ts = watermark;
        for line in reader.lines().map_while(|r| r.ok()) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let op: Op = match serde_json::from_str(line) {
                Ok(op) => op,
                Err(_) => continue,
            };
            let at = match &op {
                Op::Upsert { at, .. } => *at,
                Op::Delete { at, .. } => *at,
            };
            if at <= watermark {
                continue;
            }
            apply_op(&tx, &op)?;
            last_ts = last_ts.max(at);
            applied += 1;
        }

        if last_ts > watermark {
            tx.execute(
                "INSERT OR REPLACE INTO sync_metadata (key, value) VALUES (?1, ?2)",
                params![format!("watermark:{device}"), last_ts.to_string()],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(applied)
}

fn apply_op(conn: &rusqlite::Connection, op: &Op) -> Result<(), String> {
    match op {
        Op::Upsert {
            entity,
            uid,
            fields,
            at,
            ..
        } => match entity.as_str() {
            "folder" => {
                let folder: Folder = match serde_json::from_value(fields.clone()) {
                    Ok(f) => f,
                    Err(_) => return Ok(()),
                };
                conn.execute(
                    "INSERT INTO folders (uid, name, created_at, updated_at, deleted_at, sort_order, parent_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) ON CONFLICT(uid) DO UPDATE SET name = excluded.name, updated_at = excluded.updated_at, deleted_at = excluded.deleted_at, sort_order = excluded.sort_order, parent_id = excluded.parent_id",
                    params![uid, folder.name, folder.created_at, *at, folder.deleted_at, folder.sort_order, folder.parent_id],
                ).map_err(|e| e.to_string())?;
            }
            "video" => {
                let video: Video = match serde_json::from_value(fields.clone()) {
                    Ok(f) => f,
                    Err(_) => return Ok(()),
                };
                let folder_id = if video.folder_id == Some(0) {
                    None
                } else {
                    video.folder_id
                };
                conn.execute(
                    "INSERT INTO videos (id, title, thumbnail_url, duration, last_position, created_at, updated_at, deleted_at, folder_id, start_time, end_time, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12) ON CONFLICT(id) DO UPDATE SET title = excluded.title, thumbnail_url = excluded.thumbnail_url, updated_at = excluded.updated_at, deleted_at = excluded.deleted_at, folder_id = excluded.folder_id, start_time = excluded.start_time, end_time = excluded.end_time, sort_order = excluded.sort_order",
                    params![video.id, video.title, video.thumbnail_url, video.duration, video.last_position, video.created_at, *at, video.deleted_at, folder_id, video.start_time, video.end_time, video.sort_order],
                ).map_err(|e| e.to_string())?;
            }
            "clip" => {
                let clip: Clip = match serde_json::from_value(fields.clone()) {
                    Ok(f) => f,
                    Err(_) => return Ok(()),
                };
                if let Some(id) = clip.id {
                    conn.execute(
                        "INSERT INTO clips (id, uid, video_id, start_time, end_time, title, created_at, updated_at, deleted_at, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) ON CONFLICT(id) DO UPDATE SET uid = excluded.uid, video_id = excluded.video_id, start_time = excluded.start_time, end_time = excluded.end_time, title = excluded.title, updated_at = excluded.updated_at, deleted_at = excluded.deleted_at, sort_order = excluded.sort_order",
                        params![id, uid, clip.video_id, clip.start_time, clip.end_time, clip.title, clip.created_at, *at, clip.deleted_at, clip.sort_order],
                    ).map_err(|e| e.to_string())?;
                } else {
                    conn.execute(
                        "INSERT INTO clips (uid, video_id, start_time, end_time, title, created_at, updated_at, deleted_at, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?7, ?8)",
                        params![uid, clip.video_id, clip.start_time, clip.end_time, clip.title, clip.created_at, clip.deleted_at, clip.sort_order],
                    ).map_err(|e| e.to_string())?;
                }
            }
            _ => {}
        },
        Op::Delete {
            entity,
            uid,
            at,
            ..
        } => match entity.as_str() {
            "folder" => {
                let local_id: Option<i64> = conn
                    .query_row(
                        "SELECT id FROM folders WHERE uid = ?1",
                        params![uid],
                        |row| row.get(0),
                    )
                    .ok();
                if let Some(id) = local_id {
                    conn.execute(
                        "UPDATE folders SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1",
                        params![id, *at],
                    )
                    .map_err(|e| e.to_string())?;
                }
            }
            "video" => {
                conn.execute(
                    "UPDATE videos SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1",
                    params![uid, *at],
                )
                .map_err(|e| e.to_string())?;
            }
            "clip" => {
                let local_id: Option<i64> = conn
                    .query_row(
                        "SELECT id FROM clips WHERE uid = ?1",
                        params![uid],
                        |row| row.get(0),
                    )
                    .ok();
                if let Some(id) = local_id {
                    conn.execute(
                        "UPDATE clips SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1",
                        params![id, *at],
                    )
                    .map_err(|e| e.to_string())?;
                }
            }
            _ => {}
        },
    }
    Ok(())
}

#[tauri::command]
pub fn pull_remote(app: AppHandle) -> Result<u64, String> {
    pull_ops_from_remote(&app)
}

#[tauri::command]
pub fn compact_library(app: AppHandle) -> Result<(), String> {
    let mut watermarks = read_watermarks(&app);
    let devices_dir = vault_dir(&app)
        .ok_or_else(|| "vault dir missing".to_string())?
        .join("devices");
    fs::create_dir_all(&devices_dir).map_err(|e| e.to_string())?;

    let state: tauri::State<DbState> = app.state();
    let (folders, videos, clips) = {
        let conn = state.conn.lock().unwrap();
        let f = db::get_all_folders_internal(&conn).map_err(|e| e.to_string())?;
        let v = db::get_all_videos_internal(&conn).map_err(|e| e.to_string())?;
        let c = db::get_all_clips_internal(&conn).map_err(|e| e.to_string())?;
        (f, v, c)
    };

    let payload = LibraryPayload {
        folders,
        videos,
        clips,
        updated_at: current_time(),
    };
    write_library(&app, &payload)?;

    let entries = fs::read_dir(&devices_dir).map_err(|e| e.to_string())?;
    let device_files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|e| e == "jsonl").unwrap_or(false))
        .collect();

    for path in device_files {
        let device = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let file = match fs::File::open(&path) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let reader = BufReader::new(file);
        let mut max_ts = 0i64;
        for line in reader.lines().map_while(|r| r.ok()) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let op: Op = match serde_json::from_str(line) {
                Ok(op) => op,
                Err(_) => continue,
            };
            let at = match &op {
                Op::Upsert { at, .. } => *at,
                Op::Delete { at, .. } => *at,
            };
            max_ts = max_ts.max(at);
        }
        if max_ts > 0 {
            watermarks
                .compacted_through
                .insert(device.clone(), max_ts);
            let empty = path.with_extension("processed");
            let _ = fs::rename(&path, &empty);
            let _ = fs::remove_file(&empty);
        }
    }

    write_watermarks(&app, &watermarks)?;
    Ok(())
}