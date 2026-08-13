//! The bridge between SQLite and the sync wire format.
//!
//! There is deliberately no local log file. What a device needs to publish is
//! *derivable* from its own rows: every record it last wrote, since the last
//! time compaction folded its work into `library.json`. Keeping a separate log
//! only creates a second copy that can disagree with the database — which is
//! how the previous design managed to write ops, never send them, and then
//! delete them.
//!
//! So each row carries `last_writer`, stamped whenever this device changes it.
//! "My outgoing log" is a query. It cannot drift, and replaying it is harmless
//! because the merge is idempotent.

use crate::db::DbState;
use crate::wire::{Library, Op, Record, Snapshot, WireClip, WireFolder, WireVideo};
use rusqlite::{params, Connection};
use std::collections::BTreeMap;
use tauri::{AppHandle, Manager};

pub fn current_time() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Mark a record as last written by this device.
///
/// Called at the points that already know a local change happened. The caller
/// must have released the connection lock; `DbState.conn` is a plain mutex and
/// re-entering it would deadlock.
pub fn stamp(app: &AppHandle, entity: &str, uid: &str) -> Result<(), String> {
    let state: tauri::State<DbState> = app.state();
    let device = state.device_id.clone();
    let conn = state.conn.lock().map_err(|_| "database lock poisoned")?;
    stamp_with(&conn, &device, entity, uid)
}

