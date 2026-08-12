<script lang="ts">
    import { appState } from "./state.svelte";
    import { saveVideo, type Video } from "./db";

    let { folderId = null } = $props<{ folderId?: number | null }>();

    let url = $state("");
    let customTitle = $state("");
    let startTimeStr = $state("");
    let endTimeStr = $state("");
    let loading = $state(false);
    let error = $state("");

    async function handleSubmit() {
        loading = true;
        error = "";
        try {
            let videoId = "";
            try {
                const urlObj = new URL(url);
                if (urlObj.hostname.includes("youtube.com")) {
                    videoId = urlObj.searchParams.get("v") || "";
                } else if (urlObj.hostname.includes("youtu.be")) {
                    videoId = urlObj.pathname.slice(1);
                }
            } catch (e) {
                if (url.length === 11) videoId = url;
            }

            if (!videoId) throw new Error("Invalid URL");

            let title = customTitle.trim();
            let thumbnail_url = `https://img.youtube.com/vi/${videoId}/maxresdefault.jpg`;

            if (!title) {
                try {
                    const response = await fetch(
                        `https://noembed.com/embed?url=https://www.youtube.com/watch?v=${videoId}`,
                    );
                    if (!response.ok) {
                        throw new Error(`HTTP error: ${response.status}`);
                    }
                    const data = await response.json();
                    if (!data.title)
                        throw new Error("Could not fetch video metadata");
                    title = data.title;
                    thumbnail_url = data.thumbnail_url || thumbnail_url;
                } catch (fetchErr: any) {
                    title = `Video ${videoId}`;
                }
            }

            const start = Math.floor(parseFloat(startTimeStr) || 0);
            const end = Math.floor(parseFloat(endTimeStr) || 0);

            const video: Video = {
                id: videoId,
                title: title,
                thumbnail_url: thumbnail_url,
                duration: 0,
                last_position: 0,
                created_at: Date.now(),
                folder_id: folderId,
                start_time: start,
                end_time: end,
                sort_order: 0,
            };

            await saveVideo(video);
            await appState.refreshVideos();
            close();
        } catch (e: any) {
            error = e.message || String(e);
        } finally {
            loading = false;
        }
    }

    function close() {
        appState.isAddVideoModalOpen = false;
        url = "";
        customTitle = "";
        startTimeStr = "";
        endTimeStr = "";
    }
</script>

<div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm"
>
    <div
        class="w-full max-w-md bg-zinc-900 border border-zinc-800 rounded-xl p-6 shadow-2xl"
    >
        <h2 class="text-xl font-bold text-white mb-4">Add Video</h2>

        <form
            onsubmit={(e) => {
                e.preventDefault();
                handleSubmit();
            }}
        >
            <div class="mb-4">
                <label
                    class="block text-sm font-medium text-zinc-400 mb-1"
                    for="url">YouTube URL</label
                >
                <input
                    type="text"
                    id="url"
                    bind:value={url}
                    placeholder="https://youtube.com/watch?v=..."
                    class="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-blue-600 transition"
                />
            </div>

            <div class="mb-4">
                <label
                    class="block text-sm font-medium text-zinc-400 mb-1"
                    for="title">Title (Optional)</label
                >
                <input
                    type="text"
                    id="title"
                    bind:value={customTitle}
                    placeholder="Custom Title"
                    class="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-blue-600 transition"
                />
            </div>

            <div class="flex gap-4 mb-4">
                <div class="flex-1">
                    <label
                        class="block text-sm font-medium text-zinc-400 mb-1"
                        for="start">Start (Optional)</label
                    >
                    <input
                        type="number"
                        id="start"
                        bind:value={startTimeStr}
                        placeholder="0"
                        class="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-blue-600 transition"
                    />
                </div>
                <div class="flex-1">
                    <label
                        class="block text-sm font-medium text-zinc-400 mb-1"
                        for="end">End (Optional)</label
                    >
                    <input
                        type="number"
                        id="end"
                        bind:value={endTimeStr}
                        placeholder="0"
                        class="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-blue-600 transition"
                    />
                </div>
            </div>

            {#if error}
                <p class="text-red-500 text-sm mb-4">{error}</p>
            {/if}

            <div class="flex justify-end gap-3">
                <button
                    type="button"
                    onclick={close}
                    class="px-4 py-2 rounded-lg hover:bg-zinc-800 text-zinc-300 transition"
                    >Cancel</button
                >
                <button
                    type="submit"
                    disabled={loading}
                    class="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded-lg text-white font-medium disabled:opacity-50 transition"
                >
                    {loading ? "Adding..." : "Add Video"}
                </button>
            </div>
        </form>
    </div>
</div>
