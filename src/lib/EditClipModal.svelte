<script lang="ts">
    import type { Clip } from "./db";
    import { updateClip } from "./db";
    import { untrack } from "svelte";
    import { appState } from "./state.svelte";
    import { formatClock } from "./youtube.svelte";
    import Modal from "./Modal.svelte";

    let { clip, onClose, onSaved }: {
        clip: Clip;
        onClose: () => void;
        onSaved: () => void;
    } = $props();

    // The modal is created per clip, so these are initial values by design.
    const source = untrack(() => clip);
    let title = $state(source.title);
    let startTime = $state(source.start_time);
    let endTime = $state(source.end_time);
    let saving = $state(false);

    const length = $derived(Math.max(0, Math.round(endTime - startTime)));

    async function handleSave() {
        if (!title.trim()) return;
        saving = true;
        try {
            await updateClip({
                ...clip,
                title,
                start_time: Math.max(0, Math.round(startTime)),
                end_time: Math.max(0, Math.round(endTime)),
            });
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

<Modal title="Edit clip" onClose={onClose} size="sm">
    <form
        id="edit-clip-form"
        onsubmit={(e) => {
            e.preventDefault();
            handleSave();
        }}
        class="flex flex-col gap-4"
    >
        <div>
            <label class="label" for="clip-title">Title</label>
            <input bind:value={title} type="text" id="clip-title" class="field" />
        </div>

        <div class="flex gap-3">
            <div class="flex-1">
                <label class="label" for="clip-start">Start (seconds)</label>
                <input bind:value={startTime} type="number" min="0" id="clip-start" class="field t-num" />
            </div>
            <div class="flex-1">
                <label class="label" for="clip-end">End (seconds)</label>
                <input bind:value={endTime} type="number" min="0" id="clip-end" class="field t-num" />
            </div>
        </div>

        <p class="text-xs t-num text-[color:var(--text-faint)]">
            {formatClock(startTime)} → {formatClock(endTime)} · {length}s
        </p>
    </form>

    {#snippet footer()}
        <button type="button" class="btn btn-ghost" onclick={onClose}>Cancel</button>
        <button
            type="submit"
            form="edit-clip-form"
            class="btn btn-primary"
            disabled={saving || !title.trim()}
        >
            {saving ? "Saving…" : "Save"}
        </button>
    {/snippet}
</Modal>
