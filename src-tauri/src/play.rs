use crate::db;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// The app identifier, mirrored from tauri.conf.json. Used by the headless CLI
/// paths, which resolve the database before Tauri exists to ask.
const APP_IDENTIFIER: &str = "com.yclippy.app";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayRequest {
    pub url: String,
    pub video_id: String,
    #[serde(default)]
    pub at_seconds: Option<f64>,
    #[serde(default)]
    pub end_seconds: Option<f64>,
    #[serde(default)]
    pub folder: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    /// `add` imports without stealing focus; `play` opens the player.
    #[serde(default)]
    pub open: bool,
}

#[derive(Debug, Clone)]
pub enum Command {
    Play(PlayRequest),
    List { query: Option<String>, limit: i64 },
}

/// Holds a request that arrived before the webview had a listener. The frontend
/// drains it with `take_pending_play` on mount; without this the `setup()` emit
/// races the webview and a cold-start play is silently dropped.
#[derive(Default)]
pub struct PendingPlay(pub Mutex<Option<PlayRequest>>);

/// Accepts every timestamp shape the vault's `@video` line can carry:
/// `414`, `6:54`, `01:06:54`, `6m54s`, `90s`.
pub fn parse_timestamp(raw: &str) -> Option<f64> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    if let Ok(n) = raw.parse::<f64>() {
        return Some(n.max(0.0));
    }

    if raw.contains(':') {
        let mut total = 0f64;
        for part in raw.split(':') {
            let n: f64 = part.trim().parse().ok()?;
            total = total * 60.0 + n;
        }
        return Some(total.max(0.0));
    }

    // 1h2m3s / 6m54s / 90s
    let mut total = 0f64;
    let mut current = String::new();
    let mut saw_unit = false;
    for ch in raw.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            current.push(ch);
            continue;
        }
        let n: f64 = current.parse().ok()?;
        current.clear();
        total += match ch.to_ascii_lowercase() {
            'h' => n * 3600.0,
            'm' => n * 60.0,
            's' => n,
            _ => return None,
        };
        saw_unit = true;
    }
    if !current.is_empty() || !saw_unit {
        return None;
    }
    Some(total.max(0.0))
}

pub fn parse_play_args(args: &[String]) -> Option<Command> {
    if args.len() < 2 {
        return None;
    }

    let subcmd = args[1].as_str();
    if !matches!(subcmd, "play" | "add" | "list") {
        return None;
    }

    let mut at_seconds: Option<f64> = None;
    let mut end_seconds: Option<f64> = None;
    let mut folder: Option<String> = None;
    let mut title: Option<String> = None;
    let mut query: Option<String> = None;
    let mut limit: i64 = 200;
    let mut positional: Vec<String> = Vec::new();

    let mut i = 2;
    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "--at" if i + 1 < args.len() => {
                at_seconds = parse_timestamp(&args[i + 1]);
                i += 2;
            }
            "--end" if i + 1 < args.len() => {
                end_seconds = parse_timestamp(&args[i + 1]);
                i += 2;
            }
            "--folder" if i + 1 < args.len() => {
                folder = Some(args[i + 1].clone());
                i += 2;
            }
            "--title" if i + 1 < args.len() => {
                title = Some(args[i + 1].clone());
                i += 2;
            }
            "--query" | "-q" if i + 1 < args.len() => {
                query = Some(args[i + 1].clone());
                i += 2;
            }
            "--limit" if i + 1 < args.len() => {
                limit = args[i + 1].parse().unwrap_or(200);
                i += 2;
            }
            // Accepted and ignored: `list` is always JSON.
            "--json" => i += 1,
            _ => {
                positional.push(a.clone());
                i += 1;
            }
        }
    }

    if subcmd == "list" {
        if query.is_none() {
            query = positional.first().cloned();
        }
        return Some(Command::List {
            query,
            limit: limit.clamp(1, 1000),
        });
    }

    let raw_url = positional.first().cloned()?;
    let video_id = db::extract_video_id_for_pub(&raw_url)?;

    Some(Command::Play(PlayRequest {
        url: raw_url,
        video_id,
        at_seconds,
        end_seconds,
        folder,
        title,
        open: subcmd == "play",
    }))
}

