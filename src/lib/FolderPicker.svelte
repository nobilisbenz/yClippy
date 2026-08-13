<script lang="ts">
    import { appState } from "./state.svelte";
    import { updateVideoFolder, updateFolderParent, type Video, type Folder } from "./db";

    let {
        open = false,
        title = "Move to folder",
        excludeFolderId,
        onClose,
    } = $props<{
        open: boolean;
        title?: string;
        excludeFolderId?: number;
        onClose: () => void;
    }>();

    let selectedVideo = $state<Video | null>(null);
    let selectedFolder = $state<Folder | null>(null);
    let movingFolder = $state<Folder | null>(null);
    let movingFolderParent = $state<number | null>(null);
    let pickedFolderId = $state<number | null>(null);
    let pickOnly = $state(false);

    export function moveVideo(v: Video) {
        selectedVideo = v;
        selectedFolder = null;
        movingFolder = null;
        pickOnly = false;
    }

    export function moveFolder(f: Folder) {
        selectedVideo = null;
        selectedFolder = null;
        movingFolder = f;
        movingFolderParent = f.parent_id;
        pickOnly = false;
    }

    export function setSelectedVideo(v: Video) {
        selectedVideo = v;
        selectedFolder = null;
        movingFolder = null;
        pickOnly = true;
    }

    export function getPickedFolder(): number | null {
        return pickedFolderId;
    }

    function folderChain(id: number | null, chain: Folder[] = []): Folder[] {
        if (id === null) return chain;
        const f = appState.folders.find((f) => f.id === id);
        if (!f) return chain;
        return folderChain(f.parent_id, [f, ...chain]);
    }

    let candidateFolders = $derived(
        appState.folders.filter((f) => {
            if (excludeFolderId !== undefined && f.id === excludeFolderId) return false;
            if (selectedFolder?.id !== undefined) {
                let cur: number | null | undefined = f.parent_id;
                while (cur !== null && cur !== undefined) {
                    if (cur === selectedFolder.id) return false;
                    const parent = appState.folders.find((p) => p.id === cur);
                    cur = parent?.parent_id;
                }
            }
            if (movingFolder?.id !== undefined) {
                let cur: number | null | undefined = f.parent_id;
                while (cur !== null && cur !== undefined) {
                    if (cur === movingFolder.id) return false;
                    const parent = appState.folders.find((p) => p.id === cur);
                    cur = parent?.parent_id;
                }
            }
            return true;
        }),
    );

    async function pick(target: number | null) {
        if (pickOnly) {
            pickedFolderId = target;
            onClose();
            return;
        }
        if (selectedVideo) {
            await updateVideoFolder(selectedVideo.id, target);
            await appState.refreshVideos();
            appState.showToast(`Moved "${selectedVideo.title}"`, "success");
        } else if (movingFolder && movingFolder.id !== undefined) {
            await updateFolderParent(movingFolder.id, target);
            await appState.refreshFolders();
            appState.showToast(`Moved folder "${movingFolder.name}"`, "success");
        }
        onClose();
    }

    function jumpInto(f: Folder) {
        selectedFolder = f;
    }

    function jumpOut() {
        selectedFolder = null;
    }
</script>

{#if open}
    <div
        class="fixed inset-0 z-[250] bg-black/70 flex items-end md:items-center justify-center p-0 md:p-4"
        onclick={onClose}
        role="presentation"
    >
        <div
            class="w-full md:max-w-md bg-[color:var(--surface)] border border-[color:var(--border)] rounded-t-2xl md:rounded-xl shadow-2xl overflow-hidden flex flex-col max-h-[80vh]"
            onclick={(e) => e.stopPropagation()}
            role="dialog"
            aria-modal="true"
            tabindex="-1"
            onkeydown={(e) => e.key === "Escape" && onClose()}
        >
            <div class="p-4 border-b border-[color:var(--border)] flex items-center justify-between">
                <h3 class="font-bold">{title}</h3>
                <button
                    onclick={onClose}
                    class="min-w-[48px] min-h-[48px] flex items-center justify-center hover:bg-zinc-800 rounded-full"
                    aria-label="Close"
                >
                    <svg class="size-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                    </svg>
                </button>
            </div>

            {#if selectedFolder}
                <div class="px-4 py-2 text-xs text-zinc-500 border-b border-[color:var(--border)]">
                    Inside: {selectedFolder.name}
                </div>
                <button
                    onclick={jumpOut}
                    class="px-4 py-3 text-left text-sm hover:bg-[color:var(--surface-hi)] flex items-center gap-2 border-b border-[color:var(--border)]"
                >
                    <svg class="size-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
                    </svg>
                    Up one level
                </button>
            {/if}

            <div class="flex-1 overflow-y-auto">
                <button
                    onclick={() => pick(selectedFolder?.id ?? null)}
                    class="w-full px-4 py-3 text-left text-sm hover:bg-[color:var(--surface-hi)] flex items-center gap-2 border-b border-[color:var(--border)] text-blue-400"
                >
                    <svg class="size-4" fill="currentColor" viewBox="0 0 24 24">
                        <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
                    </svg>
                    {selectedFolder ? `Move into "${selectedFolder.name}"` : "Move to root"}
                </button>
                {#each candidateFolders.filter((f) => f.parent_id === (selectedFolder?.id ?? null)) as folder (folder.id)}
                    <div class="flex items-center border-b border-[color:var(--border)]">
                        <button
                            onclick={() => jumpInto(folder)}
                            class="flex-1 px-4 py-3 text-left text-sm hover:bg-[color:var(--surface-hi)] truncate"
                        >
                            📁 {folder.name}
                        </button>
                        <button
                            onclick={() => folder.id !== undefined && pick(folder.id)}
                            class="px-4 py-3 text-xs text-blue-400 hover:bg-[color:var(--surface-hi)]"
                            title="Move here"
                        >
                            Move here
                        </button>
                    </div>
                {/each}
                {#if candidateFolders.filter((f) => f.parent_id === (selectedFolder?.id ?? null)).length === 0}
                    <div class="px-4 py-8 text-center text-zinc-500 text-sm">
                        No subfolders
                    </div>
                {/if}
            </div>
        </div>
    </div>
{/if}