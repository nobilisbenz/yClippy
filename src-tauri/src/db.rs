use rusqlite::{params, Connection, OptionalExtension, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct YtRenamerData {
    pub clips: Vec<YtRenamerClip>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct YtRenamerClip {
    pub start_time: f64,
    pub end_time: f64,
    pub title: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct AppConfig {
    db_path: Option<String>,
    github_token: Option<String>,
    github_repo: Option<String>,
}

#[derive(Debug)]
pub struct StoredConfig {
    pub github_token: Option<String>,
    pub github_repo: Option<String>,
}

pub fn load_config_pub(app: &AppHandle) -> StoredConfig {
    let cfg = load_config(app);
    StoredConfig {
        github_token: cfg.github_token,
        github_repo: cfg.github_repo,
    }
}

fn get_config_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("failed to get app data dir")
        .join("config.json")
}

fn load_config(app: &AppHandle) -> AppConfig {
    let config_path = get_config_path(app);
    if config_path.exists() {
        let content = fs::read_to_string(config_path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        AppConfig::default()
    }
}

fn save_config(app: &AppHandle, config: &AppConfig) {
    let config_path = get_config_path(app);
    let content = match serde_json::to_string_pretty(config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("failed to serialize config: {e}");
            return;
        }
    };
    if let Err(e) = fs::write(&config_path, content) {
        eprintln!("failed to write config to {config_path:?}: {e}");
    }
}

fn current_time() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

fn setup_db(conn: &Connection) -> Result<()> {
    let _ = conn.execute(
        "CREATE TABLE IF NOT EXISTS videos (
            id TEXT PRIMARY KEY,
            title TEXT,
            thumbnail_url TEXT,
            duration INTEGER,
            last_position INTEGER DEFAULT 0,
            created_at INTEGER,
            updated_at INTEGER DEFAULT 0,
            deleted_at,
            folder_id INTEGER,
            start_time INTEGER DEFAULT 0,
            end_time INTEGER DEFAULT 0,
            sort_order INTEGER DEFAULT 0
        )",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE videos ADD COLUMN end_time INTEGER DEFAULT 0",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE videos ADD COLUMN sort_order INTEGER DEFAULT 0",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE videos ADD COLUMN updated_at INTEGER DEFAULT 0",
        [],
    );
    let _ = conn.execute("ALTER TABLE videos ADD COLUMN deleted_at INTEGER", []);

    conn.execute(
        "CREATE TABLE IF NOT EXISTS clips (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uid TEXT UNIQUE,
            video_id TEXT,
            start_time INTEGER,
            end_time INTEGER,
            title TEXT,
            created_at INTEGER,
            updated_at INTEGER DEFAULT 0,
            deleted_at INTEGER,
            sort_order INTEGER DEFAULT 0,
            FOREIGN KEY(video_id) REFERENCES videos(id)
        )",
        [],
    )?;

    let _ = conn.execute("ALTER TABLE clips ADD COLUMN uid TEXT", []);
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_clips_uid ON clips(uid)",
        [],
    )?;

    let _ = conn.execute(
        "ALTER TABLE clips ADD COLUMN updated_at INTEGER DEFAULT 0",
        [],
    );
    let _ = conn.execute("ALTER TABLE clips ADD COLUMN deleted_at INTEGER", []);
    let _ = conn.execute(
        "ALTER TABLE clips ADD COLUMN sort_order INTEGER DEFAULT 0",
        [],
    );

    conn.execute(
        "CREATE TABLE IF NOT EXISTS folders (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uid TEXT UNIQUE,
            name TEXT NOT NULL,
            created_at INTEGER,
            updated_at INTEGER DEFAULT 0,
            deleted_at INTEGER,
            sort_order INTEGER DEFAULT 0,
            parent_id INTEGER
        )",
        [],
    )?;

    let _ = conn.execute("ALTER TABLE folders ADD COLUMN uid TEXT", []);
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_folders_uid ON folders(uid)",
        [],
    )?;

    let _ = conn.execute("ALTER TABLE folders ADD COLUMN parent_id INTEGER", []);
    let _ = conn.execute(
        "ALTER TABLE folders ADD COLUMN updated_at INTEGER DEFAULT 0",
        [],
    );
    let _ = conn.execute("ALTER TABLE folders ADD COLUMN deleted_at INTEGER", []);
    let _ = conn.execute(
        "ALTER TABLE folders ADD COLUMN sort_order INTEGER DEFAULT 0",
        [],
    );

    conn.execute(
        "CREATE TABLE IF NOT EXISTS sync_metadata (
            key TEXT PRIMARY KEY,
            value TEXT
        )",
        [],
    )?;

    backfill_uids(conn)?;

    Ok(())
}