/// Tauri's `app_data_dir` rules, reproduced for the headless subcommands that
/// run before `tauri::Builder`. Kept beside `db::init_db`'s own resolution —
/// if one changes, change both.
fn headless_app_data_dir() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "linux")]
    {
        dirs::data_dir().map(|d| d.join(APP_IDENTIFIER))
    }
    #[cfg(target_os = "macos")]
    {
        dirs::data_dir().map(|d| d.join(APP_IDENTIFIER))
    }
    #[cfg(target_os = "windows")]
    {
        dirs::data_dir().map(|d| d.join(APP_IDENTIFIER))
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        None
    }
}

fn headless_db_path() -> Option<std::path::PathBuf> {
    let dir = headless_app_data_dir()?;
    let config_path = dir.join("config.json");
    if config_path.exists() {
        if let Ok(text) = std::fs::read_to_string(&config_path) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(path) = value.get("db_path").and_then(|v| v.as_str()) {
                    if !path.is_empty() {
                        return Some(std::path::PathBuf::from(path));
                    }
                }
            }
        }
    }
    Some(dir.join("yclippy.db"))
}

/// `yclippy list [--query Q] [--limit N]` — prints JSON and exits without ever
/// starting the GUI, so pickers in Neovim and the TUI can shell out to it.
pub fn run_list(query: Option<String>, limit: i64) -> Result<String, String> {
    let path = headless_db_path().ok_or_else(|| "could not locate the yClippy database".to_string())?;
    if !path.exists() {
        return serde_json::to_string_pretty(&serde_json::json!({
            "protocol_version": 1,
            "items": [],
        }))
        .map_err(|e| e.to_string());
    }

    let conn = rusqlite::Connection::open_with_flags(
        &path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|e| format!("could not open {}: {e}", path.display()))?;

    let items = query_picker_items(&conn, query.as_deref(), limit).map_err(|e| e.to_string())?;

    serde_json::to_string_pretty(&serde_json::json!({
        "protocol_version": 1,
        "items": items,
    }))
    .map_err(|e| e.to_string())
}

/// One row per playable target — a whole video, or a named clip inside one.
/// Clips carry their own start/end so a picker can hand them straight back to
/// `yclippy play --at … --end …`.
#[derive(Debug, Clone, Serialize)]
pub struct PickerItem {
    pub kind: &'static str,
    pub video_id: String,
    pub url: String,
    pub title: String,
    pub thumbnail_url: String,
    pub start_seconds: i32,
    pub end_seconds: i32,
    pub last_position: i32,
    pub clip_uid: Option<String>,
    pub clip_count: i32,
}

