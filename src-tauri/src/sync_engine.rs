//! Sync, in the order that keeps data.
//!
//! ```text
//! .notes/yclippy/library.json            canonical merged state, written by compaction
//! .notes/yclippy/devices/<device>.jsonl  one file per device, one writer, ever
//! ```
//!
//! The rule that makes this safe is that **no device ever writes another
//! device's file**. A phone appends only to its own log; `library.json` is only
//! rewritten by a compactor holding the SHA it read, so a concurrent compaction
//! loses the race instead of the data.
//!
//! The order is pull → merge → push → compact, and it is not negotiable. The
//! previous implementation pushed the whole local database *first*, which meant
//! a device returning from a week offline overwrote the remote with its own
//! stale world before it had seen a single thing anyone else did.

use crate::db::{self, DbState};
use crate::github_api::{self, GithubError};
use crate::oplog;
use crate::wire::{Library, Op, Snapshot};
use std::collections::BTreeMap;
use tauri::{AppHandle, Manager};

/// Where the library lives inside the vault repository. Matches the layout the
/// rest of the ecosystem uses for machine state.
const LIBRARY_PATH: &str = ".notes/yclippy/library.json";
const DEVICES_DIR: &str = ".notes/yclippy/devices";

pub struct SyncEngine {
    pub token: String,
    pub owner: String,
    pub repo: String,
}

#[derive(Debug, Default)]
pub struct SyncReport {
    pub devices_read: usize,
    pub ops_applied: usize,
    pub ops_pushed: usize,
    pub lines_skipped: usize,
    /// Records the merge could not write — a clip whose video is unknown.
    pub records_skipped: usize,
    pub compacted: bool,
}

impl SyncReport {
    pub fn describe(&self) -> String {
        let mut parts = vec![format!(
            "{} op{} from {} device{}",
            self.ops_applied,
            if self.ops_applied == 1 { "" } else { "s" },
            self.devices_read,
            if self.devices_read == 1 { "" } else { "s" },
        )];
        if self.ops_pushed > 0 {
            parts.push(format!("{} pushed", self.ops_pushed));
        }
        if self.compacted {
            parts.push("compacted".into());
        }
        let skipped = self.lines_skipped + self.records_skipped;
        if skipped > 0 {
            parts.push(format!("{skipped} unusable record(s) skipped"));
        }
        parts.join(", ")
    }
}

impl SyncEngine {
    pub fn new(token: String, repo_url: String) -> Option<Self> {
        let (owner, repo) = parse_repo_url(&repo_url)?;
        Some(Self { token, owner, repo })
    }

