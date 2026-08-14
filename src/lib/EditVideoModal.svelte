<script lang="ts">
    import { appState } from "./state.svelte";
    import { updateVideoMetadata } from "./db";
    import { formatClock } from "./youtube.svelte";
    import Modal from "./Modal.svelte";

    let title = $state(appState.videoToEdit?.title || "");
    let startTime = $state(appState.videoToEdit?.start_time || 0);
    let endTime = $state(appState.videoToEdit?.end_time || 0);
    let saving = $state(false);

    async function save() {
        const video = appState.videoToEdit;
        if (!video?.id) return;
        saving = true;
        try {
            await updateVideoMetadata(video.id, title, Math.max(0, startTime), Math.max(0, endTime));
            await appState.refreshVideos();
            appState.showToast("Video updated", "success");
            close();
        } catch (e) {
            appState.showToast(`Could not save: ${String(e)}`, "error");
        } finally {
            saving = false;
        }
    }

    function close() {
        appState.isEditVideoModalOpen = false;
        appState.videoToEdit = null;
    }
</script>

<Modal title="Edit video" onClose={close}>
    <form
        id="edit-video-form"
        onsubmit={(e) => {
            e.preventDefault();
            save();
        }}
        class="flex flex-col gap-4"
    >
        <div>
            <label class="label" for="edit-title">Title</label>
            <input type="text" id="edit-title" bind:value={title} class="field" />
        </div>

        <div class="flex gap-3">
            <div class="flex-1">
                <label class="label" for="edit-start">Start (seconds)</label>
                <input type="number" min="0" id="edit-start" bind:value={startTime} class="field t-num" />
            </div>
            <div class="flex-1">
                <label class="label" for="edit-end">End (seconds)</label>
                <input type="number" min="0" id="edit-end" bind:value={endTime} class="field t-num" />
            </div>
        </div>

        <p class="text-xs text-[color:var(--text-faint)]">
            Playback is trimmed to {formatClock(startTime)} –
            {endTime > 0 ? formatClock(endTime) : "the end of the video"}.
            Set the end to 0 to play to the end.
        </p>
    </form>

    {#snippet footer()}
        <button type="button" class="btn btn-ghost" onclick={close}>Cancel</button>
        <button
            type="submit"
            form="edit-video-form"
            class="btn btn-primary"
            disabled={saving || !title.trim()}
        >
            {saving ? "Saving…" : "Save"}
        </button>
    {/snippet}
</Modal>
