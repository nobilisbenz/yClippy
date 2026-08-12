<script lang="ts">
    import { onMount } from "svelte";
    import { fetchVideoOembed, saveVideo, type Video, type VideoOembed } from "./db";
    import { appState } from "./state.svelte";

    let { videoId, onClose }: { videoId: string; onClose: () => void } = $props();

    let metadata = $state<VideoOembed | null>(null);
    let loading = $state(true);
    let saving = $state(false);
    let error = $state<string | null>(null);

    onMount(async () => {
        try {
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

    async function save() {
        if (!metadata) return;
        saving = true;
        try {
            const video: Video = {
                id: metadata.video_id,
                title: metadata.title,
                thumbnail_url: metadata.thumbnail_url,
                duration: 0,
                last_position: 0,
                created_at: Date.now(),
                folder_id: null,
                start_time: 0,
                end_time: 0,
                sort_order: 0,
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
            <h2 class="text-lg font-semibold text-white">Save shared video?</h2>
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
                    {#if metadata.thumbnail_url}
                        <img
                            src={metadata.thumbnail_url}
                            alt=""
                            class="w-32 h-20 object-cover rounded bg-zinc-800 shrink-0"
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
                {saving ? "Saving…" : "Save to library"}
            </button>
        </div>
    </div>
</div>