    pub async fn sync(&self, app: AppHandle) -> Result<String, String> {
        let device = {
            let state: tauri::State<DbState> = app.state();
            state.device_id.clone()
        };
        let mut report = SyncReport::default();

        // ── 1. Pull. A 404 means "nothing published yet"; every other failure
        // is a failure, and must not be mistaken for an empty remote.
        let (remote_library, library_sha) =
            match github_api::get_file(&self.token, &self.owner, &self.repo, LIBRARY_PATH).await {
                Ok(Some((text, sha))) => {
                    let library: Library = serde_json::from_str(&text).map_err(|e| {
                        format!("{LIBRARY_PATH} on the remote is not valid JSON: {e}")
                    })?;
                    (library, Some(sha))
                }
                Ok(None) => (Library::default(), None),
                Err(e) => return Err(format!("could not read {LIBRARY_PATH}: {e}")),
            };

        let device_files =
            match github_api::list_dir(&self.token, &self.owner, &self.repo, DEVICES_DIR).await {
                Ok(files) => files,
                Err(GithubError::NotFound) => Vec::new(),
                Err(e) => return Err(format!("could not list {DEVICES_DIR}: {e}")),
            };

        let mut remote_ops: Vec<Op> = Vec::new();
        for file in &device_files {
            if !file.name.ends_with(".jsonl") {
                continue;
            }
            let path = format!("{DEVICES_DIR}/{}", file.name);
            match github_api::get_file(&self.token, &self.owner, &self.repo, &path).await {
                Ok(Some((text, _))) => {
                    let (ops, skipped) = oplog::ops_from_jsonl(&text);
                    report.lines_skipped += skipped;
                    report.ops_applied += ops.len();
                    report.devices_read += 1;
                    remote_ops.extend(ops);
                }
                Ok(None) => {}
                Err(e) => return Err(format!("could not read {path}: {e}")),
            }
        }

        // ── 2. Merge, locally, in one transaction. Local state is the base, so
        // anything this device did while offline is already represented; the
        // remote is layered on top and the newer edit wins per record.
        let merged = {
            let state: tauri::State<DbState> = app.state();
            let mut conn = state.conn.lock().map_err(|_| "database lock poisoned")?;

            let mut snapshot = oplog::read_snapshot(&conn)?;
            snapshot.merge_library(&remote_library);
            snapshot.merge_ops(&remote_ops);
            snapshot.repair_links();

            let tx = conn.transaction().map_err(|e| e.to_string())?;
            report.records_skipped = oplog::write_snapshot(&tx, &snapshot)?;

            let mut marks = oplog::read_watermarks(&tx);
            for (k, v) in &remote_library.compacted_through {
                let entry = marks.entry(k.clone()).or_insert(i64::MIN);
                *entry = (*entry).max(*v);
            }
            oplog::write_watermarks(&tx, &marks)?;
            tx.commit().map_err(|e| e.to_string())?;

            snapshot
        };

        let compacted_through = {
            let state: tauri::State<DbState> = app.state();
            let conn = state.conn.lock().map_err(|_| "database lock poisoned")?;
            oplog::read_watermarks(&conn)
        };

        // ── 3. Push this device's own log, and only that file. Replacing it
        // wholesale is correct: it is derived from the database every time, so
        // it is self-healing rather than an append that can drift.
        let mine = oplog::outgoing_ops(
            &merged,
            &device,
            compacted_through.get(&device).copied().unwrap_or(i64::MIN),
        );
        report.ops_pushed = mine.len();

        if !mine.is_empty() {
            let path = format!("{DEVICES_DIR}/{device}.jsonl");
            let body = oplog::ops_to_jsonl(&mine)?;
            let sha = device_files
                .iter()
                .find(|f| f.name == format!("{device}.jsonl"))
                .map(|f| f.sha.clone());
            github_api::put_file(
                &self.token,
                &self.owner,
                &self.repo,
                &path,
                &body,
                sha,
                &format!("yClippy: {} change(s) from {device}", mine.len()),
            )
            .await
            .map_err(|e| format!("could not publish this device's log: {e}"))?;
        }

        // ── 4. Compact, last, and only on desktop. Folding the logs into
        // library.json is what lets each device shorten its own log, so it must
        // happen after every log has been read and this device's has been sent.
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            report.compacted = self
                .compact(&merged, &remote_ops, &mine, library_sha, &app)
                .await?;
        }
        #[cfg(any(target_os = "android", target_os = "ios"))]
        {
            let _ = library_sha;
        }

