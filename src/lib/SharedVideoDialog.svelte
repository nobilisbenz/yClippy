<script lang="ts">
    import { onMount } from "svelte";
    import { fetchVideoOembed, saveVideo, getVideos, type Video, type VideoOembed } from "./db";
    import { appState } from "./state.svelte";
    import FolderPicker from "./FolderPicker.svelte";
    import Icon from "./Icon.svelte";
    import Modal from "./Modal.svelte";
    import Thumbnail from "./Thumbnail.svelte";
    import { formatClock } from "./youtube.svelte";

    let { videoId, onClose }: { videoId: string; onClose: () => void } = $props();

    let metadata = $state<VideoOembed | null>(null);
    let loading = $state(true);
    let saving = $state(false);
    let error = $state<string | null>(null);
    let existingVideo = $state<Video | null>(null);
    let isAlreadyInLibrary = $derived(existingVideo !== null);
    let pickedFolderId = $state<number | null>(null);
    let isFolderPickerOpen = $state(false);
    let folderPickerRef = $state<
        | {
              setSelectedVideo: (v: Video) => void;
              getPickedFolder: () => number | null;
          }
        | undefined
    >();

    onMount(async () => {
        try {
            const allVideos = await getVideos();
            existingVideo = allVideos.find((v) => v.id === videoId) ?? null;
            metadata = await fetchVideoOembed(videoId);
            if (!metadata) error = "Could not fetch video info";
        } catch (e) {
            error = String(e);
        } finally {
            loading = false;
        }
    });

    const targetFolderId = $derived(
        pickedFolderId !== null ? pickedFolderId : (existingVideo?.folder_id ?? null),
    );

    function folderName(id: number | null): string {
        if (id === null) return "Library";
        return appState.folders.find((f) => f.id === id)?.name ?? "Library";
    }

    /// Everything the library already knows about this video is carried over.
    /// Sharing a video you already have used to reset its watch position, its
    /// folder and its trim to zero.
    async function save() {
        if (!metadata) return;
        saving = true;
        try {
            const now = Date.now();
            const video: Video = {
                id: metadata.video_id,
                title: metadata.title || existingVideo?.title || `Video ${metadata.video_id}`,
                thumbnail_url: metadata.thumbnail_url || existingVideo?.thumbnail_url || "",
                duration: existingVideo?.duration ?? 0,
                last_position: existingVideo?.last_position ?? 0,
                created_at: existingVideo?.created_at ?? now,
                folder_id: targetFolderId,
                start_time: existingVideo?.start_time ?? 0,
                end_time: existingVideo?.end_time ?? 0,
                sort_order: existingVideo?.sort_order ?? 0,
            };
            await saveVideo(video);
            await appState.refreshVideos();
            appState.showToast(isAlreadyInLibrary ? "Video updated" : "Saved to library", "success");
            onClose();
        } catch (e) {
            error = String(e);
            saving = false;
        }
    }
</script>

<Modal title={isAlreadyInLibrary ? "Already in your library" : "Save shared video"} onClose={onClose}>
    {#if loading}
        <div class="flex items-center gap-3 py-8 justify-center text-[color:var(--text-dim)]">
            <span class="animate-spin"><Icon name="sync" size={16} /></span>
            <span class="text-sm">Loading video info…</span>
        </div>
    {:else if error}
        <p class="text-sm" style="color: var(--danger)">{error}</p>
        <p class="text-xs t-num text-[color:var(--text-faint)] mt-1 break-all">{videoId}</p>
    {:else if metadata}
        <div class="flex gap-3">
            <Thumbnail
                videoId={metadata.video_id}
                alt=""
                className="w-32 h-[72px] object-cover rounded-[4px] bg-black shrink-0"
            />
            <div class="min-w-0 flex-1">
                <div class="text-sm text-[color:var(--text)] line-clamp-2 leading-snug">
                    {metadata.title}
                </div>
                {#if metadata.author}
                    <div class="text-xs text-[color:var(--text-dim)] mt-1">{metadata.author}</div>
                {/if}
                {#if existingVideo && existingVideo.last_position > 0}
                    <div class="text-[11px] t-num text-[color:var(--text-faint)] mt-1 flex items-center gap-1">
                        <Icon name="play" size={9} />
                        watched to {formatClock(existingVideo.last_position)}
                    </div>
                {/if}
            </div>
        </div>

        <button
            onclick={() => (isFolderPickerOpen = true)}
            class="row mt-4 rounded-[6px] border border-[color:var(--border)] py-2"
        >
            <Icon name="folder" size={15} />
            <span class="flex-1 text-left text-[13px]">Folder</span>
            <span class="text-[13px] text-[color:var(--text)]">{folderName(targetFolderId)}</span>
            <Icon name="chevronRight" size={14} />
        </button>
    {/if}

    {#snippet footer()}
        <button class="btn btn-ghost" onclick={onClose} disabled={saving}>Cancel</button>
        <button class="btn btn-primary" onclick={save} disabled={saving || loading || !!error}>
            {saving ? "Saving…" : isAlreadyInLibrary ? "Refresh metadata" : "Save to library"}
        </button>
    {/snippet}
</Modal>

<FolderPicker
    bind:this={folderPickerRef}
    open={isFolderPickerOpen}
    title="Choose a folder"
    pickOnly
    onClose={() => {
        const picked = folderPickerRef?.getPickedFolder();
        if (picked !== null && picked !== undefined) pickedFolderId = picked;
        isFolderPickerOpen = false;
    }}
/>
