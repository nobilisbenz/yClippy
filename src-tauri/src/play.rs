use crate::db;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

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
}

pub fn parse_play_args(args: &[String]) -> Option<PlayRequest> {
    if args.len() < 2 {
        return None;
    }
    if args[1] != "play" && args[1] != "add" {
        return None;
    }

    let _subcmd = &args[1];
    let mut at_seconds: Option<f64> = None;
    let mut end_seconds: Option<f64> = None;
    let mut folder: Option<String> = None;
    let mut title: Option<String> = None;

    let mut positional: Vec<String> = Vec::new();

    let mut i = 2;
    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "--at" if i + 1 < args.len() => {
                at_seconds = args[i + 1].parse().ok();
                i += 2;
            }
            "--end" if i + 1 < args.len() => {
                end_seconds = args[i + 1].parse().ok();
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
            _ => {
                positional.push(a.clone());
                i += 1;
            }
        }
    }

    let raw_url = positional.first().cloned()?;

    let video_id = extract_video_id_pub(&raw_url)?;

    Some(PlayRequest {
        url: raw_url,
        video_id,
        at_seconds,
        end_seconds,
        folder,
        title,
    })
}

fn extract_video_id_pub(input: &str) -> Option<String> {
    db::extract_video_id_for_pub(input)
}

pub async fn process_play_request(app: &AppHandle, req: PlayRequest) -> Result<(), String> {
    let at = req.at_seconds.unwrap_or(0.0).max(0.0);
    let end = req.end_seconds.unwrap_or(0.0).max(0.0);

    let existing_title: Option<String> = {
        let state: tauri::State<db::DbState> = app.state();
        let conn = state.conn.lock().unwrap();
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

    if needs_title {
        if let Ok(Some(meta)) = db::fetch_video_oembed_inner(&req.video_id).await {
            let state: tauri::State<db::DbState> = app.state();
            let conn = state.conn.lock().unwrap();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;

            if existing_title.is_none() {
                let _ = conn.execute(
                    "INSERT OR IGNORE INTO videos (id, title, thumbnail_url, duration, last_position, created_at, updated_at, start_time, end_time, sort_order, folder_id) VALUES (?1, ?2, ?3, 0, 0, ?4, ?4, ?5, ?6, 0, NULL)",
                    rusqlite::params![req.video_id, meta.title, meta.thumbnail_url, now, at as i32, end as i32],
                );
            } else {
                let _ = conn.execute(
                    "UPDATE videos SET title = COALESCE(NULLIF(?1, ''), title), thumbnail_url = COALESCE(NULLIF(?2, ''), thumbnail_url), updated_at = ?3 WHERE id = ?4",
                    rusqlite::params![meta.title, meta.thumbnail_url, now, req.video_id],
                );
            }
        }
    }

    let payload = serde_json::json!({
        "url": req.url,
        "video_id": req.video_id,
        "at_seconds": at,
        "end_seconds": end,
        "folder": req.folder,
        "title": req.title,
    });

    let app_clone = app.clone();
    let payload_clone = payload.clone();
    let _ = app.run_on_main_thread(move || {
        let _ = app_clone.emit("yclippy://play", payload_clone);
    });

    for _ in 0..40 {
        let app_clone = app.clone();
        let (tx, rx) = std::sync::mpsc::channel::<bool>();
        let _ = app.run_on_main_thread(move || {
            let focused = if let Some(win) = app_clone.get_webview_window("main") {
                #[cfg(not(any(target_os = "android", target_os = "ios")))]
                {
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

    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct PickerItem {
    pub video_id: String,
    pub title: String,
    pub thumbnail_url: String,
    pub last_position: i32,
    pub start_time: i32,
    pub end_time: i32,
    pub clip_count: i32,
}

#[tauri::command]
pub async fn list_videos_for_picker(
    _app: AppHandle,
    state: tauri::State<'_, db::DbState>,
    query: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<PickerItem>, String> {
    let conn = state.conn.lock().unwrap();
    let lim = limit.unwrap_or(50).clamp(1, 500);
    let q = query.unwrap_or_default().to_lowercase();

    let mut sql = String::from(
        "SELECT v.id, v.title, v.thumbnail_url, v.last_position, v.start_time, v.end_time,
         (SELECT COUNT(*) FROM clips c WHERE c.video_id = v.id AND c.deleted_at IS NULL) as clip_count
         FROM videos v WHERE v.deleted_at IS NULL",
    );
    if !q.is_empty() {
        sql.push_str(" AND LOWER(v.title) LIKE ?1");
    }
    sql.push_str(" ORDER BY v.created_at DESC LIMIT ?2");

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let map_row = |row: &rusqlite::Row<'_>| -> rusqlite::Result<PickerItem> {
        Ok(PickerItem {
            video_id: row.get(0)?,
            title: row.get(1)?,
            thumbnail_url: row.get(2)?,
            last_position: row.get(3)?,
            start_time: row.get(4)?,
            end_time: row.get(5)?,
            clip_count: row.get(6)?,
        })
    };

    let items: Vec<PickerItem> = if q.is_empty() {
        stmt.query_map(rusqlite::params![lim], map_row)
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect()
    } else {
        let pat = format!("%{q}%");
        stmt.query_map(rusqlite::params![pat, lim], map_row)
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect()
    };

    Ok(items)
}