        Ok(report.describe())
    }

    /// Rewrite `library.json` from the merged state and record how far each
    /// device's log has been folded in.
    ///
    /// Returns false — not an error — when another device compacted first. The
    /// SHA guard means the loser simply retries next round, having already
    /// pulled the winner's work.
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    async fn compact(
        &self,
        merged: &Snapshot,
        remote_ops: &[Op],
        mine: &[Op],
        library_sha: Option<String>,
        app: &AppHandle,
    ) -> Result<bool, String> {
        let mut marks = oplog::high_water(remote_ops);
        for (device, at) in oplog::high_water(mine) {
            let entry = marks.entry(device).or_insert(i64::MIN);
            *entry = (*entry).max(at);
        }
        // Never advance a watermark past what this library actually contains.
        let marks: BTreeMap<String, i64> = marks
            .into_iter()
            .filter(|(_, at)| *at > i64::MIN)
            .collect();

        let library = oplog::library_from_snapshot(merged, marks.clone());
        let body = serde_json::to_string_pretty(&library).map_err(|e| e.to_string())?;

        // Mirror into the local vault, if one is configured, so an ordinary
        // `yalive sync` carries the library through git as well. Best-effort:
        // a missing or read-only vault must not fail the real sync.
        if let Some(vault) = db::load_config_pub(app).vault_path {
            if let Err(e) = mirror_into_vault(&vault, &body) {
                eprintln!("could not mirror the library into {vault}: {e}");
            }
        }

        match github_api::put_file(
            &self.token,
            &self.owner,
            &self.repo,
            LIBRARY_PATH,
            &body,
            library_sha,
            "yClippy: compact library",
        )
        .await
        {
            Ok(()) => {}
            // Someone else compacted between our read and our write. Their
            // version already includes what we pulled; leave it alone.
            Err(GithubError::Conflict) => return Ok(false),
            Err(e) => return Err(format!("could not write {LIBRARY_PATH}: {e}")),
        }

        let state: tauri::State<DbState> = app.state();
        let conn = state.conn.lock().map_err(|_| "database lock poisoned")?;
        oplog::write_watermarks(&conn, &marks)?;
        Ok(true)
    }
}

/// `https://github.com/owner/repo`, with or without `.git`, a trailing slash,
/// or an `owner/repo` shorthand.
fn parse_repo_url(url: &str) -> Option<(String, String)> {
    let url = url.trim().trim_end_matches('/');
    let url = url.strip_suffix(".git").unwrap_or(url);
    let mut parts = url.rsplit('/').filter(|part| !part.is_empty());
    let repo = parts.next()?;
    let owner = parts.next()?;
    if repo.is_empty() || owner.is_empty() || owner.contains(':') {
        return None;
    }
    Some((owner.to_string(), repo.to_string()))
}

/// Write `library.json` under `<vault>/.notes/yclippy/`, atomically, so a
/// crash mid-write cannot leave the vault holding a truncated file.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn mirror_into_vault(vault: &str, body: &str) -> std::io::Result<()> {
    use std::io::Write;

    let dir = std::path::Path::new(vault).join(".notes").join("yclippy");
    std::fs::create_dir_all(&dir)?;

    let final_path = dir.join("library.json");
    let temp_path = dir.join("library.json.pending");
    {
        let mut file = std::fs::File::create(&temp_path)?;
        file.write_all(body.as_bytes())?;
        file.sync_all()?;
    }
    // Rename is atomic within a filesystem; a partial write is not. Same
    // discipline the TUI uses for the yGraphy command file.
    std::fs::rename(&temp_path, &final_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_urls_parse_in_every_shape_you_might_paste() {
        let expected = Some(("nobilisbenz".to_string(), "notes".to_string()));
        assert_eq!(parse_repo_url("https://github.com/nobilisbenz/notes"), expected);
        // A trailing slash used to return None, which read as "invalid repo".
        assert_eq!(parse_repo_url("https://github.com/nobilisbenz/notes/"), expected);
        assert_eq!(parse_repo_url("https://github.com/nobilisbenz/notes.git"), expected);
        assert_eq!(parse_repo_url("  https://github.com/nobilisbenz/notes/  "), expected);
        assert_eq!(parse_repo_url("nobilisbenz/notes"), expected);
        assert_eq!(parse_repo_url("notes"), None);
        assert_eq!(parse_repo_url(""), None);
    }

    #[test]
    fn a_report_reads_like_something_a_person_would_say() {
        let report = SyncReport {
            devices_read: 2,
            ops_applied: 7,
            ops_pushed: 3,
            lines_skipped: 0,
            records_skipped: 0,
            compacted: true,
        };
        assert_eq!(report.describe(), "7 ops from 2 devices, 3 pushed, compacted");

        let quiet = SyncReport {
            devices_read: 1,
            ops_applied: 1,
            ..Default::default()
        };
        assert_eq!(quiet.describe(), "1 op from 1 device");
    }
}
