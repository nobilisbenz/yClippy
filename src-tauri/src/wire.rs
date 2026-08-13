//! The sync wire format, and the merge.
//!
//! Everything here is pure and keyed on stable identity, which is the whole
//! point. The local database identifies folders and clips by an `AUTOINCREMENT`
//! rowid, and two devices that each create a folder both call it `1`. Nothing on
//! the wire may ever carry one of those numbers: a folder is its `uid`, a clip
//! is its `uid`, and a video is its YouTube id. Parent and folder links travel
//! as uids too, and are resolved back to rowids only when writing to SQLite.
//!
//! ## Conflict resolution
//!
//! Whole-record last-writer-wins, ordered by `(updated_at, last_writer)`.
//!
//! The timestamp is a wall clock, so a device with a fast clock wins more often
//! than it should — that is a real limitation and the reason `last_writer` is
//! here. Without a tiebreak, two devices that edit the same record in the same
//! millisecond each keep their own version *forever*, because neither is
//! strictly newer. Comparing the device id second makes every device pick the
//! same winner, so the fleet converges even when the clocks disagree.
//!
//! A delete is an upsert with `deleted_at` set. The schema already soft-deletes,
//! so tombstones travel down the same path as edits and cannot be reordered
//! against them.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireFolder {
    pub uid: String,
    pub name: String,
    #[serde(default)]
    pub parent_uid: Option<String>,
    #[serde(default)]
    pub sort_order: i64,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
    #[serde(default)]
    pub deleted_at: Option<i64>,
    #[serde(default)]
    pub last_writer: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireVideo {
    /// The YouTube id. Stable everywhere, so videos need no separate uid.
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub thumbnail_url: String,
    #[serde(default)]
    pub duration: i64,
    #[serde(default)]
    pub last_position: i64,
    #[serde(default)]
    pub folder_uid: Option<String>,
    #[serde(default)]
    pub start_time: i64,
    #[serde(default)]
    pub end_time: i64,
    #[serde(default)]
    pub sort_order: i64,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
    #[serde(default)]
    pub deleted_at: Option<i64>,
    #[serde(default)]
    pub last_writer: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireClip {
    pub uid: String,
    pub video_id: String,
    #[serde(default)]
    pub start_time: i64,
    #[serde(default)]
    pub end_time: i64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub sort_order: i64,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
    #[serde(default)]
    pub deleted_at: Option<i64>,
    #[serde(default)]
    pub last_writer: String,
}

/// One record, whatever kind. A delete is one of these with `deleted_at` set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "entity", rename_all = "lowercase")]
pub enum Record {
    Folder(WireFolder),
    Video(WireVideo),
    Clip(WireClip),
}

impl Record {
    pub fn updated_at(&self) -> i64 {
        match self {
            Self::Folder(f) => f.updated_at,
            Self::Video(v) => v.updated_at,
            Self::Clip(c) => c.updated_at,
        }
    }
}

/// A line in a device's append-only log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Op {
    pub device: String,
    /// When the op was recorded. Used for watermarking, not for conflicts —
    /// the record's own `updated_at` decides those.
    pub at: i64,
    #[serde(flatten)]
    pub record: Record,
}

/// The canonical merged state, rewritten by compaction.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Library {
    #[serde(default)]
    pub protocol_version: u32,
    #[serde(default)]
    pub folders: Vec<WireFolder>,
    #[serde(default)]
    pub videos: Vec<WireVideo>,
    #[serde(default)]
    pub clips: Vec<WireClip>,
    /// Per device, the highest op timestamp already folded into this file. A
    /// device may drop its own ops up to its own watermark and no further —
    /// which is what makes truncation safe.
    #[serde(default)]
    pub compacted_through: BTreeMap<String, i64>,
    #[serde(default)]
    pub updated_at: i64,
}

/// The whole library in memory, keyed by identity.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub folders: BTreeMap<String, WireFolder>,
    pub videos: BTreeMap<String, WireVideo>,
    pub clips: BTreeMap<String, WireClip>,
}

/// Does `incoming` replace `current`? See the module docs for why the device id
/// is part of the comparison.
fn wins(incoming: (i64, &str), current: (i64, &str)) -> bool {
    incoming > current
}

impl Snapshot {
    pub fn from_library(library: &Library) -> Self {
        let mut snapshot = Self::default();
        snapshot.merge_library(library);
        snapshot
    }

    pub fn merge_library(&mut self, library: &Library) {
        for folder in &library.folders {
            self.merge_folder(folder.clone());
        }
        for video in &library.videos {
            self.merge_video(video.clone());
        }
        for clip in &library.clips {
            self.merge_clip(clip.clone());
        }
    }

