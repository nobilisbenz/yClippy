<script lang="ts">
    import { appState } from "./state.svelte";
    import { saveVideo, fetchVideoOembed, getVideos, type Video } from "./db";
    import Modal from "./Modal.svelte";
    import Thumbnail from "./Thumbnail.svelte";

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

    /// The id is recognised as you type, so a bad paste is obvious before you
    /// press the button and the thumbnail confirms you got the right video.
    const previewId = $derived(extractVideoId(url));

    const folderLabel = $derived(
        folderId === null
            ? "Library"
            : (appState.folders.find((f) => f.id === folderId)?.name ?? "Library"),
    );

    /// Accepts `1:23`, `1:02:03` and plain seconds — a timestamp copied off
    /// YouTube is never a bare number.
    function parseTime(input: string): number {
        const trimmed = input.trim();
        if (!trimmed) return 0;
        if (trimmed.includes(":")) {
            const parts = trimmed.split(":").map((p) => parseFloat(p) || 0);
            return Math.floor(parts.reduce((total, part) => total * 60 + part, 0));
        }
        return Math.floor(parseFloat(trimmed) || 0);
    }

    async function handleSubmit() {
        loading = true;
        error = "";
        try {
            const videoId = extractVideoId(url);
            if (!videoId) throw new Error("That is not a YouTube link or video id.");

            const existing = (await getVideos()).find((v) => v.id === videoId);
            let title = customTitle.trim();
            if (!title) {
                try {
                    const oembed = await fetchVideoOembed(videoId);
                    title = oembed?.title || existing?.title || `Video ${videoId}`;
                } catch {
                    title = existing?.title || `Video ${videoId}`;
                }
            }

            // Re-adding a video you already have keeps its watch position and
            // its clips instead of resetting them to zero.
            const video: Video = {
                id: videoId,
                title,
                thumbnail_url: existing?.thumbnail_url ?? "",
                duration: existing?.duration ?? 0,
                last_position: existing?.last_position ?? 0,
                created_at: existing?.created_at ?? Date.now(),
                folder_id: existing ? existing.folder_id : folderId,
                start_time: parseTime(startTimeStr) || existing?.start_time || 0,
                end_time: parseTime(endTimeStr) || existing?.end_time || 0,
                sort_order: existing?.sort_order ?? 0,
            };

            await saveVideo(video);
            await appState.refreshVideos();
            appState.showToast(existing ? "Video updated" : `Added “${title}”`, "success");
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
        error = "";
    }
</script>

<Modal title="Add a video" onClose={close}>
    <form
        id="add-video-form"
        onsubmit={(e) => {
            e.preventDefault();
            handleSubmit();
        }}
        class="flex flex-col gap-4"
    >
        <div>
            <label class="label" for="url">YouTube link or id</label>
            <!-- svelte-ignore a11y_autofocus -->
            <input
                type="text"
                id="url"
                autofocus
                bind:value={url}
                placeholder="https://www.youtube.com/watch?v=…"
                class="field"
            />
        </div>

        {#if previewId}
            <div class="flex items-center gap-3 p-2 rounded-[6px] bg-[color:var(--bg)] border border-[color:var(--border)]">
                <Thumbnail
                    videoId={previewId}
                    alt=""
                    className="w-24 h-[54px] object-cover rounded-[4px] bg-black shrink-0"
                />
                <div class="min-w-0 text-xs">
                    <div class="t-num text-[color:var(--text-dim)] truncate">{previewId}</div>
                    <div class="text-[color:var(--text-faint)] mt-0.5">
                        Saving to <span class="text-[color:var(--text-dim)]">{folderLabel}</span>
                    </div>
                </div>
            </div>
        {/if}

        <div>
            <label class="label" for="title">Title <span class="text-[color:var(--text-faint)]">(optional)</span></label>
            <input
                type="text"
                id="title"
                bind:value={customTitle}
                placeholder="Taken from YouTube if left blank"
                class="field"
            />
        </div>

        <div class="flex gap-3">
            <div class="flex-1">
                <label class="label" for="start">Start</label>
                <input type="text" id="start" bind:value={startTimeStr} placeholder="0:00" class="field t-num" />
            </div>
            <div class="flex-1">
                <label class="label" for="end">End</label>
                <input type="text" id="end" bind:value={endTimeStr} placeholder="end of video" class="field t-num" />
            </div>
        </div>

        {#if error}
            <p class="text-sm" style="color: var(--danger)">{error}</p>
        {/if}
    </form>

    {#snippet footer()}
        <button type="button" class="btn btn-ghost" onclick={close}>Cancel</button>
        <button
            type="submit"
            form="add-video-form"
            class="btn btn-primary"
            disabled={loading || !url.trim()}
        >
            {loading ? "Adding…" : "Add video"}
        </button>
    {/snippet}
</Modal>