fn backfill_uids(conn: &Connection) -> Result<()> {
    let mut stmt = conn
        .prepare("SELECT id FROM folders WHERE uid IS NULL OR uid = ''")
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    let folder_ids: Vec<i64> = stmt
        .query_map([], |row| row.get::<_, i64>(0))?
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt);

    for id in folder_ids {
        let uid = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "UPDATE folders SET uid = ?1 WHERE id = ?2",
            params![uid, id],
        )?;
    }

    let mut stmt = conn
        .prepare("SELECT id FROM clips WHERE uid IS NULL OR uid = ''")
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    let clip_ids: Vec<i64> = stmt
        .query_map([], |row| row.get::<_, i64>(0))?
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt);

    for id in clip_ids {
        let uid = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "UPDATE clips SET uid = ?1 WHERE id = ?2",
            params![uid, id],
        )?;
    }

    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Video {
    pub id: String,
    pub title: String,
    pub thumbnail_url: String,
    pub duration: i32,
    pub last_position: i32,
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
    #[serde(default)]
    pub deleted_at: Option<i64>,
    pub folder_id: Option<i64>,
    pub start_time: i32,
    pub end_time: i32,
    pub sort_order: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Clip {
    pub id: Option<i32>,
    #[serde(default)]
    pub uid: Option<String>,
    pub video_id: String,
    pub start_time: i32,
    pub end_time: i32,
    pub title: String,
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
    #[serde(default)]
    pub deleted_at: Option<i64>,
    pub sort_order: i32,
}

pub struct DbState {
    pub conn: Mutex<Connection>,
    #[allow(dead_code)]
    pub device_id: String,
}

pub fn init_db(app: &AppHandle) -> Result<DbState> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .expect("failed to get app data dir");
    if !app_data_dir.exists() {
        fs::create_dir_all(&app_data_dir).expect("failed to create app data dir");
    }

    let config = load_config(app);
    let db_path = if let Some(path) = config.db_path {
        PathBuf::from(path)
    } else {
        app_data_dir.join("yclippy.db")
    };

    let conn = Connection::open(db_path)?;
    setup_db(&conn)?;

    let device_id: String = conn
        .query_row(
            "SELECT value FROM sync_metadata WHERE key = 'device_id'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| {
            let new_id = uuid::Uuid::new_v4().to_string();
            let _ = conn.execute(
                "INSERT OR REPLACE INTO sync_metadata (key, value) VALUES ('device_id', ?1)",
                params![new_id],
            );
            new_id
        });

    Ok(DbState {
        conn: Mutex::new(conn),
        device_id,
    })
}

#[tauri::command]
pub fn get_db_path(app: AppHandle) -> Result<String, String> {
    let config = load_config(&app);
    if let Some(path) = config.db_path {
        Ok(path)
    } else {
        let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
        Ok(app_data_dir
            .join("yclippy.db")
            .to_string_lossy()
            .to_string())
    }
}

#[tauri::command]
pub fn set_db_path(app: AppHandle, state: State<DbState>, path: String) -> Result<(), String> {
    let new_path = PathBuf::from(&path);
    if let Some(parent) = new_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            return Err(format!(
                "Parent directory does not exist: {}",
                parent.display()
            ));
        }
    }

    let test_conn = Connection::open(&new_path).map_err(|e| e.to_string())?;
    setup_db(&test_conn).map_err(|e| e.to_string())?;
    drop(test_conn);

    let conn = Connection::open(&new_path).map_err(|e| e.to_string())?;

    {
        let mut global_conn = state.conn.lock().unwrap();
        *global_conn = conn;
    }

    let mut config = load_config(&app);
    config.db_path = Some(path);
    save_config(&app, &config);

    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GithubConfigPublic {
    pub github_repo: String,
    pub token_present: bool,
}

#[tauri::command]
pub fn get_github_config(app: AppHandle) -> Result<GithubConfigPublic, String> {
    let config = load_config(&app);
    Ok(GithubConfigPublic {
        github_repo: config.github_repo.unwrap_or_default(),
        token_present: config
            .github_token
            .as_ref()
            .map(|t| !t.is_empty())
            .unwrap_or(false),
    })
}

#[tauri::command]
pub fn set_github_config(
    app: AppHandle,
    github_repo: String,
    github_token: Option<String>,
) -> Result<(), String> {
    let mut config = load_config(&app);
    config.github_repo = if github_repo.is_empty() {
        None
    } else {
        Some(github_repo)
    };
    if let Some(token) = github_token {
        config.github_token = if token.is_empty() {
            None
        } else {
            Some(token)
        };
    }
    save_config(&app, &config);
    Ok(())
}

