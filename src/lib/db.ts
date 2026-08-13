import { invoke } from "@tauri-apps/api/core";

export function formatTime(totalSeconds: number): string {
    const h = Math.floor(totalSeconds / 3600);
    const m = Math.floor((totalSeconds % 3600) / 60);
    const s = Math.floor(totalSeconds % 60);
    return `${h}:${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
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
    uid?: string;
    video_id: string;
    start_time: number;
    end_time: number;
    title: string;
    created_at: number;
    sort_order: number;
}

export interface Folder {
    id?: number;
    uid?: string;
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
}

export async function getClips(videoId: string): Promise<Clip[]> {
    return await invoke("get_clips", { videoId });
}

export async function saveClip(clip: Clip): Promise<void> {
    await invoke("save_clip", { clip });
}

export async function deleteClip(id: number): Promise<void> {
    await invoke("delete_clip", { id });
}

export async function deleteVideo(id: string): Promise<void> {
    await invoke("delete_video", { id });
}

export async function getFolders(): Promise<Folder[]> {
    return await invoke("get_folders");
}

export async function saveFolder(folder: Folder): Promise<number> {
    const result = await invoke<number>("save_folder", { folder });
    return result;
}

export async function deleteFolder(id: number): Promise<void> {
    await invoke("delete_folder", { id });
}

export async function renameFolder(id: number, name: string): Promise<void> {
    await invoke("rename_folder", { id, name });
}

export async function renameVideo(id: string, title: string): Promise<void> {
    await invoke("rename_video", { id, title });
}

export async function renameClip(id: number, title: string): Promise<void> {
    await invoke("rename_clip", { id, title });
}

export async function updateClip(clip: Clip): Promise<void> {
    await invoke("update_clip", { clip });
}

export async function updateVideoFolder(videoId: string, folderId: number | null): Promise<void> {
    await invoke("update_video_folder", { videoId, folderId });
}

export async function updateFolderParent(folderId: number, parentId: number | null): Promise<void> {
    await invoke("update_folder_parent", { folderId, parentId });
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
}

export async function updateClipSortOrder(
    clips: { id: number; sort_order: number }[],
): Promise<void> {
    await invoke("update_clip_sort_order", { clips });
}

export async function restoreVideo(id: string): Promise<void> {
    await invoke("restore_video", { id });
}

export async function restoreFolder(id: number): Promise<void> {
    await invoke("restore_folder", { id });
}

export async function restoreClip(id: number): Promise<void> {
    await invoke("restore_clip", { id });
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

export interface GithubConfigPublic {
    github_repo: string;
    token_present: boolean;
    vault_path: string;
}

export async function getGithubConfig(): Promise<GithubConfigPublic> {
    return await invoke("get_github_config");
}

export async function setGithubConfig(
    githubRepo: string,
    githubToken: string | null,
    vaultPath?: string | null,
): Promise<void> {
    await invoke("set_github_config", { githubRepo, githubToken, vaultPath });
}

export async function clearGithubToken(): Promise<void> {
    await invoke("clear_github_token");
}

// Pulling, merging, pushing, and compaction are one operation now — splitting
// them let a caller compact before it had pulled, which dropped changes that
// had never been applied. `start_github_sync` is the whole cycle.

export interface PlayRequest {
    url: string;
    video_id: string;
    at_seconds: number | null;
    end_seconds: number | null;
    folder: string | null;
    title: string | null;
    open: boolean;
}

/// Drains a `yclippy play` that arrived before this webview was listening.
export async function takePendingPlay(): Promise<PlayRequest | null> {
    return await invoke<PlayRequest | null>("take_pending_play");
}

export interface PickerItem {
    kind: "video" | "clip";
    video_id: string;
    url: string;
    title: string;
    thumbnail_url: string;
    start_seconds: number;
    end_seconds: number;
    last_position: number;
    clip_uid: string | null;
    clip_count: number;
}

export async function listForPicker(
    query?: string,
    limit?: number,
): Promise<PickerItem[]> {
    return await invoke<PickerItem[]>("list_videos_for_picker", { query, limit });
}
