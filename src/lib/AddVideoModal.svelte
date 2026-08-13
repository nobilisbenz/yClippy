<script lang="ts">
    import { appState } from "./state.svelte";
    import { saveVideo, fetchVideoOembed, type Video } from "./db";

    let { folderId = null } = $props<{ folderId?: number | null }>();

    let url = $state("");
    let customTitle = $state("");
    let startTimeStr = $state("");
    let endTimeStr = $state("");
    let loading = $state(false);
    let error = $state("");

    function extractVideoId(input: string): string | null {
        const trimmed = input.trim();
        if (/^[a-zA-Z0-9_-]{11}$/.test(trimmed)) return trimmed;
        try {
            const urlObj = new URL(trimmed);
            const host = urlObj.hostname.replace(/^www\./, "").replace(/^m\./, "");
            if (host === "youtu.be") {
                const id = urlObj.pathname.slice(1).split("/")[0];
                return /^[a-zA-Z0-9_-]{11}$/.test(id) ? id : null;
            }
            if (host.endsWith("youtube.com") || host.endsWith("youtube-nocookie.com")) {
                const v = urlObj.searchParams.get("v");
                if (v && /^[a-zA-Z0-9_-]{11}$/.test(v)) return v;
                const parts = urlObj.pathname.split("/").filter(Boolean);
                const idx = parts.findIndex((p) => ["embed", "v", "shorts", "live"].includes(p));
                if (idx >= 0 && parts[idx + 1] && /^[a-zA-Z0-9_-]{11}$/.test(parts[idx + 1])) {
                    return parts[idx + 1];
                }
            }
        } catch {
            // not a URL
        }
        return null;
    }

    async function handleSubmit() {
        loading = true;
        error = "";
        try {
            const videoId = extractVideoId(url);
            if (!videoId) throw new Error("Invalid YouTube URL or ID");

            let title = customTitle.trim();

            if (!title) {
                try {
                    const oembed = await fetchVideoOembed(videoId);
                    if (oembed?.title) {
                        title = oembed.title;
                    } else {
                        title = `Video ${videoId}`;
                    }
                } catch {
                    title = `Video ${videoId}`;
                }
            }

            const start = Math.floor(parseFloat(startTimeStr) || 0);
            const end = Math.floor(parseFloat(endTimeStr) || 0);

            const video: Video = {
                id: videoId,
                title: title,
                thumbnail_url: "",
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