fn query_picker_items(
    conn: &rusqlite::Connection,
    query: Option<&str>,
    limit: i64,
) -> rusqlite::Result<Vec<PickerItem>> {
    let needle = query.unwrap_or_default().to_lowercase();
    let pattern = format!("%{needle}%");
    let filtered = !needle.is_empty();

    let mut items: Vec<PickerItem> = Vec::new();

    let video_sql = if filtered {
        "SELECT v.id, v.title, v.thumbnail_url, v.last_position, v.start_time, v.end_time,
          (SELECT COUNT(*) FROM clips c WHERE c.video_id = v.id AND c.deleted_at IS NULL)
         FROM videos v
         WHERE v.deleted_at IS NULL AND LOWER(v.title) LIKE ?1
         ORDER BY v.updated_at DESC LIMIT ?2"
    } else {
        "SELECT v.id, v.title, v.thumbnail_url, v.last_position, v.start_time, v.end_time,
          (SELECT COUNT(*) FROM clips c WHERE c.video_id = v.id AND c.deleted_at IS NULL)
         FROM videos v
         WHERE v.deleted_at IS NULL
         ORDER BY v.updated_at DESC LIMIT ?2"
    };

    let mut stmt = conn.prepare(video_sql)?;
    let rows = stmt.query_map(rusqlite::params![pattern, limit], |row| {
        let video_id: String = row.get(0)?;
        Ok(PickerItem {
            kind: "video",
            url: watch_url(&video_id, row.get::<_, i32>(4)?),
            video_id,
            title: row.get(1)?,
            thumbnail_url: row.get(2)?,
            last_position: row.get(3)?,
            start_seconds: row.get(4)?,
            end_seconds: row.get(5)?,
            clip_uid: None,
            clip_count: row.get(6)?,
        })
    })?;
    for row in rows {
        items.push(row?);
    }

    let clip_sql = if filtered {
        "SELECT c.uid, c.video_id, c.title, c.start_time, c.end_time, v.title
         FROM clips c JOIN videos v ON v.id = c.video_id
         WHERE c.deleted_at IS NULL AND v.deleted_at IS NULL
           AND (LOWER(c.title) LIKE ?1 OR LOWER(v.title) LIKE ?1)
         ORDER BY c.updated_at DESC LIMIT ?2"
    } else {
        "SELECT c.uid, c.video_id, c.title, c.start_time, c.end_time, v.title
         FROM clips c JOIN videos v ON v.id = c.video_id
         WHERE c.deleted_at IS NULL AND v.deleted_at IS NULL
         ORDER BY c.updated_at DESC LIMIT ?2"
    };

    let mut stmt = conn.prepare(clip_sql)?;
    let rows = stmt.query_map(rusqlite::params![pattern, limit], |row| {
        let video_id: String = row.get(1)?;
        let clip_title: String = row.get(2)?;
        let video_title: String = row.get(5)?;
        let start: i32 = row.get(3)?;
        Ok(PickerItem {
            kind: "clip",
            url: watch_url(&video_id, start),
            video_id,
            title: if clip_title.trim().is_empty() {
                video_title
            } else {
                format!("{clip_title} — {video_title}")
            },
            thumbnail_url: String::new(),
            last_position: 0,
            start_seconds: start,
            end_seconds: row.get(4)?,
            clip_uid: row.get(0)?,
            clip_count: 0,
        })
    })?;
    for row in rows {
        items.push(row?);
    }

    Ok(items)
}

fn watch_url(video_id: &str, start: i32) -> String {
    if start > 0 {
        format!("https://www.youtube.com/watch?v={video_id}&t={start}s")
    } else {
        format!("https://www.youtube.com/watch?v={video_id}")
    }
}