    /// Apply ops in timestamp order. Ordering the batch first means a log that
    /// interleaves edits to one record still lands on the newest.
    pub fn merge_ops(&mut self, ops: &[Op]) {
        let mut ordered: Vec<&Op> = ops.iter().collect();
        ordered.sort_by_key(|op| (op.at, op.device.clone()));
        for op in ordered {
            self.merge_record(op.record.clone());
        }
    }

    pub fn merge_record(&mut self, record: Record) {
        match record {
            Record::Folder(f) => self.merge_folder(f),
            Record::Video(v) => self.merge_video(v),
            Record::Clip(c) => self.merge_clip(c),
        }
    }

    pub fn merge_folder(&mut self, incoming: WireFolder) {
        match self.folders.get(&incoming.uid) {
            Some(current)
                if !wins(
                    (incoming.updated_at, &incoming.last_writer),
                    (current.updated_at, &current.last_writer),
                ) => {}
            _ => {
                self.folders.insert(incoming.uid.clone(), incoming);
            }
        }
    }

    pub fn merge_video(&mut self, incoming: WireVideo) {
        match self.videos.get(&incoming.id) {
            Some(current)
                if !wins(
                    (incoming.updated_at, &incoming.last_writer),
                    (current.updated_at, &current.last_writer),
                ) => {}
            _ => {
                self.videos.insert(incoming.id.clone(), incoming);
            }
        }
    }

    pub fn merge_clip(&mut self, incoming: WireClip) {
        match self.clips.get(&incoming.uid) {
            Some(current)
                if !wins(
                    (incoming.updated_at, &incoming.last_writer),
                    (current.updated_at, &current.last_writer),
                ) => {}
            _ => {
                self.clips.insert(incoming.uid.clone(), incoming);
            }
        }
    }

    /// Drop links that point at nothing, so a partially-synced peer cannot
    /// leave a video parented to a folder this device has never heard of.
    pub fn repair_links(&mut self) {
        let known: Vec<String> = self.folders.keys().cloned().collect();
        let known: std::collections::BTreeSet<&str> =
            known.iter().map(String::as_str).collect();

        for folder in self.folders.values_mut() {
            if let Some(parent) = &folder.parent_uid {
                if parent == &folder.uid || !known.contains(parent.as_str()) {
                    folder.parent_uid = None;
                }
            }
        }
        for video in self.videos.values_mut() {
            if let Some(parent) = &video.folder_uid {
                if !known.contains(parent.as_str()) {
                    video.folder_uid = None;
                }
            }
        }
        self.break_folder_cycles();
    }

    /// Two devices can each re-parent a folder under the other. Neither move is
    /// wrong on its own; together they make a ring with no root, which would
    /// hide both folders from a tree walk forever. Cut the ring at whichever
    /// member sorts first, so every device cuts it in the same place.
    fn break_folder_cycles(&mut self) {
        let uids: Vec<String> = self.folders.keys().cloned().collect();
        for uid in uids {
            let mut seen = std::collections::BTreeSet::new();
            seen.insert(uid.clone());
            let mut cursor = self.folders.get(&uid).and_then(|f| f.parent_uid.clone());
            while let Some(current) = cursor {
                if !seen.insert(current.clone()) {
                    // `seen` is ordered, so its first element is the same on
                    // every device that observes this cycle.
                    if let Some(cut) = seen.iter().next().cloned() {
                        if let Some(folder) = self.folders.get_mut(&cut) {
                            folder.parent_uid = None;
                        }
                    }
                    break;
                }
                cursor = self
                    .folders
                    .get(&current)
                    .and_then(|f| f.parent_uid.clone());
            }
        }
    }

    /// Folders ordered so every parent precedes its children, which is what
    /// lets the writer resolve `parent_uid` to a rowid in one pass.
    pub fn folders_parents_first(&self) -> Vec<&WireFolder> {
        let mut out: Vec<&WireFolder> = Vec::with_capacity(self.folders.len());
        let mut placed = std::collections::BTreeSet::new();
        // Bounded by depth; `repair_links` has already removed any cycle.
        while out.len() < self.folders.len() {
            let mut progressed = false;
            for (uid, folder) in &self.folders {
                if placed.contains(uid) {
                    continue;
                }
                let ready = match &folder.parent_uid {
                    None => true,
                    Some(parent) => placed.contains(parent) || !self.folders.contains_key(parent),
                };
                if ready {
                    out.push(folder);
                    placed.insert(uid.clone());
                    progressed = true;
                }
            }
            if !progressed {
                // Defensive: emit whatever is left rather than spin.
                for (uid, folder) in &self.folders {
                    if !placed.contains(uid) {
                        out.push(folder);
                    }
                }
                break;
            }
        }
        out
    }

