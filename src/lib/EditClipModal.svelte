<script lang="ts">
    import type { Clip } from "./db";
    import { updateClip } from "./db";
    import { appState } from "./state.svelte";

    let { clip, onClose, onSaved }: {
        clip: Clip;
        onClose: () => void;
        onSaved: () => void;
    } = $props();

    let title = $state("");
    let startTime = $state(0);
    let endTime = $state(0);
    let saving = $state(false);

    $effect(() => {
        // Reset when clip changes (if modal is reused, though likely destroyed/recreated)
        title = clip.title;
        startTime = clip.start_time;
        endTime = clip.end_time;
    });

    async function handleSave() {
        if (!title.trim()) return;
        saving = true;
        try {
            const updatedClip: Clip = {
                ...clip,
                title,
                start_time: Math.round(startTime),
                end_time: Math.round(endTime),
            };
            await updateClip(updatedClip);
            onSaved();
            onClose();
            appState.showToast("Clip updated", "success");
        } catch (e) {
            console.error(e);
            appState.showToast(`Failed to update clip: ${String(e)}`, "error");
        } finally {
            saving = false;
        }
    }
</script>

<div
    class="absolute inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm"
>
    <div
        class="w-full max-w-md bg-zinc-900 border border-zinc-800 rounded-xl p-6 shadow-2xl"
    >
        <h3 class="text-lg font-bold text-white mb-4">Edit Clip</h3>

        <div class="mb-4">
            <label
                class="block text-sm font-medium text-zinc-400 mb-1"
                for="title">Title</label
            >
            <input
                bind:value={title}
                type="text"
                id="title"
                class="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-blue-600 transition"
            />
        </div>

        <div class="grid grid-cols-2 gap-4 mb-4">
            <div>
                <label
                    class="block text-sm font-medium text-zinc-400 mb-1"
                    for="start">Start Time (s)</label
                >
                <input
                    bind:value={startTime}
                    type="number"
                    id="start"
                    class="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-blue-600 transition"
                />
            </div>
            <div>
                <label
                    class="block text-sm font-medium text-zinc-400 mb-1"
                    for="end">End Time (s)</label
                >
                <input
                    bind:value={endTime}
                    type="number"
                    id="end"
                    class="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-blue-600 transition"
                />
            </div>
        </div>

        <div class="mb-4 text-sm text-zinc-400">
            Duration: {(endTime - startTime).toFixed(1)}s
        </div>

        <div class="flex justify-end gap-3">
            <button
                onclick={onClose}
                class="px-4 py-2 rounded-lg hover:bg-zinc-800 text-zinc-300 transition"
                >Cancel</button
            >
            <button
                onclick={handleSave}
                disabled={saving || !title.trim()}
                class="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded-lg text-white font-medium disabled:opacity-50 transition"
            >
                {saving ? "Saving..." : "Save Changes"}
            </button>
        </div>
    </div>
</div>
