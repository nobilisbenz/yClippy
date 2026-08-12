use rusqlite::{params, Connection, OptionalExtension, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct YtRenamerData {
    pub clips: Vec<YtRenamerClip>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct YtRenamerClip {
    pub start_time: f64,
    pub end_time: f64,
    pub title: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub enum ChangeType {
    Create,
    Update,
    Delete,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct Change {
    pub id: i64,
    pub entity_type: String,
    pub entity_id: String,
    pub change_type: ChangeType,
    pub data: String,
    pub timestamp: i64,
    pub device_id: String,
    pub synced: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct SyncMetadata {
    pub last_sync_timestamp: i64,
    pub last_sync_device: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct ChangeSet {
    pub changes: Vec<Change>,
    pub metadata: SyncMetadata,
    pub device_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct AppConfig {
    db_path: Option<String>,
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
        serde_json::from_str(&content).unwrap_or(AppConfig { db_path: None })
    } else {
        AppConfig { db_path: None }
    }
}

fn save_config(app: &AppHandle, config: &AppConfig) {
    let config_path = get_config_path(app);
    let content = serde_json::to_string_pretty(config).expect("failed to serialize config");
    fs::write(config_path, content).expect("failed to write config");
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

    // Migration for existing clips
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
            name TEXT NOT NULL,
            created_at INTEGER,
            updated_at INTEGER DEFAULT 0,
            deleted_at INTEGER,
            sort_order INTEGER DEFAULT 0,
            parent_id INTEGER
        )",
        [],
    )?;

    // Migration for existing folders
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

    // Deprecated but kept for compatibility during migration if needed
    conn.execute(
        "CREATE TABLE IF NOT EXISTS changes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            entity_type TEXT NOT NULL,
            entity_id TEXT NOT NULL,
            change_type TEXT NOT NULL,
            data TEXT,
            timestamp INTEGER NOT NULL,
            device_id TEXT NOT NULL,
            synced INTEGER DEFAULT 0
        )",
        [],
    )?;

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

    let device_id = uuid::Uuid::new_v4().to_string();

    let _: Result<_> = conn.execute(
        "INSERT OR IGNORE INTO sync_metadata (key, value) VALUES ('device_id', ?1)",
        params![device_id],
    );

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
    // 1. Verify path is valid/accessible
    let new_path = PathBuf::from(&path);
    // If directory doesn't exist, maybe create it? For now assume valid path selection.

    // 2. Open new connection
    let conn = Connection::open(&new_path).map_err(|e| e.to_string())?;

    // 3. Run migrations
    setup_db(&conn).map_err(|e| e.to_string())?;

    // 4. Update Global State (Mutex)
    {
        let mut global_conn = state.conn.lock().unwrap();
        *global_conn = conn;
    }

    // 5. Save Config
    let config = AppConfig {
        db_path: Some(path),
    };
    save_config(&app, &config);

    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Folder {
    pub id: Option<i64>,
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
        .prepare("SELECT id, name, created_at, updated_at, deleted_at, sort_order, parent_id FROM folders WHERE deleted_at IS NULL ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;

    let iter = stmt
        .query_map([], |row| {
            Ok(Folder {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
                deleted_at: row.get(4)?,
                sort_order: row.get(5)?,
                parent_id: row.get(6)?,
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
pub fn save_folder(state: State<DbState>, folder: Folder) -> Result<i64, String> {
    let conn = state.conn.lock().unwrap();
    let now = current_time();

    if let Some(_id) = folder.id {
        // Record change commented out for now as we transition to new sync engine
        // match record_change(&conn, "folder", &id.to_string(), ChangeType::Update) { ... }
    }

    conn.execute(
        "INSERT OR REPLACE INTO folders (id, name, created_at, updated_at, deleted_at, sort_order, parent_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![folder.id, folder.name, folder.created_at, now, folder.deleted_at, folder.sort_order, folder.parent_id],
    )
    .map_err(|e| e.to_string())?;

    if let Some(id) = folder.id {
        Ok(id)
    } else {
        Ok(conn.last_insert_rowid())
    }
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
pub fn delete_folder(state: State<DbState>, id: i64) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let now = current_time();

    // Soft delete: set deleted_at instead of removing
    conn.execute(
        "UPDATE folders SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1",
        params![id, now],
    )
    .map_err(|e| e.to_string())?;

    // Also soft delete items in this folder or move them?
    // Plan says "Apply soft deletes (remove items where deleted_at is set)" -> wait, logic says "Update local: apply soft deletes".
    // Usually if you delete a folder, you might want to soft delete contents too.
    // However, existing logic just unparented children (moved to root).
    // "UPDATE videos SET folder_id = ?2 WHERE folder_id = ?1" -> parent_id
    // Previous logic was: move children to grandparent.

    // For soft delete, maybe we should keep structure?
    // If we mark folder as deleted, we can't see it, but we can see its children if they are not deleted?
    // Let's stick to previous logic of re-parenting for now to avoid orphaned invisible items,
    // OR we soft-delete everything inside.

    // Previous logic:
    // let parent_id: Option<i64> = conn.query_row(...).unwrap_or(None);
    // conn.execute("UPDATE videos SET folder_id = ?2 ...", ...)?;
    // conn.execute("UPDATE folders SET parent_id = ?2 ...", ...)?;
    // conn.execute("DELETE FROM folders ...")?;

    // New logic: Soft delete the folder.
    // Ideally we should soft-delete children too?
    // Or move them to root? Moving to root/grandparent is safer for user data retention.

    let parent_id: Option<i64> = conn
        .query_row(
            "SELECT parent_id FROM folders WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()
        .unwrap_or(None);

    // Update children to point to grandparent (or root) and update their timestamp
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
pub fn save_video(state: State<DbState>, video: Video) -> Result<(), String> {
    let folder_id = if video.folder_id == Some(0) {
        None
    } else {
        video.folder_id
    };
    let conn = state.conn.lock().unwrap();
    let now = current_time();

    // Check if exists for accurate created_at if needed, but we trust input video struct
    // Actually, we should probably preserve created_at from DB if it exists and input is 0?
    // But usually frontend sends correct full object.

    conn.execute(
        "INSERT OR REPLACE INTO videos (id, title, thumbnail_url, duration, last_position, created_at, updated_at, deleted_at, folder_id, start_time, end_time, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![video.id, video.title, video.thumbnail_url, video.duration, video.last_position, video.created_at, now, video.deleted_at, folder_id, video.start_time, video.end_time, video.sort_order],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_clips(state: State<DbState>, video_id: String) -> Result<Vec<Clip>, String> {
    let conn = state.conn.lock().unwrap();
    let mut stmt = conn.prepare("SELECT id, video_id, start_time, end_time, title, created_at, updated_at, deleted_at, sort_order FROM clips WHERE video_id = ?1 AND deleted_at IS NULL ORDER BY sort_order ASC").map_err(|e| e.to_string())?;

    let clip_iter = stmt
        .query_map(params![video_id], |row| {
            Ok(Clip {
                id: row.get(0)?,
                video_id: row.get(1)?,
                start_time: row.get(2)?,
                end_time: row.get(3)?,
                title: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                deleted_at: row.get(7)?,
                sort_order: row.get(8)?,
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
pub fn save_clip(state: State<DbState>, clip: Clip) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let now = current_time();

    if let Some(id) = clip.id {
        conn.execute(
            "UPDATE clips SET video_id = ?1, start_time = ?2, end_time = ?3, title = ?4, updated_at = ?5, sort_order = ?6 WHERE id = ?7",
            params![clip.video_id, clip.start_time, clip.end_time, clip.title, now, clip.sort_order, id],
        ).map_err(|e| e.to_string())?;
    } else {
        let max_order: Option<i32> = conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), -1) FROM clips WHERE video_id = ?1 AND deleted_at IS NULL",
                params![clip.video_id],
                |row| row.get(0),
            )
            .ok();
        let new_sort_order = max_order.unwrap_or(-1) + 1;
        conn.execute(
            "INSERT INTO clips (video_id, start_time, end_time, title, created_at, updated_at, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?6)",
            params![clip.video_id, clip.start_time, clip.end_time, clip.title, clip.created_at, new_sort_order],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn delete_clip(state: State<DbState>, id: i32) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let now = current_time();
    conn.execute(
        "UPDATE clips SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1",
        params![id, now],
    )
    .map_err(|e| e.to_string())?;
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
pub fn delete_video(state: State<DbState>, id: String) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    let now = current_time();

    // Soft delete video
    conn.execute(
        "UPDATE videos SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1",
        params![id, now],
    )
    .map_err(|e| e.to_string())?;

    // Soft delete associated clips?
    // Yes, usually.
    conn.execute(
        "UPDATE clips SET deleted_at = ?2, updated_at = ?2 WHERE video_id = ?1",
        params![id, now],
    )
    .map_err(|e| e.to_string())?;

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
    // Return ALL items including deleted for sync purposes?
    // Existing export_db was for "backup.json" which was monolithic.
    // If we replace this with sync engine, we might not need this exact function exposed to frontend anymore,
    // OR we update it to return everything so sync engine can filter?
    // But sync engine will probably query DB directly.

    // For now, let's keep this compatible with "Backup" feature if it exists,
    // but maybe we should only export non-deleted items for a manual "Export Backup" file?

    let conn = state.conn.lock().unwrap();

    // Folders
    let mut stmt = conn
        .prepare("SELECT id, name, created_at, updated_at, deleted_at, sort_order, parent_id FROM folders WHERE deleted_at IS NULL ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;
    let folder_iter = stmt
        .query_map([], |row| {
            Ok(Folder {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
                deleted_at: row.get(4)?,
                sort_order: row.get(5)?,
                parent_id: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut folders = Vec::new();
    for f in folder_iter {
        folders.push(f.map_err(|e| e.to_string())?);
    }

    // Videos
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

    // Clips
    let mut stmt = conn
        .prepare("SELECT id, video_id, start_time, end_time, title, created_at, updated_at, deleted_at, sort_order FROM clips WHERE deleted_at IS NULL")
        .map_err(|e| e.to_string())?;
    let clip_iter = stmt
        .query_map([], |row| {
            Ok(Clip {
                id: row.get(0)?,
                video_id: row.get(1)?,
                start_time: row.get(2)?,
                end_time: row.get(3)?,
                title: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                deleted_at: row.get(7)?,
                sort_order: row.get(8)?,
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
    // This function is for manual import of legacy backups or such.
    // We should update it to respect new columns, but for now it's less critical than sync engine.
    // Just ensuring it compiles is enough for this step.
    // Users might import old backups which lack new fields, so we use defaults.

    let mut conn = state.conn.lock().unwrap();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let now = current_time();

    // ... (Simplified logic for now, or just keep old logic adapted)

    for folder in backup.folders {
        tx.execute(
            "INSERT OR IGNORE INTO folders (name, created_at, updated_at, sort_order, parent_id) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![folder.name, folder.created_at, now, folder.sort_order, folder.parent_id],
        ).map_err(|e| e.to_string())?;
    }

    for video in backup.videos {
        tx.execute(
             "INSERT OR IGNORE INTO videos (id, title, thumbnail_url, duration, last_position, created_at, updated_at, folder_id, start_time, end_time, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
             params![video.id, video.title, video.thumbnail_url, video.duration, video.last_position, video.created_at, now, video.folder_id, video.start_time, video.end_time, video.sort_order],
        ).map_err(|e| e.to_string())?;
    }

    for clip in backup.clips {
        tx.execute(
            "INSERT OR IGNORE INTO clips (video_id, start_time, end_time, title, created_at, updated_at, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?6)",
            params![clip.video_id, clip.start_time, clip.end_time, clip.title, clip.created_at, clip.sort_order],
        ).map_err(|e| e.to_string())?;
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

#[tauri::command]
#[allow(dead_code)]
pub fn mark_changes_synced(_state: State<DbState>) -> Result<(), String> {
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
        "SELECT id, name, created_at, updated_at, deleted_at, sort_order, parent_id FROM folders",
    )?;
    let folder_iter = stmt.query_map([], |row| {
        Ok(Folder {
            id: row.get(0)?,
            name: row.get(1)?,
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
            deleted_at: row.get(4)?,
            sort_order: row.get(5)?,
            parent_id: row.get(6)?,
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
    let mut stmt = conn.prepare("SELECT id, video_id, start_time, end_time, title, created_at, updated_at, deleted_at, sort_order FROM clips")?;
    let clip_iter = stmt.query_map([], |row| {
        Ok(Clip {
            id: row.get(0)?,
            video_id: row.get(1)?,
            start_time: row.get(2)?,
            end_time: row.get(3)?,
            title: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
            deleted_at: row.get(7)?,
            sort_order: row.get(8)?,
        })
    })?;
    let mut clips = Vec::new();
    for c in clip_iter {
        clips.push(c?);
    }
    Ok(clips)
}

pub fn upsert_folder_internal(conn: &Connection, folder: &Folder) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR REPLACE INTO folders (id, name, created_at, updated_at, deleted_at, sort_order, parent_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![folder.id, folder.name, folder.created_at, folder.updated_at, folder.deleted_at, folder.sort_order, folder.parent_id],
    )?;
    Ok(())
}

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

pub fn upsert_clip_internal(conn: &Connection, clip: &Clip) -> Result<(), rusqlite::Error> {
    if let Some(id) = clip.id {
        conn.execute(
            "INSERT OR REPLACE INTO clips (id, video_id, start_time, end_time, title, created_at, updated_at, deleted_at, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
             params![id, clip.video_id, clip.start_time, clip.end_time, clip.title, clip.created_at, clip.updated_at, clip.deleted_at, clip.sort_order],
        )?;
    } else {
        conn.execute(
            "INSERT INTO clips (video_id, start_time, end_time, title, created_at, updated_at, deleted_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
             params![clip.video_id, clip.start_time, clip.end_time, clip.title, clip.created_at, clip.updated_at, clip.deleted_at],
        )?;
    }
    Ok(())
}

fn extract_video_id(url: &str) -> Option<String> {
    if url.len() == 11 {
        return Some(url.to_string());
    }

    let patterns = [
        "youtube.com/watch?v=",
        "youtu.be/",
        "youtube.com/embed/",
        "youtube.com/v/",
        "youtube.com/shorts/",
    ];

    for pattern in patterns {
        if let Some(pos) = url.find(pattern) {
            let start = pos + pattern.len();
            let end = start + 11.min(url.len() - start);
            let id = &url[start..end];
            if id.len() == 11 {
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

#[tauri::command]
pub async fn fetch_video_oembed(video_id: String) -> Result<Option<VideoOembed>, String> {
    let url = format!(
        "https://www.youtube.com/oembed?url=https://www.youtube.com/watch?v={}&format=json",
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
        video_id,
        title: json["title"].as_str().unwrap_or("Untitled").to_string(),
        author: json["author_name"].as_str().unwrap_or("").to_string(),
        thumbnail_url: json["thumbnail_url"].as_str().unwrap_or("").to_string(),
    }))
}