    pub fn to_library(&self, compacted_through: BTreeMap<String, i64>, now: i64) -> Library {
        Library {
            protocol_version: PROTOCOL_VERSION,
            folders: self.folders.values().cloned().collect(),
            videos: self.videos.values().cloned().collect(),
            clips: self.clips.values().cloned().collect(),
            compacted_through,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(uid: &str, name: &str, updated_at: i64, writer: &str) -> WireFolder {
        WireFolder {
            uid: uid.into(),
            name: name.into(),
            parent_uid: None,
            sort_order: 0,
            created_at: 1,
            updated_at,
            deleted_at: None,
            last_writer: writer.into(),
        }
    }

    fn clip(uid: &str, title: &str, updated_at: i64, writer: &str) -> WireClip {
        WireClip {
            uid: uid.into(),
            video_id: "vid".into(),
            start_time: 0,
            end_time: 0,
            title: title.into(),
            sort_order: 0,
            created_at: 1,
            updated_at,
            deleted_at: None,
            last_writer: writer.into(),
        }
    }

    #[test]
    fn two_devices_creating_folders_offline_both_survive() {
        // The failure the old merge had: both devices call their new folder
        // rowid 1, and one silently overwrites the other.
        let mut a = Snapshot::default();
        a.merge_folder(folder("uid-a", "Rust", 100, "device-a"));

        let mut b = Snapshot::default();
        b.merge_folder(folder("uid-b", "OBS", 100, "device-b"));

        // Each pulls the other's log.
        a.merge_folder(folder("uid-b", "OBS", 100, "device-b"));
        b.merge_folder(folder("uid-a", "Rust", 100, "device-a"));

        assert_eq!(a.folders.len(), 2);
        assert_eq!(a, b, "devices must converge");
        assert_eq!(a.folders["uid-a"].name, "Rust");
        assert_eq!(a.folders["uid-b"].name, "OBS");
    }

    #[test]
    fn the_newer_edit_wins_regardless_of_arrival_order() {
        let mut forward = Snapshot::default();
        forward.merge_folder(folder("uid", "old", 100, "a"));
        forward.merge_folder(folder("uid", "new", 200, "b"));

        let mut backward = Snapshot::default();
        backward.merge_folder(folder("uid", "new", 200, "b"));
        backward.merge_folder(folder("uid", "old", 100, "a"));

        assert_eq!(forward.folders["uid"].name, "new");
        assert_eq!(backward.folders["uid"].name, "new", "merge must commute");
        assert_eq!(forward, backward);
    }

    #[test]
    fn a_tie_is_broken_the_same_way_on_every_device() {
        // Without this, both devices keep their own version forever and the
        // fleet never converges.
        let mut a = Snapshot::default();
        a.merge_folder(folder("uid", "from-a", 100, "device-a"));
        a.merge_folder(folder("uid", "from-b", 100, "device-b"));

        let mut b = Snapshot::default();
        b.merge_folder(folder("uid", "from-b", 100, "device-b"));
        b.merge_folder(folder("uid", "from-a", 100, "device-a"));

        assert_eq!(a, b, "a tie must resolve identically everywhere");
        assert_eq!(a.folders["uid"].name, "from-b");
    }

    #[test]
    fn merging_is_idempotent() {
        let mut once = Snapshot::default();
        once.merge_clip(clip("c1", "The bit", 100, "a"));

        let mut twice = once.clone();
        twice.merge_clip(clip("c1", "The bit", 100, "a"));
        twice.merge_clip(clip("c1", "The bit", 100, "a"));

        assert_eq!(once, twice, "replaying a log must not change the result");
    }

    #[test]
    fn a_delete_travels_as_an_upsert_and_beats_an_older_edit() {
        let mut snapshot = Snapshot::default();
        snapshot.merge_clip(clip("c1", "The bit", 100, "a"));

        let mut tombstone = clip("c1", "The bit", 200, "b");
        tombstone.deleted_at = Some(200);
        snapshot.merge_clip(tombstone);
        assert_eq!(snapshot.clips["c1"].deleted_at, Some(200));

        // And an edit older than the delete does not resurrect it.
        snapshot.merge_clip(clip("c1", "Renamed", 150, "a"));
        assert_eq!(snapshot.clips["c1"].deleted_at, Some(200));

        // But a newer edit legitimately does.
        snapshot.merge_clip(clip("c1", "Renamed later", 300, "a"));
        assert_eq!(snapshot.clips["c1"].deleted_at, None);
    }

    #[test]
    fn ops_apply_in_timestamp_order_not_file_order() {
        let ops = vec![
            Op { device: "a".into(), at: 300, record: Record::Folder(folder("uid", "third", 300, "a")) },
            Op { device: "a".into(), at: 100, record: Record::Folder(folder("uid", "first", 100, "a")) },
            Op { device: "a".into(), at: 200, record: Record::Folder(folder("uid", "second", 200, "a")) },
        ];
        let mut snapshot = Snapshot::default();
        snapshot.merge_ops(&ops);
        assert_eq!(snapshot.folders["uid"].name, "third");
    }

    #[test]
    fn a_link_to_an_unknown_folder_is_dropped_rather_than_dangling() {
        let mut snapshot = Snapshot::default();
        snapshot.merge_folder(folder("known", "Rust", 100, "a"));
        let mut orphan = folder("child", "Child", 100, "a");
        orphan.parent_uid = Some("never-seen".into());
        snapshot.merge_folder(orphan);

        let mut video = WireVideo {
            id: "vid".into(),
            title: "V".into(),
            thumbnail_url: String::new(),
            duration: 0,
            last_position: 0,
            folder_uid: Some("never-seen".into()),
            start_time: 0,
            end_time: 0,
            sort_order: 0,
            created_at: 1,
            updated_at: 1,
            deleted_at: None,
            last_writer: "a".into(),
        };
        video.folder_uid = Some("never-seen".into());
        snapshot.merge_video(video);

        snapshot.repair_links();
        assert_eq!(snapshot.folders["child"].parent_uid, None);
        assert_eq!(snapshot.videos["vid"].folder_uid, None);
    }

    #[test]
    fn a_parent_cycle_is_cut_identically_on_every_device() {
        // Each device re-parented one folder under the other. Together that is
        // a ring, and a tree walk would never reach either folder again.
        let build = || {
            let mut snapshot = Snapshot::default();
            let mut a = folder("aaa", "A", 100, "x");
            let mut b = folder("bbb", "B", 100, "y");
            a.parent_uid = Some("bbb".into());
            b.parent_uid = Some("aaa".into());
            snapshot.merge_folder(a);
            snapshot.merge_folder(b);
            snapshot.repair_links();
            snapshot
        };

        let one = build();
        let two = build();
        assert_eq!(one, two);
        let rooted = one.folders.values().filter(|f| f.parent_uid.is_none()).count();
        assert_eq!(rooted, 1, "the ring must be cut exactly once");
    }

    #[test]
    fn folders_come_out_parents_first() {
        let mut snapshot = Snapshot::default();
        let mut child = folder("child", "Child", 100, "a");
        child.parent_uid = Some("parent".into());
        let mut grandchild = folder("grandchild", "Grandchild", 100, "a");
        grandchild.parent_uid = Some("child".into());
        // Inserted deepest-first on purpose.
        snapshot.merge_folder(grandchild);
        snapshot.merge_folder(child);
        snapshot.merge_folder(folder("parent", "Parent", 100, "a"));
        snapshot.repair_links();

        let order: Vec<&str> = snapshot
            .folders_parents_first()
            .iter()
            .map(|f| f.uid.as_str())
            .collect();
        assert_eq!(order, vec!["parent", "child", "grandchild"]);
    }

    #[test]
    fn a_library_round_trips_through_json() {
        let mut snapshot = Snapshot::default();
        snapshot.merge_folder(folder("uid", "Rust", 100, "a"));
        snapshot.merge_clip(clip("c1", "The bit", 100, "a"));

        let library = snapshot.to_library(BTreeMap::from([("a".into(), 100)]), 999);
        let json = serde_json::to_string(&library).unwrap();
        let back: Library = serde_json::from_str(&json).unwrap();

        assert_eq!(Snapshot::from_library(&back), snapshot);
        assert_eq!(back.compacted_through["a"], 100);
        assert_eq!(back.protocol_version, PROTOCOL_VERSION);
    }

    #[test]
    fn an_op_round_trips_through_jsonl() {
        let op = Op {
            device: "device-a".into(),
            at: 100,
            record: Record::Clip(clip("c1", "The bit", 100, "device-a")),
        };
        let line = serde_json::to_string(&op).unwrap();
        assert!(line.contains("\"entity\":\"clip\""));
        let back: Op = serde_json::from_str(&line).unwrap();
        assert_eq!(back, op);
    }
}
