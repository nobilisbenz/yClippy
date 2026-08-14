<script lang="ts">
    import { appState } from "./state.svelte";
    import { deleteClip, renameClip, restoreClip, updateClipSortOrder, type Clip } from "./db";
    import { writeText } from "@tauri-apps/plugin-clipboard-manager";
    import { formatClock } from "./youtube.svelte";
    import EditClipModal from "./EditClipModal.svelte";
    import Icon from "./Icon.svelte";

    /// A clip is a named range, so the row leads with the range: one
    /// tabular-nums timecode, then the name. Sorted by `sort_order`, which is
    /// the order you chose, not the order you happened to record in.
    let { videoId, seekTo, touch = false } = $props<{
        videoId: string;
        seekTo: (t: number) => void;
        touch?: boolean;
    }>();

    let draggedClipId = $state<number | null>(null);
    let dragOverClipId = $state<number | null>(null);
    let dragBefore = $state(true);
    let editingClip = $state<Clip | null>(null);
    let renamingClip = $state<Clip | null>(null);
    let renamingTitle = $state("");

    const clips = $derived(
        [...appState.activeClips].sort((a, b) => a.sort_order - b.sort_order),
    );

    /// Renumbers the whole list on every move. Writing only the two swapped
    /// rows leaves the rest with stale or duplicate orders.
    async function commitOrder(ordered: Clip[]) {
        const updates: { id: number; sort_order: number }[] = [];
        ordered.forEach((clip, index) => {
            if (clip.sort_order !== index && clip.id !== undefined) {
                updates.push({ id: clip.id, sort_order: index });
            }
        });
        if (updates.length === 0) return;
        await updateClipSortOrder(updates);
        await appState.refreshActiveClips();
    }

    async function move(clipId: number, delta: number) {
        const ordered = [...clips];
        const index = ordered.findIndex((c) => c.id === clipId);
        const next = index + delta;
        if (index === -1 || next < 0 || next >= ordered.length) return;
        const [lifted] = ordered.splice(index, 1);
        ordered.splice(next, 0, lifted);
        await commitOrder(ordered);
    }

    async function handleClipReorder(draggedId: number, targetId: number, before: boolean) {
        const ordered = [...clips];
        const draggedIndex = ordered.findIndex((c) => c.id === draggedId);
        if (draggedIndex === -1) return;
        const [lifted] = ordered.splice(draggedIndex, 1);
        // The target index is read *after* the lift, so "drop above" and "drop
        // below" no longer differ by one depending on travel direction.
        let targetIndex = ordered.findIndex((c) => c.id === targetId);
        if (targetIndex === -1) return;
        if (!before) targetIndex += 1;
        ordered.splice(targetIndex, 0, lifted);
        await commitOrder(ordered);
    }

    function handleDragStart(e: DragEvent, clipId: number) {
        draggedClipId = clipId;
        if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
    }

    function handleDragOver(e: DragEvent, clipId: number) {
        e.preventDefault();
        if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
        const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
        dragBefore = e.clientY - rect.top < rect.height / 2;
        dragOverClipId = clipId;
    }

    function handleDrop(e: DragEvent, targetId: number) {
        e.preventDefault();
        if (draggedClipId !== null && draggedClipId !== targetId) {
            handleClipReorder(draggedClipId, targetId, dragBefore);
        }
        draggedClipId = null;
        dragOverClipId = null;
    }

    function handleDragEnd() {
        draggedClipId = null;
        dragOverClipId = null;
    }

    async function handleDelete(id: number) {
        const clip = appState.activeClips.find((c) => c.id === id);
        if (!clip) return;
        const snapshot = appState.activeClips;
        await deleteClip(id);
        await appState.refreshActiveClips();
        appState.showUndo(`Deleted clip "${clip.title}"`, async () => {
            await restoreClip(id);
            appState.activeClips = snapshot;
            await appState.refreshActiveClips();
        });
    }

    async function handleCopy(clip: Clip, e?: MouseEvent) {
        e?.stopPropagation();
        try {
            await writeText(renderTemplate(clip));
            appState.showToast("Copied to clipboard", "success");
        } catch (e) {
            console.error(e);
            appState.showToast(`Clipboard error: ${String(e)}`, "error");
        }
    }

    async function handleCopyAll() {
        if (clips.length === 0) return;
        try {
            await writeText(clips.map((clip) => renderTemplate(clip)).join("\n\n"));
            appState.showToast(`Copied ${clips.length} clips`, "success");
        } catch (e) {
            console.error(e);
            appState.showToast(`Clipboard error: ${String(e)}`, "error");
        }
    }

    function renderTemplate(clip: Clip): string {
        const template = appState.settings.clipboardTemplate || "";
        const seconds = Math.floor(clip.start_time);
        const urlWithTs = `https://www.youtube.com/watch?v=${videoId}&t=${seconds}s`;
        const endSeconds = Math.floor(clip.end_time);
        const title = clip.title || "Clip";
        // `{url_clean}` carries no `t=`. The vault stores canonical URLs and
        // keeps the timestamp as its own field, so a line that writes both
        // would say the same thing twice.
        const urlClean = `https://www.youtube.com/watch?v=${videoId}`;
        return template
            .replace(/\\n/g, "\n")
            .replace(/{id}/gi, videoId)
            .replace(/{start}/gi, seconds.toString())
            .replace(/{start_hms}/gi, formatHMS(seconds))
            .replace(/{end}/gi, endSeconds.toString())
            .replace(/{end_hms}/gi, formatHMS(endSeconds))
            .replace(/{title}/gi, title)
            .replace(/{url_clean}/gi, urlClean)
            .replace(/{url}/gi, urlWithTs);
    }

    function formatHMS(totalSeconds: number): string {
        const h = Math.floor(totalSeconds / 3600);
        const m = Math.floor((totalSeconds % 3600) / 60);
        const s = Math.floor(totalSeconds % 60);
        return `${h.toString().padStart(2, "0")}:${m
            .toString()
            .padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
    }

    function startRename(clip: Clip) {
        renamingClip = clip;
        renamingTitle = clip.title;
    }

    async function handleRenameSave() {
        if (!renamingClip || !renamingTitle.trim()) return;
        if (renamingTitle !== renamingClip.title) {
            await renameClip(renamingClip.id!, renamingTitle);
            await appState.refreshActiveClips();
        }
        renamingClip = null;
    }

    function contextItems(clip: Clip) {
        return [
            { label: "Play from here", action: () => seekTo(clip.start_time) },
            { label: "Rename", action: () => startRename(clip) },
            { label: "Edit range…", action: () => (editingClip = clip) },
            { label: "Copy", action: () => handleCopy(clip) },
            { label: "Delete", danger: true, action: () => handleDelete(clip.id!) },
        ];
    }
</script>

{#if editingClip}
    <EditClipModal
        clip={editingClip}
        onClose={() => (editingClip = null)}
        onSaved={async () => {
            await appState.refreshActiveClips();
            editingClip = null;
        }}
    />
{/if}

{#if renamingClip}
    <div class="overlay z-[260] items-center justify-center p-4" role="presentation">
        <div class="dialog w-full max-w-sm p-5">
            <h3 class="text-base font-semibold mb-4">Rename clip</h3>
            <!-- svelte-ignore a11y_autofocus -->
            <input
                bind:value={renamingTitle}
                type="text"
                autofocus
                class="field"
                onkeydown={(e) => {
                    if (e.key === "Enter") handleRenameSave();
                    if (e.key === "Escape") renamingClip = null;
                }}
            />
            <div class="flex justify-end gap-2 mt-5">
                <button class="btn" onclick={() => (renamingClip = null)}>Cancel</button>
                <button
                    class="btn btn-primary"
                    onclick={handleRenameSave}
                    disabled={!renamingTitle.trim()}>Rename</button
                >
            </div>
        </div>
    </div>
{/if}

<div class="flex flex-col h-full min-h-0">
    {#if clips.length === 0}
        <div class="empty">
            <span class="text-[color:var(--text-faint)]"><Icon name="scissors" size={22} /></span>
            <p>No clips yet</p>
            <p class="text-[color:var(--text-faint)] leading-relaxed">
                {#if touch}
                    Tap <span class="text-[color:var(--text-dim)]">Mark in</span> at the
                    start of a moment, then again at its end.
                {:else}
                    Press <kbd class="kbd">[</kbd> at the start of a moment and
                    <kbd class="kbd">]</kbd> at its end, then <kbd class="kbd">↵</kbd> to
                    name it.
                {/if}
            </p>
        </div>
    {:else}
        <div class="flex-1 min-h-0 overflow-y-auto scroll-thin">
            {#each clips as clip, index (clip.id)}
                <div
                    role="button"
                    tabindex="0"
                    draggable={!touch}
                    ondragstart={(e) => handleDragStart(e, clip.id!)}
                    ondragover={(e) => handleDragOver(e, clip.id!)}
                    ondragleave={() => (dragOverClipId = null)}
                    ondrop={(e) => handleDrop(e, clip.id!)}
                    ondragend={handleDragEnd}
                    onclick={() => seekTo(clip.start_time)}
                    onkeydown={(e) => e.key === "Enter" && seekTo(clip.start_time)}
                    oncontextmenu={(e) => {
                        e.preventDefault();
                        appState.contextMenu = {
                            x: e.clientX,
                            y: e.clientY,
                            show: true,
                            items: contextItems(clip),
                        };
                    }}
                    class="row items-start py-2 border-b border-[color:var(--border)] cursor-pointer
                           {touch ? 'row-touch' : ''}
                           {draggedClipId === clip.id ? 'opacity-40' : ''}
                           {dragOverClipId === clip.id
                        ? dragBefore
                            ? 'border-t-2 border-t-[color:var(--accent)]'
                            : 'border-b-2 border-b-[color:var(--accent)]'
                        : ''}"
                >
                    <!-- The index doubles as the keyboard shortcut: 1–9 jump
                         to clip N while the player has focus. -->
                    <span
                        class="w-4 shrink-0 text-[11px] t-num text-[color:var(--text-faint)] pt-0.5 text-right"
                    >
                        {index + 1}
                    </span>

                    <div class="flex-1 min-w-0">
                        <div
                            class="text-sm text-[color:var(--text)] leading-snug break-words"
                        >
                            {clip.title}
                        </div>
                        <div
                            class="mt-1 flex items-center gap-2 text-[11px] t-num text-[color:var(--text-faint)]"
                        >
                            <span>{formatClock(clip.start_time)} → {formatClock(clip.end_time)}</span>
                            <span class="opacity-60">
                                {Math.max(0, Math.round(clip.end_time - clip.start_time))}s
                            </span>
                        </div>
                    </div>

                    <div class="row-actions pt-0.5">
                        {#if touch}
                            <button
                                class="icon-btn"
                                style="--size: 40px"
                                disabled={index === 0}
                                onclick={(e) => {
                                    e.stopPropagation();
                                    move(clip.id!, -1);
                                }}
                                aria-label="Move up"><Icon name="arrowUp" size={16} /></button
                            >
                            <button
                                class="icon-btn"
                                style="--size: 40px"
                                disabled={index === clips.length - 1}
                                onclick={(e) => {
                                    e.stopPropagation();
                                    move(clip.id!, 1);
                                }}
                                aria-label="Move down"><Icon name="arrowDown" size={16} /></button
                            >
                        {/if}
                        <button
                            class="icon-btn"
                            onclick={(e) => {
                                e.stopPropagation();
                                editingClip = clip;
                            }}
                            title="Edit range"
                            aria-label="Edit clip"><Icon name="pencil" size={14} /></button
                        >
                        <button
                            class="icon-btn"
                            onclick={(e) => handleCopy(clip, e)}
                            title="Copy"
                            aria-label="Copy clip"><Icon name="copy" size={14} /></button
                        >
                        <button
                            class="icon-btn icon-btn-danger"
                            onclick={(e) => {
                                e.stopPropagation();
                                handleDelete(clip.id!);
                            }}
                            title="Delete"
                            aria-label="Delete clip"><Icon name="trash" size={14} /></button
                        >
                    </div>
                </div>
            {/each}
        </div>

        <div
            class="shrink-0 border-t border-[color:var(--border)] p-2 flex items-center gap-2"
        >
            <button class="btn flex-1" onclick={handleCopyAll} style={touch ? "height: 44px" : ""}>
                <Icon name="copy" size={14} />
                Copy all {clips.length}
            </button>
        </div>
    {/if}
</div>
