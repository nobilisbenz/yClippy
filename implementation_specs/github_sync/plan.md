# GitHub Sync Implementation Plan

## Objective
Enable users to synchronize their local `yclippy` data (videos, clips, folders) across devices using a private GitHub repository as the storage backend. The sync must handle CRUD operations, reordering/sorting, and conflict resolution intelligently.

## Architecture
- **Storage**: GitHub Repository (Private/Public).
- **Transport**: GitHub REST API (HTTPS) via `reqwest` (Rust).
- **Format**: JSON files per entity type (`videos.json`, `folders.json`, `clips.json`) for granular sync.
- **Strategy**: 
    1.  **Pull**: Fetch remote JSON files, merge into local DB using timestamps and UUIDs.
    2.  **Push**: Export local changes to JSON, upload to GitHub (with optimistic locking via SHA).

## Sync Data Model

### Entity Structure
Each entity (video, folder, clip) includes:
```json
{
  "id": "uuid-v4",
  "name": "string",
  "created_at": "ISO8601",
  "updated_at": "ISO8601",
  "deleted_at": "ISO8601 | null",
  "sort_order": "integer",
  "parent_id": "uuid | null"
}
```

### Change Tracking
- `deleted_at`: Soft delete for sync (null = active, timestamp = deleted)
- `updated_at`: Always updated on any modification
- `sort_order`: Integer for ordering within a parent

## Conflict Resolution Strategy

### Last-Write-Wins with Conflict Detection
1. Compare `updated_at` timestamps
2. If local and remote both modified since last sync:
   - Keep both versions (local gets `name (conflict)`, remote preserved)
   - User can resolve manually later
3. Otherwise: newer timestamp wins

### Sort Order Conflicts
- Use `sort_order` as secondary sort key
- If both changed: higher `sort_order` wins (more recent positioning)

## Sync Flow

### Pull Phase
1. Fetch remote `metadata.json` (contains last sync timestamps)
2. For each entity type (videos, folders, clips):
   - Fetch remote JSON if modified since last sync
   - Merge with local using conflict resolution
   - Apply soft deletes (remove items where `deleted_at` is set)
3. Update local `last_sync_at` timestamp

### Push Phase
1. Export all entities to JSON (including soft-deleted for cleanup)
2. Calculate diff: only include entities modified since last sync
3. Upload with SHA check (optimistic locking)
4. If SHA mismatch: pull first, re-merge, then push again
5. Update local `last_sync_at`

## File Structure in GitHub
```
/
├── metadata.json       # Sync metadata (last sync times, schema version)
├── videos.json        # All videos (including soft-deleted)
├── folders.json       # All folders
└── clips.json         # All clips
```

## Current Status
- **Frontend**: Settings UI is implemented in `src/lib/SettingsModal.svelte`.
- **Backend**: Core logic exists in `src-tauri/src/sync.rs` and `src-tauri/src/github_api.rs`.
- **Database**: `src-tauri/src/db.rs` supports `export_db` and `import_db` with conflict handling.

## Implementation Steps

### Step 1: Update Data Model
Add sync fields to database schema:
```sql
ALTER TABLE videos ADD COLUMN updated_at TEXT DEFAULT CURRENT_TIMESTAMP;
ALTER TABLE videos ADD COLUMN deleted_at TEXT;
ALTER TABLE videos ADD COLUMN sort_order INTEGER DEFAULT 0;
-- Repeat for folders, clips
```

### Step 2: Update CRUD Operations
Ensure all write operations:
- Set `updated_at = now()` on any modification
- Set `deleted_at = now()` on delete (soft delete)
- Update `sort_order` on reorder

### Step 3: Implement Sync Engine
Create `src-tauri/src/sync_engine.rs`:

```rust
pub struct SyncEngine {
    client: Client,
    repo_owner: String,
    repo_name: String,
    last_sync: Option<DateTime<Utc>>,
}

impl SyncEngine {
    pub async fn pull(&mut self) -> Result<SyncResult>;
    pub async fn push(&mut self) -> Result<SyncResult>;
    fn merge_entity<T: Syncable>(local: T, remote: T) -> T;
}
```

### Step 4: Implement GitHub API Updates
Add functions to `src-tauri/src/github_api.rs`:
- `get_file_sha()` - Get file SHA for optimistic locking
- `update_file()` - Update file with SHA check
- `get_directory_contents()` - List repo contents

### Step 5: Update Frontend State
In `src/lib/state.svelte.ts`:
- Track `lastSyncAt` timestamp
- Add `syncInProgress` state
- Implement conflict resolution UI

## Android Compatibility Fix (CRITICAL)
The current implementation uses `reqwest` with `rustls-tls`. On Android, `rustls` may fail to find the system's root certificates.

**Solution**: Switch to using `webpki-roots`:

```toml
[dependencies]
reqwest = { version = "0.11", default-features = false, features = ["json", "rustls-tls-webpki-roots"] }
```

## Verification

### Prerequisite: GitHub Token
1. Go to GitHub -> Settings -> Developer settings -> Personal access tokens -> Tokens (classic).
2. Generate a new token with `repo` scope.
3. Copy the token.

### Test Scenario 1: Basic Sync
1. Run the app: `bun run tauri dev`.
2. Open Settings -> GitHub Sync.
3. Enter Repo URL and Token.
4. Click "Sync Now".
5. **Verify**: Check GitHub repo for `videos.json`, `folders.json`, `clips.json`.

### Test Scenario 2: CRUD Sync
1. **Device A**: Create video "Test Video". Sync.
2. **Device B**: Sync. Should fetch "Test Video".
3. **Device B**: Rename to "Updated Video". Sync.
4. **Device A**: Sync. Should see "Updated Video".
5. **Device A**: Delete "Updated Video". Sync.
6. **Device B**: Sync. Video should be gone.

