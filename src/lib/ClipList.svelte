<script lang="ts">
    import { appState } from "./state.svelte";
    import { deleteClip, renameClip, restoreClip, updateClipSortOrder, type Clip } from "./db";
    import { writeText } from "@tauri-apps/plugin-clipboard-manager";

    let { videoId, seekTo } = $props<{
        videoId: string;
        seekTo: (t: number) => void;
    }>();

    let draggedClipId = $state<number | null>(null);
    let dragOverClipId = $state<number | null>(null);

    async function moveClipUp(clipId: number) {
        const clips = [...appState.activeClips].sort((a, b) => a.sort_order - b.sort_order);
        const index = clips.findIndex((c) => c.id === clipId);
        if (index <= 0) return;

        [clips[index - 1], clips[index]] = [clips[index], clips[index - 1]];

        const updates: { id: number; sort_order: number }[] = [];
        clips.forEach((clip, i) => {
            if (clip.sort_order !== i) {
                updates.push({ id: clip.id!, sort_order: i });
            }
        });
        if (updates.length > 0) {
            await updateClipSortOrder(updates);
            await appState.refreshActiveClips();
        }
    }

    async function moveClipDown(clipId: number) {
        const clips = [...appState.activeClips].sort((a, b) => a.sort_order - b.sort_order);
        const index = clips.findIndex((c) => c.id === clipId);
        if (index === -1 || index >= clips.length - 1) return;

        [clips[index], clips[index + 1]] = [clips[index + 1], clips[index]];

        const updates: { id: number; sort_order: number }[] = [];
        clips.forEach((clip, i) => {
            if (clip.sort_order !== i) {
                updates.push({ id: clip.id!, sort_order: i });
            }
        });
        if (updates.length > 0) {
            await updateClipSortOrder(updates);
            await appState.refreshActiveClips();
        }
    }

    function getIsFirstClip(clipId: number): boolean {
        const clips = [...appState.activeClips].sort((a, b) => a.sort_order - b.sort_order);
        return clips[0]?.id === clipId;
    }

    function getIsLastClip(clipId: number): boolean {
        const clips = [...appState.activeClips].sort((a, b) => a.sort_order - b.sort_order);
        return clips[clips.length - 1]?.id === clipId;
    }

    async function handleClipReorder(draggedId: number, targetId: number, before: boolean) {
        const clips = [...appState.activeClips].sort((a, b) => a.sort_order - b.sort_order);

        const draggedIndex = clips.findIndex((c) => c.id === draggedId);
        if (draggedIndex === -1) return;

        const [draggedClip] = clips.splice(draggedIndex, 1);
        let targetIndex = clips.findIndex((c) => c.id === targetId);
        if (targetIndex === -1) return;

        if (!before) {
            targetIndex += 1;
        }
        clips.splice(targetIndex, 0, draggedClip);

        const updates: { id: number; sort_order: number }[] = [];
        clips.forEach((clip, index) => {
            if (clip.sort_order !== index) {
                updates.push({ id: clip.id!, sort_order: index });
            }
        });

        if (updates.length > 0) {
            await updateClipSortOrder(updates);
            await appState.refreshActiveClips();
        }
    }

    function handleDragStart(e: DragEvent, clipId: number) {
        draggedClipId = clipId;
        if (e.dataTransfer) {
            e.dataTransfer.effectAllowed = "move";
        }
    }

    function handleDragOver(e: DragEvent, clipId: number) {
        e.preventDefault();
        if (e.dataTransfer) {
            e.dataTransfer.dropEffect = "move";
        }
        dragOverClipId = clipId;
    }

    function handleDragLeave() {
        dragOverClipId = null;
    }

    function handleDrop(e: DragEvent, targetId: number) {
        e.preventDefault();
        if (draggedClipId !== null && draggedClipId !== targetId) {
            handleClipReorder(draggedClipId, targetId, true);
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

    async function handleCopy(clip: any, e?: MouseEvent) {
        e?.stopPropagation();
        try {
            const code = renderTemplate(clip);
            await writeText(code);
            appState.showToast("Copied to clipboard", "success");
        } catch (e) {
            console.error(e);
            appState.showToast(`Clipboard error: ${String(e)}`, "error");
        }
    }
    async function handleCopyAll() {
        if (appState.activeClips.length === 0) return;

        try {
            const allClipsContent = appState.activeClips
                .map((clip) => renderTemplate(clip))
                .join("\n\n");

            await writeText(allClipsContent);
            appState.showToast("Copied all clips to clipboard", "success");
        } catch (e) {
            console.error(e);
            appState.showToast(`Clipboard error: ${String(e)}`, "error");
        }
    }

    function renderTemplate(clip: any): string {
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

    // Edit Modal State
    import EditClipModal from "./EditClipModal.svelte";
    let editingClip = $state<Clip | null>(null);

    // Rename Modal State
    let renamingClip = $state<Clip | null>(null);
    let renamingTitle = $state("");

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
    <div
        class="absolute inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm"
    >
        <div
            class="w-full max-w-md bg-zinc-900 border border-zinc-800 rounded-xl p-6 shadow-2xl"
        >
            <h3 class="text-lg font-bold text-white mb-4">Rename Clip</h3>

            <div class="mb-4">
                <label
                    class="block text-sm font-medium text-zinc-400 mb-1"
                    for="rename-title">Title</label
                >
                <input
                    bind:value={renamingTitle}
                    type="text"
                    id="rename-title"
                    class="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-blue-600 transition"
                    onkeydown={(e) => e.key === "Enter" && handleRenameSave()}
                />
            </div>

            <div class="flex justify-end gap-3">
                <button
                    onclick={() => (renamingClip = null)}
                    class="px-4 py-2 rounded-lg hover:bg-zinc-800 text-zinc-300 transition"
                    >Cancel</button
                >
                <button
                    onclick={handleRenameSave}
                    disabled={!renamingTitle.trim()}
                    class="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded-lg text-white font-medium disabled:opacity-50 transition"
                >
                    Rename
                </button>
            </div>
        </div>
    </div>
{/if}

<div class="flex flex-col gap-2 p-2 h-full">
    {#if appState.activeClips.length === 0}
        <div class="text-zinc-500 text-center py-8 text-sm w-full">
            No clips yet
        </div>
    {:else}
        <button
            onclick={handleCopyAll}
            class="w-full py-2 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded text-sm font-medium transition-colors border border-zinc-700 flex flex-col items-center justify-center gap-1 shrink-0"
        >
            <span>Clip All</span>
            <span class="text-xs opacity-60"
                >({appState.activeClips.length})</span
            >
        </button>
    {/if}

    {#each appState.activeClips as clip (clip.id)}
        <div
            role="button"
            tabindex="0"
            draggable="true"
            ondragstart={(e) => handleDragStart(e, clip.id!)}
            ondragover={(e) => handleDragOver(e, clip.id!)}
            ondragleave={handleDragLeave}
            ondrop={(e) => handleDrop(e, clip.id!)}
            ondragend={handleDragEnd}
            onclick={() => seekTo(clip.start_time)}
            oncontextmenu={(e) => {
                e.preventDefault();
                appState.contextMenu = {
                    x: e.clientX,
                    y: e.clientY,
                    show: true,
                    items: [
                        {
                            label: "Rename",
                            action: () => startRename(clip),
                        },
                        {
                            label: "Edit",
                            action: () => (editingClip = clip),
                        },
                        {
                            label: "Copy Embed",
                            action: () => handleCopy(clip),
                        },
                        {
                            label: "Delete",
                            danger: true,
                            action: () => handleDelete(clip.id!),
                        },
                    ],
                };
            }}
            onkeydown={(e) => e.key === "Enter" && seekTo(clip.start_time)}
            class="group w-full p-3 rounded-lg bg-zinc-900 border transition cursor-pointer flex flex-col gap-1 shrink-0 justify-between {draggedClipId === clip.id ? 'opacity-50 border-blue-500' : dragOverClipId === clip.id ? 'border-blue-400' : 'border-zinc-800 hover:border-zinc-700'}"
        >
            <div class="flex justify-between items-start">
                <span
                    class="font-medium text-sm text-zinc-200 line-clamp-2 md:line-clamp-none whitespace-normal"
                    >{clip.title}</span
                >
                <div class="flex gap-1 shrink-0">
                    <button
                        onclick={(e) => {
                            e.stopPropagation();
                            editingClip = clip;
                        }}
                        class="p-1 hover:text-blue-400"
                        title="Edit"
                    >
                        <svg
                            class="size-4"
                            fill="none"
                            viewBox="0 0 24 24"
                            stroke="currentColor"
                        >
                            <path
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                stroke-width="2"
                                d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"
                            />
                        </svg>
                    </button>
                    <div class="flex flex-col">
                        <button
                            onclick={(e) => {
                                e.stopPropagation();
                                moveClipUp(clip.id!);
                            }}
                            class="p-1 hover:text-blue-400 disabled:opacity-30 disabled:cursor-not-allowed"
                            title="Move Up"
                            disabled={getIsFirstClip(clip.id!)}
                        >
                            <svg class="size-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7" />
                            </svg>
                        </button>
                        <button
                            onclick={(e) => {
                                e.stopPropagation();
                                moveClipDown(clip.id!);
                            }}
                            class="p-1 hover:text-blue-400 disabled:opacity-30 disabled:cursor-not-allowed"
                            title="Move Down"
                            disabled={getIsLastClip(clip.id!)}
                        >
                            <svg class="size-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
                            </svg>
                        </button>
                    </div>
                    <button
                        onclick={(e) => handleCopy(clip, e)}
                        class="p-1 hover:text-blue-400"
                        title="Copy Embed"
                    >
                        <svg
                            class="size-4"
                            fill="none"
                            viewBox="0 0 24 24"
                            stroke="currentColor"
                            ><path
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                stroke-width="2"
                                d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"
                            /></svg
                        >
                    </button>
                    <button
                        onclick={(e) => {
                            e.stopPropagation();
                            handleDelete(clip.id!);
                        }}
                        class="p-1 hover:text-red-400"
                        title="Delete"
                    >
                        <svg
                            class="size-4"
                            fill="none"
                            viewBox="0 0 24 24"
                            stroke="currentColor"
                            ><path
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                stroke-width="2"
                                d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                            /></svg
                        >
                    </button>
                </div>
            </div>
            <div class="text-xs text-zinc-500 font-mono">
                {Math.floor(clip.start_time)}s - {Math.floor(clip.end_time)}s
            </div>
        </div>
    {/each}
</div>