#[tauri::command]
pub fn clear_github_token(app: AppHandle) -> Result<(), String> {
    let mut config = load_config(&app);
    config.github_token = None;
    save_config(&app, &config);
    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Folder {
    pub id: Option<i64>,
    #[serde(default)]
    pub uid: Option<String>,
    pub name: String,
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
    #[serde(default)]
    pub deleted_at: Option<i64>,
    #[serde(default)]
    pub sort_order: i32,
    pub parent_id: Option<i64>,
}

#[tauri::command]
pub fn get_folders(state: State<DbState>) -> Result<Vec<Folder>, String> {
    let conn = state.conn.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, uid, name, created_at, updated_at, deleted_at, sort_order, parent_id FROM folders WHERE deleted_at IS NULL ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;

    let iter = stmt
        .query_map([], |row| {
            Ok(Folder {
                id: row.get(0)?,
                uid: row.get(1)?,
                name: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                deleted_at: row.get(5)?,
                sort_order: row.get(6)?,
                parent_id: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for item in iter {
        result.push(item.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

#[tauri::command]
pub fn save_folder(state: State<DbState>, app: AppHandle, folder: Folder) -> Result<i64, String> {
    let conn = state.conn.lock().unwrap();
    let now = current_time();

    let uid = match folder.uid.clone().filter(|s| !s.is_empty()) {
        Some(u) => u,
        None => uuid::Uuid::new_v4().to_string(),
    };

    let id = if let Some(existing_id) = folder.id {
        conn.execute(
            "UPDATE folders SET name = ?1, uid = ?2, updated_at = ?3, deleted_at = ?4, sort_order = ?5, parent_id = ?6 WHERE id = ?7",
            params![folder.name, uid, now, folder.deleted_at, folder.sort_order, folder.parent_id, existing_id],
        )
        .map_err(|e| e.to_string())?;
        existing_id
    } else {
        conn.execute(
            "INSERT INTO folders (uid, name, created_at, updated_at, deleted_at, sort_order, parent_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![uid, folder.name, folder.created_at, now, folder.deleted_at, folder.sort_order, folder.parent_id],
        )
        .map_err(|e| e.to_string())?;
        conn.last_insert_rowid()
    };

    drop(conn);

    let snapshot = crate::db::Folder {
        id: Some(id),
        uid: Some(uid.clone()),
        name: folder.name.clone(),
        created_at: folder.created_at,
        updated_at: now,
        deleted_at: folder.deleted_at,
        sort_order: folder.sort_order,
        parent_id: folder.parent_id,
    };
    let fields = serde_json::to_value(&snapshot).unwrap_or(serde_json::Value::Null);
    let _ = crate::oplog::append_upsert(&app, "folder", &uid, fields);

    Ok(id)
}

#[tauri::command]
pub fn update_folder_parent(
    state: State<DbState>,
    folder_id: i64,
    parent_id: Option<i64>,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let now = current_time();
    conn.execute(
        "UPDATE folders SET parent_id = ?2, updated_at = ?3 WHERE id = ?1",
        params![folder_id, parent_id, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_folder(state: State<DbState>, app: AppHandle, id: i64) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let now = current_time();

    let uid: Option<String> = conn
        .query_row(
            "SELECT uid FROM folders WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .ok();

    conn.execute(
        "UPDATE folders SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1",
        params![id, now],
    )
    .map_err(|e| e.to_string())?;

    let parent_id: Option<i64> = conn
        .query_row(
            "SELECT parent_id FROM folders WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()
        .unwrap_or(None);

    conn.execute(
        "UPDATE videos SET folder_id = ?2, updated_at = ?3 WHERE folder_id = ?1",
        params![id, parent_id, now],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE folders SET parent_id = ?2, updated_at = ?3 WHERE parent_id = ?1",
        params![id, parent_id, now],
    )
    .map_err(|e| e.to_string())?;

    if let Some(uid) = uid {
        let _ = crate::oplog::append_delete(&app, "folder", &uid);
    }

    Ok(())
}

#[tauri::command]
pub fn update_video_folder(
    state: State<DbState>,
    video_id: String,
    folder_id: Option<i64>,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let now = current_time();
    conn.execute(
        "UPDATE videos SET folder_id = ?2, updated_at = ?3 WHERE id = ?1",
        params![video_id, folder_id, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_videos(state: State<DbState>) -> Result<Vec<Video>, String> {
    let conn = state.conn.lock().unwrap();
    let mut stmt = conn.prepare("SELECT id, title, thumbnail_url, duration, last_position, created_at, updated_at, deleted_at, folder_id, start_time, end_time, sort_order FROM videos WHERE deleted_at IS NULL ORDER BY created_at DESC").map_err(|e| e.to_string())?;

    let video_iter = stmt
        .query_map([], |row| {
            Ok(Video {
                id: row.get(0)?,
                title: row.get(1)?,
                thumbnail_url: row.get(2)?,
                duration: row.get(3)?,
                last_position: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                deleted_at: row.get(7)?,
                folder_id: row.get(8)?,
                start_time: row.get(9)?,
                end_time: row.get(10)?,
                sort_order: row.get(11)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut videos = Vec::new();
    for video in video_iter {
        videos.push(video.map_err(|e| e.to_string())?);
    }
    Ok(videos)
}

#[tauri::command]
pub fn save_video(state: State<DbState>, app: AppHandle, video: Video) -> Result<(), String> {
    let folder_id = if video.folder_id == Some(0) {
        None
    } else {
        video.folder_id
    };
    let conn = state.conn.lock().unwrap();
    let now = current_time();

    conn.execute(
        "INSERT INTO videos (id, title, thumbnail_url, duration, last_position, created_at, updated_at, deleted_at, folder_id, start_time, end_time, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12) ON CONFLICT(id) DO UPDATE SET title = COALESCE(NULLIF(excluded.title, ''), videos.title), thumbnail_url = COALESCE(NULLIF(excluded.thumbnail_url, ''), videos.thumbnail_url), updated_at = excluded.updated_at, deleted_at = excluded.deleted_at, folder_id = excluded.folder_id, start_time = excluded.start_time, end_time = excluded.end_time, sort_order = excluded.sort_order",
        params![video.id, video.title, video.thumbnail_url, video.duration, video.last_position, video.created_at, now, video.deleted_at, folder_id, video.start_time, video.end_time, video.sort_order],
    ).map_err(|e| e.to_string())?;
    drop(conn);

    let snapshot = crate::db::Video {
        id: video.id.clone(),
        title: video.title.clone(),
        thumbnail_url: video.thumbnail_url.clone(),
        duration: video.duration,
        last_position: video.last_position,
        created_at: video.created_at,
        updated_at: now,
        deleted_at: video.deleted_at,
        folder_id,
        start_time: video.start_time,
        end_time: video.end_time,
        sort_order: video.sort_order,
    };
    let fields = serde_json::to_value(&snapshot).unwrap_or(serde_json::Value::Null);
    let _ = crate::oplog::append_upsert(&app, "video", &video.id, fields);

    Ok(())
}

#[tauri::command]
pub fn get_clips(state: State<DbState>, video_id: String) -> Result<Vec<Clip>, String> {
    let conn = state.conn.lock().unwrap();
    let mut stmt = conn.prepare("SELECT id, uid, video_id, start_time, end_time, title, created_at, updated_at, deleted_at, sort_order FROM clips WHERE video_id = ?1 AND deleted_at IS NULL ORDER BY sort_order ASC").map_err(|e| e.to_string())?;

    let clip_iter = stmt
        .query_map(params![video_id], |row| {
            Ok(Clip {
                id: row.get(0)?,
                uid: row.get(1)?,
                video_id: row.get(2)?,
                start_time: row.get(3)?,
                end_time: row.get(4)?,
                title: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                deleted_at: row.get(8)?,
                sort_order: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut clips = Vec::new();
    for clip in clip_iter {
        clips.push(clip.map_err(|e| e.to_string())?);
    }
    Ok(clips)
}

#[tauri::command]
pub fn save_clip(state: State<DbState>, app: AppHandle, clip: Clip) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let now = current_time();

    let (id, uid) = if let Some(id) = clip.id {
        let uid = match clip.uid.clone().filter(|s| !s.is_empty()) {
            Some(u) => u,
            None => {
                let existing: Option<String> = conn
                    .query_row("SELECT uid FROM clips WHERE id = ?1", params![id], |row| row.get(0))
                    .ok();
                existing.unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
            }
        };
        conn.execute(
            "UPDATE clips SET uid = ?1, video_id = ?2, start_time = ?3, end_time = ?4, title = ?5, updated_at = ?6, sort_order = ?7 WHERE id = ?8",
            params![uid, clip.video_id, clip.start_time, clip.end_time, clip.title, now, clip.sort_order, id],
        ).map_err(|e| e.to_string())?;
        (id, uid)
    } else {
        let max_order: Option<i32> = conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), -1) FROM clips WHERE video_id = ?1 AND deleted_at IS NULL",
                params![clip.video_id],
                |row| row.get(0),
            )
            .ok();
        let new_sort_order = max_order.unwrap_or(-1) + 1;
        let uid = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO clips (uid, video_id, start_time, end_time, title, created_at, updated_at, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?7)",
            params![uid, clip.video_id, clip.start_time, clip.end_time, clip.title, clip.created_at, new_sort_order],
        ).map_err(|e| e.to_string())?;
        (conn.last_insert_rowid() as i32, uid)
    };
    drop(conn);

    let snapshot = crate::db::Clip {
        id: Some(id),
        uid: Some(uid.clone()),
        video_id: clip.video_id.clone(),
        start_time: clip.start_time,
        end_time: clip.end_time,
        title: clip.title.clone(),
        created_at: clip.created_at,
        updated_at: now,
        deleted_at: clip.deleted_at,
        sort_order: clip.sort_order,
    };
    let fields = serde_json::to_value(&snapshot).unwrap_or(serde_json::Value::Null);
    let _ = crate::oplog::append_upsert(&app, "clip", &uid, fields);

    Ok(())
}

#[tauri::command]
pub fn delete_clip(state: State<DbState>, app: AppHandle, id: i32) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let now = current_time();

    let uid: Option<String> = conn
        .query_row(
            "SELECT uid FROM clips WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .ok();

    conn.execute(
        "UPDATE clips SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1",
        params![id, now],
    )
    .map_err(|e| e.to_string())?;

    if let Some(uid) = uid {
        let _ = crate::oplog::append_delete(&app, "clip", &uid);
    }
    Ok(())
}

#[tauri::command]
pub fn rename_folder(state: State<DbState>, id: i64, name: String) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let now = current_time();
    conn.execute(
        "UPDATE folders SET name = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, name, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn rename_video(state: State<DbState>, id: String, title: String) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let now = current_time();
    conn.execute(
        "UPDATE videos SET title = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, title, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn update_video_metadata(
    state: State<DbState>,
    id: String,
    title: String,
    start_time: i32,
    end_time: i32,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let now = current_time();
    conn.execute(
        "UPDATE videos SET title = ?2, start_time = ?3, end_time = ?4, updated_at = ?5 WHERE id = ?1",
        params![id, title, start_time, end_time, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn rename_clip(state: State<DbState>, id: i32, title: String) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let now = current_time();
    conn.execute(
        "UPDATE clips SET title = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, title, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_video(state: State<DbState>, app: AppHandle, id: String) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let now = current_time();

    conn.execute(
        "UPDATE videos SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1",
        params![id.clone(), now],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE clips SET deleted_at = ?2, updated_at = ?2 WHERE video_id = ?1",
        params![id.clone(), now],
    )
    .map_err(|e| e.to_string())?;

    let _ = crate::oplog::append_delete(&app, "video", &id);

    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Backup {
    pub folders: Vec<Folder>,
    pub videos: Vec<Video>,
    pub clips: Vec<Clip>,
}

#[tauri::command]
pub fn export_db(state: State<DbState>) -> Result<Backup, String> {
    let conn = state.conn.lock().unwrap();

    let mut stmt = conn
        .prepare("SELECT id, uid, name, created_at, updated_at, deleted_at, sort_order, parent_id FROM folders WHERE deleted_at IS NULL ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;
    let folder_iter = stmt
        .query_map([], |row| {
            Ok(Folder {
                id: row.get(0)?,
                uid: row.get(1)?,
                name: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                deleted_at: row.get(5)?,
                sort_order: row.get(6)?,
                parent_id: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut folders = Vec::new();
    for f in folder_iter {
        folders.push(f.map_err(|e| e.to_string())?);
    }

    let mut stmt = conn.prepare("SELECT id, title, thumbnail_url, duration, last_position, created_at, updated_at, deleted_at, folder_id, start_time, end_time, sort_order FROM videos WHERE deleted_at IS NULL ORDER BY created_at DESC").map_err(|e| e.to_string())?;
    let video_iter = stmt
        .query_map([], |row| {
            Ok(Video {
                id: row.get(0)?,
                title: row.get(1)?,
                thumbnail_url: row.get(2)?,
                duration: row.get(3)?,
                last_position: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                deleted_at: row.get(7)?,
                folder_id: row.get(8)?,
                start_time: row.get(9)?,
                end_time: row.get(10)?,
                sort_order: row.get(11)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut videos = Vec::new();
    for v in video_iter {
        videos.push(v.map_err(|e| e.to_string())?);
    }

    let mut stmt = conn
        .prepare("SELECT id, uid, video_id, start_time, end_time, title, created_at, updated_at, deleted_at, sort_order FROM clips WHERE deleted_at IS NULL")
        .map_err(|e| e.to_string())?;
    let clip_iter = stmt
        .query_map([], |row| {
            Ok(Clip {
                id: row.get(0)?,
                uid: row.get(1)?,
                video_id: row.get(2)?,
                start_time: row.get(3)?,
                end_time: row.get(4)?,
                title: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                deleted_at: row.get(8)?,
                sort_order: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut clips = Vec::new();
    for clip in clip_iter {
        clips.push(clip.map_err(|e| e.to_string())?);
    }

    Ok(Backup {
        folders,
        videos,
        clips,
    })
}

#[tauri::command]
pub fn import_db(state: State<DbState>, backup: Backup) -> Result<(), String> {
    let mut conn = state.conn.lock().unwrap();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let now = current_time();

    let mut uid_to_local_id: HashMap<String, i64> = HashMap::new();

    for folder in &backup.folders {
        let uid = folder.uid.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let existing_local_id: Option<i64> = tx
            .query_row(
                "SELECT id FROM folders WHERE uid = ?1",
                params![uid],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = existing_local_id {
            uid_to_local_id.insert(uid.clone(), id);
        } else {
            tx.execute(
                "INSERT INTO folders (uid, name, created_at, updated_at, deleted_at, sort_order, parent_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![uid, folder.name, folder.created_at, now, folder.deleted_at, folder.sort_order, folder.parent_id],
            ).map_err(|e| e.to_string())?;
            let new_id = tx.last_insert_rowid();
            uid_to_local_id.insert(uid.clone(), new_id);
        }
    }

    for folder in &backup.folders {
        let uid = folder.uid.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        if let Some(new_parent_uid) = backup
            .folders
            .iter()
            .find(|f| f.id == folder.parent_id)
            .and_then(|f| f.uid.clone())
        {
            if let Some(&new_parent_id) = uid_to_local_id.get(&new_parent_uid) {
                if let Some(&local_id) = uid_to_local_id.get(&uid) {
                    tx.execute(
                        "UPDATE folders SET parent_id = ?1 WHERE id = ?2",
                        params![new_parent_id, local_id],
                    )
                    .map_err(|e| e.to_string())?;
                }
            }
        }
    }

    for video in backup.videos {
        let new_folder_id = video.folder_id.and_then(|orig_id| {
            backup
                .folders
                .iter()
                .find(|f| f.id == Some(orig_id))
                .and_then(|f| f.uid.clone())
                .and_then(|u| uid_to_local_id.get(&u).copied())
        });

        tx.execute(
            "INSERT INTO videos (id, title, thumbnail_url, duration, last_position, created_at, updated_at, folder_id, start_time, end_time, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11) ON CONFLICT(id) DO UPDATE SET title = excluded.title, thumbnail_url = excluded.thumbnail_url, folder_id = excluded.folder_id, start_time = excluded.start_time, end_time = excluded.end_time, sort_order = excluded.sort_order, updated_at = excluded.updated_at",
             params![video.id, video.title, video.thumbnail_url, video.duration, video.last_position, video.created_at, now, new_folder_id, video.start_time, video.end_time, video.sort_order],
        ).map_err(|e| e.to_string())?;
    }

    for clip in backup.clips {
        let uid = clip.uid.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let existing_local_id: Option<i64> = tx
            .query_row(
                "SELECT id FROM clips WHERE uid = ?1",
                params![uid],
                |row| row.get(0),
            )
            .ok();

        if existing_local_id.is_none() {
            tx.execute(
                "INSERT INTO clips (uid, video_id, start_time, end_time, title, created_at, updated_at, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?7)",
                params![uid, clip.video_id, clip.start_time, clip.end_time, clip.title, clip.created_at, clip.sort_order],
            ).map_err(|e| e.to_string())?;
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn update_clip(state: State<DbState>, clip: Clip) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let now = current_time();
    conn.execute(
        "UPDATE clips SET title = ?2, start_time = ?3, end_time = ?4, updated_at = ?5 WHERE id = ?1",
        params![clip.id, clip.title, clip.start_time, clip.end_time, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SortItem {
    pub id: i64,
    pub sort_order: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VideoSortItem {
    pub id: String,
    pub sort_order: i32,
}

#[tauri::command]
pub fn restore_video(state: State<DbState>, app: AppHandle, id: String) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let now = current_time();
    conn.execute(
        "UPDATE videos SET deleted_at = NULL, updated_at = ?2 WHERE id = ?1",
        params![id.clone(), now],
    )
    .map_err(|e| e.to_string())?;

    let snapshot: Option<crate::db::Video> = conn
        .query_row(
            "SELECT id, title, thumbnail_url, duration, last_position, created_at, updated_at, deleted_at, folder_id, start_time, end_time, sort_order FROM videos WHERE id = ?1",
            params![id.clone()],
            |row| Ok(crate::db::Video {
                id: row.get(0)?,
                title: row.get(1)?,
                thumbnail_url: row.get(2)?,
                duration: row.get(3)?,
                last_position: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                deleted_at: row.get(7)?,
                folder_id: row.get(8)?,
                start_time: row.get(9)?,
                end_time: row.get(10)?,
                sort_order: row.get(11)?,
            }),
        )
        .ok();
    drop(conn);

    if let Some(v) = snapshot {
        let fields = serde_json::to_value(&v).unwrap_or(serde_json::Value::Null);
        let _ = crate::oplog::append_upsert(&app, "video", &id, fields);
    }
    Ok(())
}

#[tauri::command]
pub fn restore_folder(state: State<DbState>, app: AppHandle, id: i64) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let now = current_time();
    conn.execute(
        "UPDATE folders SET deleted_at = NULL, updated_at = ?2 WHERE id = ?1",
        params![id, now],
    )
    .map_err(|e| e.to_string())?;

    let snapshot: Option<crate::db::Folder> = conn
        .query_row(
            "SELECT id, uid, name, created_at, updated_at, deleted_at, sort_order, parent_id FROM folders WHERE id = ?1",
            params![id],
            |row| Ok(crate::db::Folder {
                id: row.get(0)?,
                uid: row.get(1)?,
                name: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                deleted_at: row.get(5)?,
                sort_order: row.get(6)?,
                parent_id: row.get(7)?,
            }),
        )
        .ok();
    drop(conn);

    if let Some(folder) = snapshot {
        if let Some(uid) = folder.uid.clone() {
            let fields = serde_json::to_value(&folder).unwrap_or(serde_json::Value::Null);
            let _ = crate::oplog::append_upsert(&app, "folder", &uid, fields);
        }
    }
    Ok(())
}

#[tauri::command]
pub fn restore_clip(state: State<DbState>, app: AppHandle, id: i32) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let now = current_time();
    conn.execute(
        "UPDATE clips SET deleted_at = NULL, updated_at = ?2 WHERE id = ?1",
        params![id, now],
    )
    .map_err(|e| e.to_string())?;

    let snapshot: Option<crate::db::Clip> = conn
        .query_row(
            "SELECT id, uid, video_id, start_time, end_time, title, created_at, updated_at, deleted_at, sort_order FROM clips WHERE id = ?1",
            params![id],
            |row| Ok(crate::db::Clip {
                id: row.get(0)?,
                uid: row.get(1)?,
                video_id: row.get(2)?,
                start_time: row.get(3)?,
                end_time: row.get(4)?,
                title: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                deleted_at: row.get(8)?,
                sort_order: row.get(9)?,
            }),
        )
        .ok();
    drop(conn);

    if let Some(clip) = snapshot {
        if let Some(uid) = clip.uid.clone() {
            let fields = serde_json::to_value(&clip).unwrap_or(serde_json::Value::Null);
            let _ = crate::oplog::append_upsert(&app, "clip", &uid, fields);
        }
    }
    Ok(())
}

#[tauri::command]
pub fn update_sort_order(
    state: State<DbState>,
    folders: Vec<SortItem>,
    videos: Vec<VideoSortItem>,
) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let now = current_time();

    for folder in folders {
        conn.execute(
            "UPDATE folders SET sort_order = ?1, updated_at = ?2 WHERE id = ?3",
            params![folder.sort_order, now, folder.id],
        )
        .map_err(|e| e.to_string())?;
    }

    for video in videos {
        conn.execute(
            "UPDATE videos SET sort_order = ?1, updated_at = ?2 WHERE id = ?3",
            params![video.sort_order, now, video.id],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub fn update_clip_sort_order(state: State<DbState>, clips: Vec<SortItem>) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let now = current_time();

    for clip in clips {
        conn.execute(
            "UPDATE clips SET sort_order = ?1, updated_at = ?2 WHERE id = ?3",
            params![clip.sort_order, now, clip.id],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

// Internal methods for Sync Engine

pub fn get_all_folders_internal(conn: &Connection) -> Result<Vec<Folder>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, uid, name, created_at, updated_at, deleted_at, sort_order, parent_id FROM folders",
    )?;
    let folder_iter = stmt.query_map([], |row| {
        Ok(Folder {
            id: row.get(0)?,
            uid: row.get(1)?,
            name: row.get(2)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
            deleted_at: row.get(5)?,
            sort_order: row.get(6)?,
            parent_id: row.get(7)?,
        })
    })?;
    let mut folders = Vec::new();
    for f in folder_iter {
        folders.push(f?);
    }
    Ok(folders)
}

pub fn get_all_videos_internal(conn: &Connection) -> Result<Vec<Video>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT id, title, thumbnail_url, duration, last_position, created_at, updated_at, deleted_at, folder_id, start_time, end_time, sort_order FROM videos")?;
    let video_iter = stmt.query_map([], |row| {
        Ok(Video {
            id: row.get(0)?,
            title: row.get(1)?,
            thumbnail_url: row.get(2)?,
            duration: row.get(3)?,
            last_position: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
            deleted_at: row.get(7)?,
            folder_id: row.get(8)?,
            start_time: row.get(9)?,
            end_time: row.get(10)?,
            sort_order: row.get(11)?,
        })
    })?;
    let mut videos = Vec::new();
    for v in video_iter {
        videos.push(v?);
    }
    Ok(videos)
}

pub fn get_all_clips_internal(conn: &Connection) -> Result<Vec<Clip>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT id, uid, video_id, start_time, end_time, title, created_at, updated_at, deleted_at, sort_order FROM clips")?;
    let clip_iter = stmt.query_map([], |row| {
        Ok(Clip {
            id: row.get(0)?,
            uid: row.get(1)?,
            video_id: row.get(2)?,
            start_time: row.get(3)?,
            end_time: row.get(4)?,
            title: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
            deleted_at: row.get(8)?,
            sort_order: row.get(9)?,
        })
    })?;
    let mut clips = Vec::new();
    for c in clip_iter {
        clips.push(c?);
    }
    Ok(clips)
}

#[allow(dead_code)]
pub fn upsert_folder_internal(conn: &Connection, folder: &Folder) -> Result<(), rusqlite::Error> {
    let uid = folder.uid.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    conn.execute(
        "INSERT INTO folders (id, uid, name, created_at, updated_at, deleted_at, sort_order, parent_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) ON CONFLICT(id) DO UPDATE SET uid = excluded.uid, name = excluded.name, updated_at = excluded.updated_at, deleted_at = excluded.deleted_at, sort_order = excluded.sort_order, parent_id = excluded.parent_id",
        params![folder.id, uid, folder.name, folder.created_at, folder.updated_at, folder.deleted_at, folder.sort_order, folder.parent_id],
    )?;
    Ok(())
}

#[allow(dead_code)]
pub fn upsert_video_internal(conn: &Connection, video: &Video) -> Result<(), rusqlite::Error> {
    let folder_id = if video.folder_id == Some(0) {
        None
    } else {
        video.folder_id
    };
    conn.execute(
        "INSERT OR REPLACE INTO videos (id, title, thumbnail_url, duration, last_position, created_at, updated_at, deleted_at, folder_id, start_time, end_time, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![video.id, video.title, video.thumbnail_url, video.duration, video.last_position, video.created_at, video.updated_at, video.deleted_at, folder_id, video.start_time, video.end_time, video.sort_order],
    )?;
    Ok(())
}

#[allow(dead_code)]
pub fn upsert_clip_internal(conn: &Connection, clip: &Clip) -> Result<(), rusqlite::Error> {
    let uid = clip.uid.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    if let Some(id) = clip.id {
        conn.execute(
            "INSERT INTO clips (id, uid, video_id, start_time, end_time, title, created_at, updated_at, deleted_at, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) ON CONFLICT(id) DO UPDATE SET uid = excluded.uid, video_id = excluded.video_id, start_time = excluded.start_time, end_time = excluded.end_time, title = excluded.title, updated_at = excluded.updated_at, deleted_at = excluded.deleted_at, sort_order = excluded.sort_order",
             params![id, uid, clip.video_id, clip.start_time, clip.end_time, clip.title, clip.created_at, clip.updated_at, clip.deleted_at, clip.sort_order],
        )?;
    } else {
        conn.execute(
            "INSERT INTO clips (uid, video_id, start_time, end_time, title, created_at, updated_at, deleted_at, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?7, ?8)",
             params![uid, clip.video_id, clip.start_time, clip.end_time, clip.title, clip.created_at, clip.updated_at, clip.deleted_at, clip.sort_order],
        )?;
    }
    Ok(())
}

fn extract_video_id(input: &str) -> Option<String> {
    let id_chars = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';

    for line in input.lines() {
        let line = line.trim();
        if line.len() == 11 && line.chars().all(id_chars) {
            return Some(line.to_string());
        }

        if let Some((scheme_end, _)) = line.find("://").map(|i| (i + 3, ())) {
            let after_scheme = &line[scheme_end.min(line.len())..];
            let host_end = after_scheme.find(|c: char| c == '/' || c == '?' || c == '#').unwrap_or(after_scheme.len());
            let host = &after_scheme[..host_end].to_lowercase();
            let host = host.strip_prefix("www.").unwrap_or(host);
            let host = host.strip_prefix("m.").unwrap_or(host);

            let path_and_query = &after_scheme[host_end.min(after_scheme.len())..];
            let (path, query) = match path_and_query.find('?') {
                Some(qi) => (&path_and_query[..qi], &path_and_query[qi + 1..]),
                None => (path_and_query, ""),
            };

            if host == "youtu.be" {
                let id = path.trim_start_matches('/').split('/').next().unwrap_or("");
                if id.len() == 11 && id.chars().all(id_chars) {
                    return Some(id.to_string());
                }
            }

            if host.ends_with("youtube.com") || host.ends_with("youtube-nocookie.com") {
                for pair in query.split('&') {
                    if let Some(eq) = pair.find('=') {
                        let key = &pair[..eq];
                        let value = &pair[eq + 1..];
                        if key == "v" && value.len() == 11 && value.chars().all(id_chars) {
                            return Some(value.to_string());
                        }
                    }
                }

                let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
                let keyword_idx = parts.iter().position(|p| {
                    matches!(*p, "embed" | "v" | "shorts" | "live" | "watch")
                });
                if let Some(idx) = keyword_idx {
                    if let Some(id) = parts.get(idx + 1) {
                        if id.len() == 11 && id.chars().all(id_chars) {
                            return Some((*id).to_string());
                        }
                    }
                }
            }
        }
    }

    let patterns = [
        "youtube.com/watch?v=",
        "youtu.be/",
        "youtube.com/embed/",
        "youtube.com/v/",
        "youtube.com/shorts/",
        "youtube.com/live/",
    ];

    for pattern in patterns {
        if let Some(pos) = input.find(pattern) {
            let start = pos + pattern.len();
            let end = start + 11.min(input.len().saturating_sub(start));
            let id = &input[start..end];
            if id.len() == 11 && id.chars().all(id_chars) {
                return Some(id.to_string());
            }
        }
    }

    None
}

#[tauri::command]
pub fn import_from_yt_renamer(
    state: State<DbState>,
    file_content: String,
    youtube_url: String,
) -> Result<i32, String> {
    let video_id = extract_video_id(&youtube_url)
        .ok_or("Invalid YouTube URL. Please provide a valid YouTube video URL or ID.")?;

    let data: YtRenamerData = serde_json::from_str(&file_content).map_err(|e| {
        format!(
            "Failed to parse JSON: {}. Please provide the ytRenamer export format.",
            e
        )
    })?;

    let mut conn = state.conn.lock().unwrap();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let now = current_time();

    tx.execute(
        "INSERT OR IGNORE INTO videos (id, title, thumbnail_url, duration, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![video_id.clone(), format!("Video {}", &video_id[..8.min(video_id.len())]), format!("https://img.youtube.com/vi/{}/0.jpg", video_id), 0, now, now],
    ).map_err(|e| e.to_string())?;

    let mut sort_order = 0;
    for yt_clip in data.clips {
        let start_time = yt_clip.start_time as i32;
        let end_time = yt_clip.end_time as i32;

        tx.execute(
            "INSERT INTO clips (video_id, start_time, end_time, title, created_at, updated_at, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![video_id, start_time, end_time, yt_clip.title, now, now, sort_order],
        ).map_err(|e| e.to_string())?;
        sort_order += 1;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(sort_order)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VideoOembed {
    pub video_id: String,
    pub title: String,
    pub author: String,
    pub thumbnail_url: String,
}

pub async fn fetch_video_oembed_inner(video_id: &str) -> Result<Option<VideoOembed>, String> {
    let url = format!(
        "https://www.youtube.com/oembed?url=https%3A%2F%2Fwww.youtube.com%2Fwatch%3Fv%3D{}&format=json",
        video_id
    );
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("yClippy/1.0")
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Ok(None);
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(Some(VideoOembed {
        video_id: video_id.to_string(),
        title: json["title"].as_str().unwrap_or("Untitled").to_string(),
        author: json["author_name"].as_str().unwrap_or("").to_string(),
        thumbnail_url: json["thumbnail_url"].as_str().unwrap_or("").to_string(),
    }))
}

pub fn extract_video_id_for_pub(input: &str) -> Option<String> {
    extract_video_id(input)
}

#[tauri::command]
pub async fn fetch_video_oembed(video_id: String) -> Result<Option<VideoOembed>, String> {
    fetch_video_oembed_inner(&video_id).await
}
