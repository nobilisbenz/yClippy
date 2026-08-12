<script lang="ts">
    import { appState } from "./state.svelte";
    import { updateVideoMetadata } from "./db";

    let title = $state(appState.videoToEdit?.title || "");
    let startTime = $state(appState.videoToEdit?.start_time || 0);
    let endTime = $state(appState.videoToEdit?.end_time || 0);

    async function save() {
        if (appState.videoToEdit && appState.videoToEdit.id !== undefined) {
            await updateVideoMetadata(
                appState.videoToEdit.id,
                title,
                startTime,
                endTime,
            );
            await appState.refreshVideos();
            close();
        }
    }

    function close() {
        appState.isEditVideoModalOpen = false;
        appState.videoToEdit = null;
    }
</script>

<div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm"
>
    <div
        class="w-full max-w-md bg-zinc-900 border border-zinc-800 rounded-xl p-6 shadow-2xl"
    >
        <h2 class="text-xl font-bold text-white mb-4">Edit Video</h2>
        <form
            onsubmit={(e) => {
                e.preventDefault();
                save();
            }}
        >
            <div class="mb-4">
                <label
                    class="block text-sm font-medium text-zinc-400 mb-1"
                    for="title">Title</label
                >
                <input
                    type="text"
                    id="title"
                    bind:value={title}
                    class="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-blue-600 transition"
                />
            </div>

            <div class="mb-4">
                <label
                    class="block text-sm font-medium text-zinc-400 mb-1"
                    for="start">Start Time (seconds)</label
                >
                <input
                    type="number"
                    id="start"
                    bind:value={startTime}
                    class="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-blue-600 transition"
                />
            </div>

            <div class="mb-4">
                <label
                    class="block text-sm font-medium text-zinc-400 mb-1"
                    for="end">End Time (seconds)</label
                >
                <p class="text-xs text-zinc-500 mb-1">
                    Set to 0 to play until the end of the video
                </p>
                <input
                    type="number"
                    id="end"
                    bind:value={endTime}
                    class="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-blue-600 transition"
                />
            </div>

            <div class="flex justify-end gap-3">
                <button
                    type="button"
                    onclick={close}
                    class="px-4 py-2 rounded-lg hover:bg-zinc-800 text-zinc-300 transition"
                    >Cancel</button
                >
                <button
                    type="submit"
                    class="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded-lg text-white font-medium transition"
                    >Save</button
                >
            </div>
        </form>
    </div>
</div>
