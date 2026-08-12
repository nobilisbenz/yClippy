<script lang="ts">
    import { saveClip, type Video, type Clip } from "./db";
    import { appState } from "./state.svelte";
    import { getCurrentWebview } from "@tauri-apps/api/webview";
    import { writeText } from "@tauri-apps/plugin-clipboard-manager";

    let { video, startTime, endTime, onClose, onSaved } = $props<{
        video: Video;
        startTime: number;
        endTime: number;
        onClose: () => void;
        onSaved: () => void;
    }>();

    let title = $state("");
    let initialized = false;
    $effect(() => {
        if (!initialized && video) {
            initialized = true;
            title = `Clip from ${video.title}`;
        }
    });

    let saving = $state(false);

    async function handleSave() {
        if (!title.trim()) return;
        saving = true;
        try {
            const clip: Clip = {
                video_id: video.id,
                start_time: Math.round(startTime),
                end_time: Math.round(endTime),
                title: title,
                created_at: Date.now(),
                sort_order: 0,
            };
            await saveClip(clip);
            await appState.refreshActiveClips();
            onSaved();
        } catch (e) {
            console.error(e);
            alert("Failed to save clip: " + e);
        } finally {
            saving = false;
        }
    }

    async function handleClipboard() {
        try {
            let template = appState.settings.clipboardTemplate || "";
            const code = template
                .replace(/\\n/g, "\n")
                .replace(/\/n/g, "\n")
                .replace(/{id}/gi, video.id)
                .replace(/{start}/gi, Math.floor(startTime).toString())
                .replace(/{end}/gi, Math.floor(endTime).toString())
                .replace(/{title}/gi, title || `Clip from ${video.title}`)
                .replace(
                    /{url}/gi,
                    `https://www.youtube.com/watch?v=${video.id}&t=${Math.floor(startTime)}s`,
                );

            await writeText(code);
            onSaved();
            alert("Copied custom template to clipboard!");
        } catch (e) {
            console.error(e);
            alert("Clipboard Error: " + e);
        }
    }
</script>

<div
    class="absolute inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm"
>
    <div
        class="w-full max-w-md bg-zinc-900 border border-zinc-800 rounded-xl p-6 shadow-2xl"
    >
        <h3 class="text-lg font-bold text-white mb-4">Save Clip</h3>
        <div class="mb-4 text-sm text-zinc-400">
            Duration: {(endTime - startTime).toFixed(1)}s
        </div>

        <div class="mb-4">
            <label
                class="block text-sm font-medium text-zinc-400 mb-1"
                for="title">Title</label
            >
            <input
                bind:value={title}
                type="text"
                class="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-blue-600 transition"
            />
        </div>

        <div class="flex justify-end gap-3">
            <button
                onclick={onClose}
                class="px-4 py-2 rounded-lg hover:bg-zinc-800 text-zinc-300 transition"
                >Cancel</button
            >
            <button
                onclick={handleClipboard}
                class="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 rounded-lg text-white font-medium transition border border-zinc-700"
                >Copy Embed</button
            >
            <button
                onclick={handleSave}
                disabled={saving || !title.trim()}
                class="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded-lg text-white font-medium disabled:opacity-50 transition"
            >
                {saving ? "Saving..." : "Save Clip"}
            </button>
        </div>
    </div>
</div>