pub fn stamp_with(
    conn: &Connection,
    device: &str,
    entity: &str,
    uid: &str,
) -> Result<(), String> {
    let sql = match entity {
        "folder" => "UPDATE folders SET last_writer = ?1 WHERE uid = ?2",
        "clip" => "UPDATE clips SET last_writer = ?1 WHERE uid = ?2",
        "video" => "UPDATE videos SET last_writer = ?1 WHERE id = ?2",
        _ => return Ok(()),
    };
    conn.execute(sql, params![device, uid])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

// ── reading ────────────────────────────────────────────────────────────────

/// The whole local library, with rowid links resolved to uids.
pub fn read_snapshot(conn: &Connection) -> Result<Snapshot, String> {
    let mut snapshot = Snapshot::default();

    // uid by rowid, so folder links can be rewritten as we go.
    let mut folder_uid: BTreeMap<i64, String> = BTreeMap::new();
    {
        let mut stmt = conn
            .prepare("SELECT id, uid FROM folders WHERE uid IS NOT NULL AND uid != ''")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (id, uid) = row.map_err(|e| e.to_string())?;
            folder_uid.insert(id, uid);
        }
    }

    {
        let mut stmt = conn
            .prepare(
                "SELECT uid, name, created_at, updated_at, deleted_at, sort_order, parent_id, \
                 COALESCE(last_writer, '') FROM folders WHERE uid IS NOT NULL AND uid != ''",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                let parent_id: Option<i64> = row.get(6)?;
                Ok(WireFolder {
                    uid: row.get(0)?,
                    name: row.get(1)?,
                    created_at: row.get::<_, Option<i64>>(2)?.unwrap_or(0),
                    updated_at: row.get::<_, Option<i64>>(3)?.unwrap_or(0),
                    deleted_at: row.get(4)?,
                    sort_order: row.get::<_, Option<i64>>(5)?.unwrap_or(0),
                    parent_uid: parent_id.and_then(|id| folder_uid.get(&id).cloned()),
                    last_writer: row.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            let folder = row.map_err(|e| e.to_string())?;
            snapshot.folders.insert(folder.uid.clone(), folder);
        }
    }

    {
        let mut stmt = conn
            .prepare(
                "SELECT id, title, COALESCE(thumbnail_url, ''), COALESCE(duration, 0), \
                 COALESCE(last_position, 0), created_at, updated_at, deleted_at, folder_id, \
                 COALESCE(start_time, 0), COALESCE(end_time, 0), COALESCE(sort_order, 0), \
                 COALESCE(last_writer, '') FROM videos",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                let folder_id: Option<i64> = row.get(8)?;
                Ok(WireVideo {
                    id: row.get(0)?,
                    title: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    thumbnail_url: row.get(2)?,
                    duration: row.get(3)?,
                    last_position: row.get(4)?,
                    created_at: row.get::<_, Option<i64>>(5)?.unwrap_or(0),
                    updated_at: row.get::<_, Option<i64>>(6)?.unwrap_or(0),
                    deleted_at: row.get(7)?,
                    folder_uid: folder_id.and_then(|id| folder_uid.get(&id).cloned()),
                    start_time: row.get(9)?,
                    end_time: row.get(10)?,
                    sort_order: row.get(11)?,
                    last_writer: row.get(12)?,
                })
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            let video = row.map_err(|e| e.to_string())?;
            snapshot.videos.insert(video.id.clone(), video);
        }
    }

    {
        let mut stmt = conn
            .prepare(
                "SELECT uid, video_id, COALESCE(start_time, 0), COALESCE(end_time, 0), \
                 COALESCE(title, ''), created_at, updated_at, deleted_at, COALESCE(sort_order, 0), \
                 COALESCE(last_writer, '') FROM clips WHERE uid IS NOT NULL AND uid != ''",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(WireClip {
                    uid: row.get(0)?,
                    video_id: row.get(1)?,
                    start_time: row.get(2)?,
                    end_time: row.get(3)?,
                    title: row.get(4)?,
                    created_at: row.get::<_, Option<i64>>(5)?.unwrap_or(0),
                    updated_at: row.get::<_, Option<i64>>(6)?.unwrap_or(0),
                    deleted_at: row.get(7)?,
                    sort_order: row.get(8)?,
                    last_writer: row.get(9)?,
                })
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            let clip = row.map_err(|e| e.to_string())?;
            snapshot.clips.insert(clip.uid.clone(), clip);
        }
    }

    Ok(snapshot)
}

/// This device's outgoing log: every record it last wrote since compaction
/// last folded its work into `library.json`.
pub fn outgoing_ops(
    snapshot: &Snapshot,
    device: &str,
    compacted_through: i64,
) -> Vec<Op> {
    let mut ops: Vec<Op> = Vec::new();
    let mine = |writer: &str, updated_at: i64| {
        writer == device && updated_at > compacted_through
    };

    for folder in snapshot.folders.values() {
        if mine(&folder.last_writer, folder.updated_at) {
            ops.push(Op {
                device: device.to_string(),
                at: folder.updated_at,
                record: Record::Folder(folder.clone()),
            });
        }
    }
    for video in snapshot.videos.values() {
        if mine(&video.last_writer, video.updated_at) {
            ops.push(Op {
                device: device.to_string(),
                at: video.updated_at,
                record: Record::Video(video.clone()),
            });
        }
    }
    for clip in snapshot.clips.values() {
        if mine(&clip.last_writer, clip.updated_at) {
            ops.push(Op {
                device: device.to_string(),
                at: clip.updated_at,
                record: Record::Clip(clip.clone()),
            });
        }
    }

    ops.sort_by_key(|op| op.at);
    ops
}

pub fn ops_to_jsonl(ops: &[Op]) -> Result<String, String> {
    let mut out = String::new();
    for op in ops {
        out.push_str(&serde_json::to_string(op).map_err(|e| e.to_string())?);
        out.push('\n');
    }
    Ok(out)
}

/// Parse a device log. A line that does not parse is skipped rather than
/// failing the sync — one corrupt record must not strand every other device.
pub fn ops_from_jsonl(text: &str) -> (Vec<Op>, usize) {
    let mut ops = Vec::new();
    let mut skipped = 0;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<Op>(line) {
            Ok(op) => ops.push(op),
            Err(_) => skipped += 1,
        }
    }
    (ops, skipped)
}

/// The highest op timestamp per device, for the compaction watermark.
pub fn high_water(ops: &[Op]) -> BTreeMap<String, i64> {
    let mut marks: BTreeMap<String, i64> = BTreeMap::new();
    for op in ops {
        let entry = marks.entry(op.device.clone()).or_insert(i64::MIN);
        *entry = (*entry).max(op.at);
    }
    marks
}

// ── writing ────────────────────────────────────────────────────────────────

/// Write a merged snapshot back to SQLite, resolving uids to rowids.
///
/// Runs inside a transaction supplied by the caller so a failed merge leaves
/// the database exactly as it was.
/// Returns how many records were skipped as unwritable.
pub fn write_snapshot(tx: &Connection, snapshot: &Snapshot) -> Result<usize, String> {
    let mut skipped = 0usize;
    // Folders first, parents before children, so `parent_uid` always resolves.
    let mut rowid: BTreeMap<String, i64> = BTreeMap::new();
    for folder in snapshot.folders_parents_first() {
        let parent_id: Option<i64> = folder
            .parent_uid
            .as_ref()
            .and_then(|uid| rowid.get(uid).copied());

        let existing: Option<i64> = tx
            .query_row(
                "SELECT id FROM folders WHERE uid = ?1",
                params![folder.uid],
                |row| row.get(0),
            )
            .ok();

        let id = match existing {
            Some(id) => {
                tx.execute(
                    "UPDATE folders SET name = ?1, created_at = ?2, updated_at = ?3, \
                     deleted_at = ?4, sort_order = ?5, parent_id = ?6, last_writer = ?7 WHERE id = ?8",
                    params![
                        folder.name,
                        folder.created_at,
                        folder.updated_at,
                        folder.deleted_at,
                        folder.sort_order,
                        parent_id,
                        folder.last_writer,
                        id
                    ],
                )
                .map_err(|e| e.to_string())?;
                id
            }
            None => {
                tx.execute(
                    "INSERT INTO folders (uid, name, created_at, updated_at, deleted_at, \
                     sort_order, parent_id, last_writer) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        folder.uid,
                        folder.name,
                        folder.created_at,
                        folder.updated_at,
                        folder.deleted_at,
                        folder.sort_order,
                        parent_id,
                        folder.last_writer
                    ],
                )
                .map_err(|e| e.to_string())?;
                tx.last_insert_rowid()
            }
        };
        rowid.insert(folder.uid.clone(), id);
    }

    for video in snapshot.videos.values() {
        let folder_id: Option<i64> = video
            .folder_uid
            .as_ref()
            .and_then(|uid| rowid.get(uid).copied());
        tx.execute(
            "INSERT INTO videos (id, title, thumbnail_url, duration, last_position, created_at, \
             updated_at, deleted_at, folder_id, start_time, end_time, sort_order, last_writer) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13) \
             ON CONFLICT(id) DO UPDATE SET title = excluded.title, \
             thumbnail_url = excluded.thumbnail_url, duration = excluded.duration, \
             last_position = excluded.last_position, updated_at = excluded.updated_at, \
             deleted_at = excluded.deleted_at, folder_id = excluded.folder_id, \
             start_time = excluded.start_time, end_time = excluded.end_time, \
             sort_order = excluded.sort_order, last_writer = excluded.last_writer",
            params![
                video.id,
                video.title,
                video.thumbnail_url,
                video.duration,
                video.last_position,
                video.created_at,
                video.updated_at,
                video.deleted_at,
                folder_id,
                video.start_time,
                video.end_time,
                video.sort_order,
                video.last_writer
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    for clip in snapshot.clips.values() {
        // `clips.video_id` is a real foreign key and rusqlite enforces it, so a
        // clip whose video is missing would abort the whole transaction and
        // strand every other device's changes with it. One unshowable clip is
        // not worth that, so skip it and carry on.
        if !snapshot.videos.contains_key(&clip.video_id) {
            let known: bool = tx
                .query_row(
                    "SELECT 1 FROM videos WHERE id = ?1",
                    params![clip.video_id],
                    |_| Ok(true),
                )
                .unwrap_or(false);
            if !known {
                skipped += 1;
                continue;
            }
        }

        // Conflict on `uid`, never on `id`. The rowid is a local join key and
        // writing one from a remote payload is what destroyed clip identity
        // before: two devices both number a clip 3, and the merge overwrote
        // whichever one it saw second — including its uid.
        let existing: Option<i64> = tx
            .query_row(
                "SELECT id FROM clips WHERE uid = ?1",
                params![clip.uid],
                |row| row.get(0),
            )
            .ok();

        match existing {
            Some(id) => {
                tx.execute(
                    "UPDATE clips SET video_id = ?1, start_time = ?2, end_time = ?3, title = ?4, \
                     created_at = ?5, updated_at = ?6, deleted_at = ?7, sort_order = ?8, \
                     last_writer = ?9 WHERE id = ?10",
                    params![
                        clip.video_id,
                        clip.start_time,
                        clip.end_time,
                        clip.title,
                        clip.created_at,
                        clip.updated_at,
                        clip.deleted_at,
                        clip.sort_order,
                        clip.last_writer,
                        id
                    ],
                )
                .map_err(|e| e.to_string())?;
            }
            None => {
                tx.execute(
                    "INSERT INTO clips (uid, video_id, start_time, end_time, title, created_at, \
                     updated_at, deleted_at, sort_order, last_writer) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        clip.uid,
                        clip.video_id,
                        clip.start_time,
                        clip.end_time,
                        clip.title,
                        clip.created_at,
                        clip.updated_at,
                        clip.deleted_at,
                        clip.sort_order,
                        clip.last_writer
                    ],
                )
                .map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(skipped)
}

// ── watermarks, in one place ───────────────────────────────────────────────

pub fn read_watermarks(conn: &Connection) -> BTreeMap<String, i64> {
    conn.query_row(
        "SELECT value FROM sync_metadata WHERE key = 'compacted_through'",
        [],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .and_then(|text| serde_json::from_str(&text).ok())
    .unwrap_or_default()
}

pub fn write_watermarks(conn: &Connection, marks: &BTreeMap<String, i64>) -> Result<(), String> {
    let text = serde_json::to_string(marks).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO sync_metadata (key, value) VALUES ('compacted_through', ?1)",
        params![text],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub fn library_from_snapshot(
    snapshot: &Snapshot,
    compacted_through: BTreeMap<String, i64>,
) -> Library {
    snapshot.to_library(compacted_through, current_time())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::Snapshot;

    fn memory_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::setup_db_for_tests(&conn).unwrap();
        conn
    }


    fn video(id: &str, writer: &str) -> WireVideo {
        WireVideo {
            id: id.into(),
            title: "A video".into(),
            thumbnail_url: String::new(),
            duration: 0,
            last_position: 0,
            folder_uid: None,
            start_time: 0,
            end_time: 0,
            sort_order: 0,
            created_at: 1,
            updated_at: 100,
            deleted_at: None,
            last_writer: writer.into(),
        }
    }

    fn folder(uid: &str, name: &str, parent: Option<&str>, writer: &str) -> WireFolder {
        WireFolder {
            uid: uid.into(),
            name: name.into(),
            parent_uid: parent.map(str::to_string),
            sort_order: 0,
            created_at: 1,
            updated_at: 100,
            deleted_at: None,
            last_writer: writer.into(),
        }
    }

    #[test]
    fn a_snapshot_survives_a_round_trip_through_sqlite() {
        let conn = memory_db();
        let mut snapshot = Snapshot::default();
        snapshot.merge_folder(folder("f-parent", "Rust", None, "dev-a"));
        snapshot.merge_folder(folder("f-child", "Ownership", Some("f-parent"), "dev-a"));
        snapshot.merge_video(WireVideo {
            id: "vid1".into(),
            title: "A video".into(),
            thumbnail_url: "thumb".into(),
            duration: 0,
            last_position: 120,
            folder_uid: Some("f-child".into()),
            start_time: 0,
            end_time: 0,
            sort_order: 0,
            created_at: 1,
            updated_at: 100,
            deleted_at: None,
            last_writer: "dev-a".into(),
        });
        snapshot.merge_clip(WireClip {
            uid: "c1".into(),
            video_id: "vid1".into(),
            start_time: 414,
            end_time: 460,
            title: "The bit".into(),
            sort_order: 0,
            created_at: 1,
            updated_at: 100,
            deleted_at: None,
            last_writer: "dev-a".into(),
        });

        write_snapshot(&conn, &snapshot).unwrap();
        let back = read_snapshot(&conn).unwrap();

        assert_eq!(back, snapshot, "uid links must survive rowid resolution");
        assert_eq!(back.folders["f-child"].parent_uid.as_deref(), Some("f-parent"));
        assert_eq!(back.videos["vid1"].folder_uid.as_deref(), Some("f-child"));
    }

    #[test]
    fn writing_twice_is_idempotent_and_does_not_duplicate_rows() {
        let conn = memory_db();
        let mut snapshot = Snapshot::default();
        snapshot.merge_folder(folder("f1", "Rust", None, "dev-a"));
        snapshot.merge_video(video("vid1", "dev-a"));
        snapshot.merge_clip(WireClip {
            uid: "c1".into(),
            video_id: "vid1".into(),
            start_time: 0,
            end_time: 0,
            title: "One".into(),
            sort_order: 0,
            created_at: 1,
            updated_at: 100,
            deleted_at: None,
            last_writer: "dev-a".into(),
        });

        write_snapshot(&conn, &snapshot).unwrap();
        write_snapshot(&conn, &snapshot).unwrap();

        let folders: i64 = conn
            .query_row("SELECT COUNT(*) FROM folders", [], |r| r.get(0))
            .unwrap();
        let clips: i64 = conn
            .query_row("SELECT COUNT(*) FROM clips", [], |r| r.get(0))
            .unwrap();
        assert_eq!((folders, clips), (1, 1));
    }

    #[test]
    fn a_remote_clip_never_overwrites_a_local_clip_that_shares_its_rowid() {
        // The exact P0: device A's clip 1 and device B's clip 1 are different
        // clips. Applying B's must not consume A's row or its uid.
        let conn = memory_db();
        let mut local = Snapshot::default();
        local.merge_video(video("vid", "dev-a"));
        local.merge_clip(WireClip {
            uid: "uid-a".into(),
            video_id: "vid".into(),
            start_time: 10,
            end_time: 20,
            title: "From A".into(),
            sort_order: 0,
            created_at: 1,
            updated_at: 100,
            deleted_at: None,
            last_writer: "dev-a".into(),
        });
        write_snapshot(&conn, &local).unwrap();

        let first_rowid: i64 = conn
            .query_row("SELECT id FROM clips WHERE uid = 'uid-a'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(first_rowid, 1, "precondition: local clip holds rowid 1");

        let mut incoming = local.clone();
        incoming.merge_clip(WireClip {
            uid: "uid-b".into(),
            video_id: "vid".into(),
            start_time: 30,
            end_time: 40,
            title: "From B".into(),
            sort_order: 0,
            created_at: 1,
            updated_at: 100,
            deleted_at: None,
            last_writer: "dev-b".into(),
        });
        write_snapshot(&conn, &incoming).unwrap();

        let back = read_snapshot(&conn).unwrap();
        assert_eq!(back.clips.len(), 2, "both clips must survive");
        assert_eq!(back.clips["uid-a"].title, "From A");
        assert_eq!(back.clips["uid-b"].title, "From B");
        let still: i64 = conn
            .query_row("SELECT id FROM clips WHERE uid = 'uid-a'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(still, first_rowid, "local identity must be untouched");
    }

    #[test]
    fn a_clip_whose_video_is_missing_is_skipped_not_fatal() {
        // `clips.video_id` is an enforced foreign key. Letting one orphan clip
        // abort the transaction would strand every other device's changes.
        let conn = memory_db();
        let mut snapshot = Snapshot::default();
        snapshot.merge_video(video("present", "dev-a"));
        snapshot.merge_clip(WireClip {
            uid: "good".into(),
            video_id: "present".into(),
            start_time: 0,
            end_time: 0,
            title: "Keeps".into(),
            sort_order: 0,
            created_at: 1,
            updated_at: 100,
            deleted_at: None,
            last_writer: "dev-a".into(),
        });
        snapshot.merge_clip(WireClip {
            uid: "orphan".into(),
            video_id: "never-synced".into(),
            start_time: 0,
            end_time: 0,
            title: "Dropped".into(),
            sort_order: 0,
            created_at: 1,
            updated_at: 100,
            deleted_at: None,
            last_writer: "dev-b".into(),
        });

        let skipped = write_snapshot(&conn, &snapshot).unwrap();
        assert_eq!(skipped, 1);

        let back = read_snapshot(&conn).unwrap();
        assert!(back.clips.contains_key("good"), "the healthy clip must land");
        assert!(!back.clips.contains_key("orphan"));
    }

    /// Two devices edit offline, exchange logs, and must end up identical.
    #[test]
    fn two_devices_converge_through_real_sqlite() {
        let a_conn = memory_db();
        let b_conn = memory_db();

        // Shared starting point: one video, already synced everywhere.
        let mut base = Snapshot::default();
        base.merge_video(video("vid", "dev-a"));
        write_snapshot(&a_conn, &base).unwrap();
        write_snapshot(&b_conn, &base).unwrap();

        // Offline: A makes a folder and a clip; B makes a different folder and
        // renames the video. Both start from local rowid 1 for their folder.
        let mut a = read_snapshot(&a_conn).unwrap();
        a.merge_folder(folder("f-a", "Rust", None, "dev-a"));
        a.merge_clip(WireClip {
            uid: "c-a".into(),
            video_id: "vid".into(),
            start_time: 414,
            end_time: 460,
            title: "From A".into(),
            sort_order: 0,
            created_at: 1,
            updated_at: 110,
            deleted_at: None,
            last_writer: "dev-a".into(),
        });
        write_snapshot(&a_conn, &a).unwrap();

        let mut b = read_snapshot(&b_conn).unwrap();
        b.merge_folder(folder("f-b", "OBS", None, "dev-b"));
        let mut renamed = video("vid", "dev-b");
        renamed.title = "Renamed by B".into();
        renamed.updated_at = 120;
        b.merge_video(renamed);
        write_snapshot(&b_conn, &b).unwrap();

        assert_eq!(
            a_conn
                .query_row("SELECT id FROM folders WHERE uid = 'f-a'", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            b_conn
                .query_row("SELECT id FROM folders WHERE uid = 'f-b'", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            "precondition: both devices gave their folder the same rowid",
        );

        // Exchange: each publishes its own log, then applies the other's.
        let a_ops = outgoing_ops(&read_snapshot(&a_conn).unwrap(), "dev-a", i64::MIN);
        let b_ops = outgoing_ops(&read_snapshot(&b_conn).unwrap(), "dev-b", i64::MIN);

        let mut a_final = read_snapshot(&a_conn).unwrap();
        a_final.merge_ops(&b_ops);
        a_final.repair_links();
        write_snapshot(&a_conn, &a_final).unwrap();

        let mut b_final = read_snapshot(&b_conn).unwrap();
        b_final.merge_ops(&a_ops);
        b_final.repair_links();
        write_snapshot(&b_conn, &b_final).unwrap();

        let a_out = read_snapshot(&a_conn).unwrap();
        let b_out = read_snapshot(&b_conn).unwrap();
        assert_eq!(a_out, b_out, "the two devices must agree");
        assert_eq!(a_out.folders.len(), 2, "neither folder may be lost");
        assert_eq!(a_out.folders["f-a"].name, "Rust");
        assert_eq!(a_out.folders["f-b"].name, "OBS");
        assert_eq!(a_out.clips["c-a"].title, "From A");
        assert_eq!(a_out.videos["vid"].title, "Renamed by B");
    }

    #[test]
    fn outgoing_ops_carry_only_this_devices_work_since_compaction() {
        let mut snapshot = Snapshot::default();
        snapshot.merge_folder(folder("mine-old", "Old", None, "dev-a"));
        snapshot.folders.get_mut("mine-old").unwrap().updated_at = 50;
        snapshot.merge_folder(folder("mine-new", "New", None, "dev-a"));
        snapshot.merge_folder(folder("theirs", "Theirs", None, "dev-b"));

        let ops = outgoing_ops(&snapshot, "dev-a", 80);
        let uids: Vec<&str> = ops
            .iter()
            .map(|op| match &op.record {
                Record::Folder(f) => f.uid.as_str(),
                _ => "?",
            })
            .collect();
        assert_eq!(uids, vec!["mine-new"]);
    }

    #[test]
    fn a_corrupt_log_line_is_skipped_not_fatal() {
        let good = Op {
            device: "dev-a".into(),
            at: 100,
            record: Record::Folder(folder("f1", "Rust", None, "dev-a")),
        };
        let jsonl = format!(
            "{}\nnot json at all\n{{\"device\":\"x\"}}\n\n",
            serde_json::to_string(&good).unwrap()
        );
        let (ops, skipped) = ops_from_jsonl(&jsonl);
        assert_eq!(ops.len(), 1);
        assert_eq!(skipped, 2);
        assert_eq!(ops[0], good);
    }

    #[test]
    fn jsonl_round_trips() {
        let ops = vec![Op {
            device: "dev-a".into(),
            at: 100,
            record: Record::Folder(folder("f1", "Rust", None, "dev-a")),
        }];
        let (back, skipped) = ops_from_jsonl(&ops_to_jsonl(&ops).unwrap());
        assert_eq!(skipped, 0);
        assert_eq!(back, ops);
    }

    #[test]
    fn high_water_takes_the_max_per_device() {
        let mk = |device: &str, at: i64| Op {
            device: device.into(),
            at,
            record: Record::Folder(folder("f", "F", None, device)),
        };
        let marks = high_water(&[mk("a", 10), mk("b", 50), mk("a", 30)]);
        assert_eq!(marks["a"], 30);
        assert_eq!(marks["b"], 50);
    }

    #[test]
    fn watermarks_have_exactly_one_home() {
        let conn = memory_db();
        assert!(read_watermarks(&conn).is_empty());
        let marks = BTreeMap::from([("dev-a".to_string(), 100i64)]);
        write_watermarks(&conn, &marks).unwrap();
        assert_eq!(read_watermarks(&conn), marks);
    }
}