### Test Scenario 3: Sort Order
1. **Device A**: Create videos A, B, C (in order). Sync.
2. **Device B**: Sync. Should have A, B, C in order.
3. **Device B**: Reorder to C, A, B. Sync.
4. **Device A**: Sync. Should see C, A, B order.

### Test Scenario 4: Folder Movement
1. **Device A**: Create folder "Folder1", video "Video1" inside. Sync.
2. **Device B**: Sync. Video should be in Folder1.
3. **Device B**: Move Video1 to root. Sync.
4. **Device A**: Sync. Video1 should be at root.

### Test Scenario 5: Conflict Resolution
1. **Device A**: Create video "Conflict Test". Sync.
2. **Device B**: Rename to "Device B Name". Sync.
3. **Device A**: Rename to "Device A Name". Sync.
4. **Expected**: Both exist - original and "Device A Name" (or resolution strategy applied).

## Code Modifications Required

1. **`src-tauri/Cargo.toml`**:
    ```diff
    - reqwest = { version = "0.11", default-features = false, features = ["json", "rustls-tls"] }
    + reqwest = { version = "0.11", default-features = false, features = ["json", "rustls-tls-webpki-roots"] }
    ```

2. **`src-tauri/src/db.rs`**:
    - Add migration to add sync columns
    - Update CRUD functions to set timestamps
    - Add soft delete support

3. **`src-tauri/src/sync_engine.rs`** (new):
    - Implement pull/push logic
    - Implement merge with conflict resolution

4. **`src-tauri/src/github_api.rs`**:
    - Add file SHA retrieval
    - Add atomic file updates

5. **`src/lib/state.svelte.ts`**:
    - Add sync state management
    - Add conflict notification UI

6. **`src-tauri/src/lib.rs`**:
    - Register new sync commands

## Multi-Device Sync Strategy

### Device Identification
Each device must have a unique identifier stored locally:
```json
// Stored locally on each device
{
  "device_id": "uuid-v4",
  "device_name": "My Laptop",
  "first_sync_at": "ISO8601",
  "last_sync_at": "ISO8601"
}
```

### Metadata Structure (in GitHub)
```json
// metadata.json in repo
{
  "schema_version": "1.0",
  "last_full_sync": "ISO8601",
  "devices": {
    "device-uuid-1": { "name": "Laptop", "last_seen": "ISO8601" },
    "device-uuid-2": { "name": "Phone", "last_seen": "ISO8601" }
  },
  "entity_timestamps": {
    "videos": "ISO8601",
    "folders": "ISO8601",
    "clips": "ISO8601"
  }
}
```

### Sync Algorithm (Multi-Device)

#### 1. Detect Changes
- Track `local_updated_at` for each entity
- Compare against remote `updated_at`
- Build change list: created, modified, deleted

#### 2. Handle Concurrent Modifications
```
Device A (Laptop)                    Device B (Phone)
     |                                     |
     |-- Create Video 1 (t=10) ---------->|
     |                                     |-- Create Video 1 (t=11) --+
     |                                     |                         |
     |                                     |<----- Conflict! ---------+
     |                                     |
     |-- Sync at t=12                     |-- Sync at t=13
     |   - Pull: sees Video 1 from B         - Pull: sees Video 1 from A
     |   - Merge: Keep both (conflict)       - Merge: Keep both (conflict)
```

#### 3. Conflict Resolution Rules
| Scenario | Resolution |
|----------|------------|
| Same entity modified on both devices | Keep newer `updated_at`, add suffix to older |
| Same entity deleted on both devices | Permanently delete |
| Entity deleted on one device, modified on other | Keep modified version (undelete) |
| Different entities with same ID | Keep both, rename local duplicate |
| Sort order conflicts | Use highest `sort_order` (most recent user action wins) |

### Optimistic Locking Flow
```
1. Fetch remote file + SHA
2. Calculate local changes
3. Merge changes with remote
4. Attempt push with SHA
5. If SHA changed (another device pushed):
   a. Pull latest
   b. Re-merge with fresh timestamps
   c. Retry push (max 3 attempts)
```

### Offline Queue
When offline, queue changes locally:
```sql
-- Local queue table
CREATE TABLE sync_queue (
  id INTEGER PRIMARY KEY,
  entity_type TEXT,  -- 'video', 'folder', 'clip'
  entity_id TEXT,
  operation TEXT,    -- 'create', 'update', 'delete'
  payload TEXT,      -- JSON of entity state
  created_at TEXT,
  attempted_at TEXT,
  status TEXT        -- 'pending', 'processing', 'failed'
);
```
- Process queue in order when online
- Retry with exponential backoff on failure

### Sync Triggers
- **Manual**: User clicks "Sync Now"
- **On App Start**: Check if last sync > 5 minutes ago
- **On App Resume**: Auto-sync if backgrounded > 10 minutes
- **On Network Change**: Auto-sync when connection restored
- **Periodic**: Every 15 minutes if app is open

### Data Verification
After each sync, verify integrity:
1. Check all parent_id references exist
2. Verify no orphaned clips
3. Validate folder hierarchy
4. Reconcile sort_order gaps

### Device Cleanup
- If device not seen for 30 days, mark as "inactive"
- Inactive device data remains (can be reactivated)
- Manual option to "Disconnect Device" clears local data

## Conclusion
This plan provides robust, intelligent sync that handles:
- Full CRUD operations with proper timestamps
- Sort order preservation across devices
- Folder hierarchy sync
- Conflict detection and resolution
- Offline-first with eventual consistency