pub async fn process_play_request(app: &AppHandle, req: PlayRequest) -> Result<(), String> {
    let at = req.at_seconds.unwrap_or(0.0).max(0.0);

    let existing_title: Option<String> = {
        let state: tauri::State<db::DbState> = app.state();
        let conn = state.conn.lock().map_err(|_| "database lock poisoned")?;
        conn.query_row(
            "SELECT title FROM videos WHERE id = ?1 AND deleted_at IS NULL",
            rusqlite::params![req.video_id],
            |row| row.get(0),
        )
        .ok()
    };

    let needs_title = match &existing_title {
        Some(t) => t.is_empty() || t.starts_with("Video "),
        None => true,
    };

    let folder_id = match req.folder.as_deref() {
        Some(name) if !name.trim().is_empty() => resolve_folder(app, name)?,
        _ => None,
    };

    if needs_title || existing_title.is_none() {
        let meta = db::fetch_video_oembed_inner(&req.video_id).await.ok().flatten();
        let state: tauri::State<db::DbState> = app.state();
        let conn = state.conn.lock().map_err(|_| "database lock poisoned")?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        let title = req
            .title
            .clone()
            .or_else(|| meta.as_ref().map(|m| m.title.clone()))
            .unwrap_or_else(|| format!("Video {}", req.video_id));
        let thumb = meta.as_ref().map(|m| m.thumbnail_url.clone()).unwrap_or_default();

        if existing_title.is_none() {
            // start_time/end_time are the video's *trim*, not a seek position.
            // `--at` must never write them, or playing a link at 6:54 silently
            // truncates that video everywhere, forever.
            let _ = conn.execute(
                "INSERT OR IGNORE INTO videos (id, title, thumbnail_url, duration, last_position, created_at, updated_at, start_time, end_time, sort_order, folder_id) \
                 VALUES (?1, ?2, ?3, 0, 0, ?4, ?4, 0, 0, 0, ?5)",
                rusqlite::params![req.video_id, title, thumb, now, folder_id],
            );
        } else {
            let _ = conn.execute(
                "UPDATE videos SET title = COALESCE(NULLIF(?1, ''), title), \
                 thumbnail_url = COALESCE(NULLIF(?2, ''), thumbnail_url), updated_at = ?3 WHERE id = ?4",
                rusqlite::params![title, thumb, now, req.video_id],
            );
        }
    } else if let Some(fid) = folder_id {
        let state: tauri::State<db::DbState> = app.state();
        let conn = state.conn.lock().map_err(|_| "database lock poisoned")?;
        let _ = conn.execute(
            "UPDATE videos SET folder_id = ?1 WHERE id = ?2",
            rusqlite::params![fid, req.video_id],
        );
    }

    if !req.open {
        // `add` imports and stops. Tell the frontend to refresh if it is up.
        let app_clone = app.clone();
        let _ = app.run_on_main_thread(move || {
            let _ = app_clone.emit("yclippy://library-changed", ());
        });
        return Ok(());
    }

    let payload = serde_json::json!({
        "url": req.url,
        "video_id": req.video_id,
        "at_seconds": at,
        "end_seconds": req.end_seconds.unwrap_or(0.0).max(0.0),
        "folder": req.folder,
        "title": req.title,
    });

    // Park it first, then emit. A webview that is already listening handles the
    // event and clears the slot; one that is still booting drains it on mount.
    if let Some(pending) = app.try_state::<PendingPlay>() {
        if let Ok(mut slot) = pending.0.lock() {
            *slot = Some(req.clone());
        }
    }

    let app_clone = app.clone();
    let _ = app.run_on_main_thread(move || {
        let _ = app_clone.emit("yclippy://play", payload);
    });

    focus_main_window(app);

    Ok(())
}

/// Resolves a `--folder` path like `Rust/Ownership`, creating what is missing.
fn resolve_folder(app: &AppHandle, path: &str) -> Result<Option<i64>, String> {
    let state: tauri::State<db::DbState> = app.state();
    let conn = state.conn.lock().map_err(|_| "database lock poisoned")?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let mut parent: Option<i64> = None;
    for segment in path.split('/').map(str::trim).filter(|s| !s.is_empty()) {
        let existing: Option<i64> = match parent {
            Some(p) => conn
                .query_row(
                    "SELECT id FROM folders WHERE name = ?1 AND parent_id = ?2 AND deleted_at IS NULL",
                    rusqlite::params![segment, p],
                    |row| row.get(0),
                )
                .ok(),
            None => conn
                .query_row(
                    "SELECT id FROM folders WHERE name = ?1 AND parent_id IS NULL AND deleted_at IS NULL",
                    rusqlite::params![segment],
                    |row| row.get(0),
                )
                .ok(),
        };

        parent = Some(match existing {
            Some(id) => id,
            None => {
                conn.execute(
                    "INSERT INTO folders (uid, name, created_at, updated_at, sort_order, parent_id) \
                     VALUES (?1, ?2, ?3, ?3, 0, ?4)",
                    rusqlite::params![uuid::Uuid::new_v4().to_string(), segment, now, parent],
                )
                .map_err(|e| e.to_string())?;
                conn.last_insert_rowid()
            }
        });
    }

    Ok(parent)
}

