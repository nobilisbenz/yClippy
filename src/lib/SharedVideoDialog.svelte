<script lang="ts">
    import { onMount } from "svelte";
    import { fetchVideoOembed, saveVideo, getVideos, type Video, type VideoOembed } from "./db";
    import { appState } from "./state.svelte";
    import Thumbnail from "./Thumbnail.svelte";
    import FolderPicker from "./FolderPicker.svelte";

    let { videoId, onClose }: { videoId: string; onClose: () => void } = $props();

    let metadata = $state<VideoOembed | null>(null);
    let loading = $state(true);
    let saving = $state(false);
    let error = $state<string | null>(null);
    let existingVideo = $state<Video | null>(null);
    let isAlreadyInLibrary = $derived(existingVideo !== null);
    let pickedFolderId = $state<number | null>(null);
    let isFolderPickerOpen = $state(false);
    let folderPickerRef = $state<{
        setSelectedVideo: (v: Video) => void;
        getPickedFolder: () => number | null;
    } | undefined>();

    onMount(async () => {
        try {
            const allVideos = await getVideos();
            existingVideo = allVideos.find((v) => v.id === videoId) ?? null;
            metadata = await fetchVideoOembed(videoId);
            if (!metadata) {
                error = "Could not fetch video info";
            }
        } catch (e) {
            error = String(e);
        } finally {
            loading = false;
        }
    });

    function folderName(id: number | null): string {
        if (id === null) return "Root";
        return appState.folders.find((f) => f.id === id)?.name ?? "Root";
    }

    async function save() {
        if (!metadata) return;
        saving = true;
        try {
            const now = Date.now();
            const folderId = pickedFolderId !== null ? pickedFolderId : existingVideo?.folder_id ?? null;
            const video: Video = {
                id: metadata.video_id,
                title: metadata.title || existingVideo?.title || `Video ${metadata.video_id}`,
                thumbnail_url: metadata.thumbnail_url || existingVideo?.thumbnail_url || "",
                duration: existingVideo?.duration ?? 0,
                last_position: existingVideo?.last_position ?? 0,
                created_at: existingVideo?.created_at ?? now,
                folder_id: folderId,
                start_time: existingVideo?.start_time ?? 0,
                end_time: existingVideo?.end_time ?? 0,
                sort_order: existingVideo?.sort_order ?? 0,
            };
            await saveVideo(video);
            await appState.refreshVideos();
            onClose();
        } catch (e) {
            error = String(e);
            saving = false;
        }
    }
</script>

<div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4"
    role="presentation"
>
    <div
        class="bg-zinc-900 rounded-lg shadow-2xl border border-zinc-700 max-w-md w-full overflow-hidden"
    >
        <div class="p-4 border-b border-zinc-700 flex items-center gap-3">
            <svg
                class="size-5 text-blue-400 shrink-0"
                fill="currentColor"
                viewBox="0 0 24 24"
            >
                <path d="M10 9V15L15 12L10 9M12 2C6.48 2 2 6.48 2 12C2 17.52 6.48 22 12 22C17.52 22 22 17.52 22 12C22 6.48 17.52 2 12 2M12 20C7.59 20 4 16.41 4 12C4 7.59 7.59 4 12 4C16.41 4 20 7.59 20 12C20 16.41 16.41 20 12 20Z" />
            </svg>
            <h2 class="text-lg font-semibold text-white">
                {isAlreadyInLibrary ? "Already in library" : "Save shared video?"}
            </h2>
        </div>

        <div class="p-4">
            {#if loading}
                <div class="flex items-center gap-3 py-6 justify-center text-zinc-400">
                    <div
                        class="animate-spin rounded-full h-5 w-5 border-b-2 border-white"
                    ></div>
                    <span>Loading video info…</span>
                </div>
            {:else if error}
                <div class="text-red-400 text-sm py-2">{error}</div>
                <div class="text-xs text-zinc-500 font-mono break-all mt-1">{videoId}</div>
            {:else if metadata}
                <div class="flex gap-3">
                    {#if metadata.thumbnail_url || metadata.video_id}
                        <Thumbnail
                            videoId={metadata.video_id}
                            alt=""
                            className="w-32 h-20 object-cover rounded bg-zinc-800 shrink-0"
                        />
                    {/if}
                    <div class="min-w-0 flex-1">
                        <div class="text-white font-medium line-clamp-2">
                            {metadata.title}
                        </div>
                        {#if metadata.author}
                            <div class="text-sm text-zinc-400 mt-1">{metadata.author}</div>
                        {/if}
                        <div class="text-xs text-zinc-500 font-mono mt-1 truncate">
                            {metadata.video_id}
                        </div>
                    </div>
                </div>
                <button
                    onclick={() => (isFolderPickerOpen = true)}
                    class="mt-3 w-full px-3 py-2 bg-zinc-800 hover:bg-zinc-700 text-sm text-left rounded flex items-center justify-between"
                >
                    <span class="text-zinc-400">Folder</span>
                    <span class="text-white">{folderName(pickedFolderId !== null ? pickedFolderId : existingVideo?.folder_id ?? null)}</span>
                </button>
            {/if}
        </div>

        <div class="p-3 border-t border-zinc-700 flex justify-end gap-2">
            <button
                onclick={onClose}
                disabled={saving}
                class="px-4 py-2 bg-zinc-700 hover:bg-zinc-600 disabled:opacity-50 text-white rounded text-sm transition-colors"
            >
                Cancel
            </button>
            <button
                onclick={save}
                disabled={saving || loading || !!error}
                class="px-4 py-2 bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white rounded text-sm transition-colors"
            >
                {saving ? "Saving…" : isAlreadyInLibrary ? "Refresh metadata" : "Save to library"}
            </button>
        </div>

        <FolderPicker
            bind:this={folderPickerRef}
            open={isFolderPickerOpen}
            title="Choose a folder"
            onClose={() => {
                if (folderPickerRef) {
                    const picked = folderPickerRef.getPickedFolder();
                    if (picked !== null) pickedFolderId = picked;
                }
                isFolderPickerOpen = false;
            }}
        />
    </div>
</div>
