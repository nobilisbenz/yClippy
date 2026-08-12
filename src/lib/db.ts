import { invoke } from "@tauri-apps/api/core";

export function formatTime(totalSeconds: number): string {
    const h = Math.floor(totalSeconds / 3600);
    const m = Math.floor((totalSeconds % 3600) / 60);
    const s = Math.floor(totalSeconds % 60);
    return `${h}:${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
}

let changeListener: (() => void) | null = null;

function notifyChange(): void {
    if (changeListener) {
        changeListener();
    }
}

export interface Video {
    id: string;
    title: string;
    thumbnail_url: string;
    duration: number;
    last_position: number;
    created_at: number;
    folder_id: number | null;
    start_time: number;
    end_time: number;
    sort_order: number;
}

export interface Clip {
    id?: number;
    video_id: string;
    start_time: number;
    end_time: number;
    title: string;
    created_at: number;
    sort_order: number;
}

export interface Folder {
    id?: number;
    name: string;
    created_at: number;
    parent_id: number | null;
    sort_order: number;
}

export async function getVideos(): Promise<Video[]> {
    return await invoke("get_videos");
}

export async function saveVideo(video: Video): Promise<void> {
    await invoke("save_video", { video });
    notifyChange();
}

export async function getClips(videoId: string): Promise<Clip[]> {
    return await invoke("get_clips", { videoId });
}

export async function saveClip(clip: Clip): Promise<void> {
    await invoke("save_clip", { clip });
    notifyChange();
}

export async function deleteClip(id: number): Promise<void> {
    await invoke("delete_clip", { id });
    notifyChange();
}

export async function deleteVideo(id: string): Promise<void> {
    await invoke("delete_video", { id });
    notifyChange();
}

export async function getFolders(): Promise<Folder[]> {
    return await invoke("get_folders");
}

export async function saveFolder(folder: Folder): Promise<number> {
    const result = await invoke<number>("save_folder", { folder });
    notifyChange();
    return result;
}

export async function deleteFolder(id: number): Promise<void> {
    await invoke("delete_folder", { id });
    notifyChange();
}

export async function renameFolder(id: number, name: string): Promise<void> {
    await invoke("rename_folder", { id, name });
    notifyChange();
}

export async function renameVideo(id: string, title: string): Promise<void> {
    await invoke("rename_video", { id, title });
    notifyChange();
}

export async function renameClip(id: number, title: string): Promise<void> {
    await invoke("rename_clip", { id, title });
    notifyChange();
}

export async function updateClip(clip: Clip): Promise<void> {
    await invoke("update_clip", { clip });
    notifyChange();
}

export async function updateVideoFolder(videoId: string, folderId: number | null): Promise<void> {
    await invoke("update_video_folder", { videoId, folderId });
    notifyChange();
}

export async function updateFolderParent(folderId: number, parentId: number | null): Promise<void> {
    await invoke("update_folder_parent", { folderId, parentId });
    notifyChange();
}

export interface Backup {
    folders: Folder[];
    videos: Video[];
    clips: Clip[];
}

export async function exportDb(): Promise<Backup> {
    return await invoke("export_db");
}

export async function importDb(backup: Backup): Promise<void> {
    await invoke("import_db", { backup });
}

export async function getDbPath(): Promise<string> {
    return await invoke("get_db_path");
}

export async function setDbPath(path: string): Promise<void> {
    await invoke("set_db_path", { path });
}

export async function updateVideoMetadata(id: string, title: string, startTime: number, endTime: number): Promise<void> {
    await invoke("update_video_metadata", { id, title, startTime, endTime });
}

export async function updateSortOrder(
    folders: { id: number; sort_order: number }[],
    videos: { id: string; sort_order: number }[],
): Promise<void> {
    await invoke("update_sort_order", { folders, videos });
    notifyChange();
}

export async function updateClipSortOrder(
    clips: { id: number; sort_order: number }[],
): Promise<void> {
    await invoke("update_clip_sort_order", { clips });
    notifyChange();
}

export async function setChangeListener(listener: () => void): Promise<void> {
    changeListener = listener;
}

export interface SyncMetadata {
    last_sync_timestamp: number;
    last_sync_device: string;
}

export interface Change {
    id?: number;
    entity_type: string;
    entity_id: string;
    change_type: 'Create' | 'Update' | 'Delete';
    data?: string;
    timestamp: number;
    device_id: string;
    synced: boolean;
}

export interface ChangeSet {
    changes: Change[];
    metadata: SyncMetadata;
    device_id: string;
}

export async function getDeviceId(): Promise<string> {
    return await invoke("get_device_id");
}

export async function getSyncStatus(): Promise<SyncMetadata> {
    return await invoke("get_sync_status");
}

export async function getPendingChangesCount(): Promise<number> {
    return await invoke("get_pending_changes_count");
}

export async function exportChanges(since: number): Promise<ChangeSet> {
    return await invoke("export_changes", { since });
}

export async function importChanges(changeset: ChangeSet): Promise<void> {
    await invoke("import_changes", { changeset });
}

export async function markChangesSynced(): Promise<void> {
    await invoke("mark_changes_synced");
}

export async function recordChange(
    entityType: string,
    entityId: string,
    changeType: string,
    data?: string
): Promise<void> {
    await invoke("record_change", { entityType, entityId, changeType, data });
}

export async function resetSync(): Promise<void> {
    await invoke("reset_sync");
}

export interface VideoOembed {
    video_id: string;
    title: string;
    author: string;
    thumbnail_url: string;
}

export async function fetchVideoOembed(videoId: string): Promise<VideoOembed | null> {
    return await invoke("fetch_video_oembed", { videoId });
}