fn focus_main_window(app: &AppHandle) {
    for _ in 0..40 {
        let app_clone = app.clone();
        let (tx, rx) = std::sync::mpsc::channel::<bool>();
        let _ = app.run_on_main_thread(move || {
            let focused = if let Some(win) = app_clone.get_webview_window("main") {
                #[cfg(not(any(target_os = "android", target_os = "ios")))]
                {
                    let _ = win.unminimize();
                    let _ = win.set_focus();
                }
                win.is_focused().unwrap_or(false)
            } else {
                false
            };
            let _ = tx.send(focused);
        });
        if let Ok(true) = rx.recv_timeout(Duration::from_millis(50)) {
            break;
        }
    }
}

/// Drained by the frontend on mount, so a cold-start `yclippy play` is not lost
/// to the gap between `setup()` and the webview registering its listener.
#[tauri::command]
pub fn take_pending_play(state: tauri::State<'_, PendingPlay>) -> Option<PlayRequest> {
    state.0.lock().ok().and_then(|mut slot| slot.take())
}

#[tauri::command]
pub async fn list_videos_for_picker(
    state: tauri::State<'_, db::DbState>,
    query: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<PickerItem>, String> {
    let conn = state.conn.lock().map_err(|_| "database lock poisoned")?;
    query_picker_items(&conn, query.as_deref(), limit.unwrap_or(200).clamp(1, 1000))
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_accept_every_video_line_shape() {
        assert_eq!(parse_timestamp("414"), Some(414.0));
        assert_eq!(parse_timestamp("6:54"), Some(414.0));
        assert_eq!(parse_timestamp("01:06:54"), Some(4014.0));
        assert_eq!(parse_timestamp("6m54s"), Some(414.0));
        assert_eq!(parse_timestamp("90s"), Some(90.0));
        assert_eq!(parse_timestamp("1h2m3s"), Some(3723.0));
        assert_eq!(parse_timestamp(""), None);
        assert_eq!(parse_timestamp("chapter-three"), None);
    }

    #[test]
    fn play_opens_and_add_does_not() {
        let args = |s: &str| -> Vec<String> {
            s.split_whitespace().map(str::to_string).collect()
        };

        let cmd = parse_play_args(&args("yclippy play https://youtu.be/dQw4w9WgXcQ --at 6:54"));
        match cmd {
            Some(Command::Play(req)) => {
                assert_eq!(req.video_id, "dQw4w9WgXcQ");
                assert_eq!(req.at_seconds, Some(414.0));
                assert!(req.open);
            }
            other => panic!("expected a play request, got {other:?}"),
        }

        let cmd = parse_play_args(&args("yclippy add https://youtu.be/dQw4w9WgXcQ"));
        match cmd {
            Some(Command::Play(req)) => assert!(!req.open),
            other => panic!("expected an add request, got {other:?}"),
        }
    }

    #[test]
    fn list_is_recognised_and_takes_a_query() {
        let args: Vec<String> = "yclippy list --json --query borrow"
            .split_whitespace()
            .map(str::to_string)
            .collect();
        match parse_play_args(&args) {
            Some(Command::List { query, limit }) => {
                assert_eq!(query.as_deref(), Some("borrow"));
                assert_eq!(limit, 200);
            }
            other => panic!("expected a list command, got {other:?}"),
        }
    }

    #[test]
    fn unknown_subcommands_fall_through_to_the_gui() {
        let args: Vec<String> = vec!["yclippy".into()];
        assert!(parse_play_args(&args).is_none());
        let args: Vec<String> = vec!["yclippy".into(), "--help".into()];
        assert!(parse_play_args(&args).is_none());
    }
}
