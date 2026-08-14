<script lang="ts">
    import { appState } from "./state.svelte";
    import { updateVideoFolder, updateFolderParent, type Video, type Folder } from "./db";
    import Icon from "./Icon.svelte";

    /// Move without a drag. Touch cannot start an HTML5 drag, so this is the
    /// only way to restructure the library on a phone — and on a desktop it is
    /// the keyboard path.
    let {
        open = false,
        title = "Move to folder",
        excludeFolderId,
        pickOnly = false,
        onClose,
    } = $props<{
        open: boolean;
        title?: string;
        excludeFolderId?: number;
        /// Report the chosen folder through `getPickedFolder()` instead of
        /// moving anything — used when the caller has not saved a row yet.
        pickOnly?: boolean;
        onClose: () => void;
    }>();

    let selectedVideo = $state<Video | null>(null);
    let movingFolder = $state<Folder | null>(null);
    let browsing = $state<Folder | null>(null);
    let pickedFolderId = $state<number | null>(null);

    export function moveVideo(v: Video) {
        selectedVideo = v;
        movingFolder = null;
        browsing = null;
    }

    export function moveFolder(f: Folder) {
        selectedVideo = null;
        movingFolder = f;
        browsing = null;
    }

    export function setSelectedVideo(v: Video) {
        selectedVideo = v;
        movingFolder = null;
        browsing = null;
    }

    export function getPickedFolder(): number | null {
        return pickedFolderId;
    }

    /// A folder cannot be moved inside itself or its own descendants.
    function isDescendantOf(folder: Folder, ancestorId: number): boolean {
        let cursor: number | null | undefined = folder.parent_id;
        while (cursor !== null && cursor !== undefined) {
            if (cursor === ancestorId) return true;
            cursor = appState.folders.find((f) => f.id === cursor)?.parent_id;
        }
        return false;
    }

    const candidates = $derived(
        appState.folders.filter((f) => {
            if (excludeFolderId !== undefined && f.id === excludeFolderId) return false;
            if (movingFolder?.id !== undefined && isDescendantOf(f, movingFolder.id)) return false;
            return true;
        }),
    );

    const shown = $derived(
        candidates
            .filter((f) => f.parent_id === (browsing?.id ?? null))
            .sort((a, b) => a.sort_order - b.sort_order),
    );

    const trail = $derived.by(() => {
        const chain: Folder[] = [];
        let cursor: Folder | null = browsing;
        while (cursor) {
            chain.unshift(cursor);
            const parentId: number | null = cursor.parent_id;
            cursor = parentId === null ? null : (appState.folders.find((f) => f.id === parentId) ?? null);
        }
        return chain;
    });

    async function pick(target: number | null) {
        pickedFolderId = target;
        if (pickOnly) {
            onClose();
            return;
        }
        try {
            if (selectedVideo) {
                await updateVideoFolder(selectedVideo.id, target);
                await appState.refreshVideos();
                appState.showToast(`Moved “${selectedVideo.title}”`, "success");
            } else if (movingFolder?.id !== undefined) {
                await updateFolderParent(movingFolder.id, target);
                await appState.refreshFolders();
                appState.showToast(`Moved folder “${movingFolder.name}”`, "success");
            }
        } catch (e) {
            appState.showToast(`Could not move: ${String(e)}`, "error");
        }
        onClose();
    }
</script>

{#if open}
    <div
        class="overlay z-[250] items-end md:items-center justify-center md:p-4"
        onclick={onClose}
        role="presentation"
    >
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <div
            class="dialog w-full md:max-w-md rounded-b-none md:rounded-xl flex flex-col max-h-[80dvh] overflow-hidden"
            onclick={(e) => e.stopPropagation()}
            role="dialog"
            tabindex="-1"
            aria-modal="true"
            aria-label={title}
            onkeydown={(e) => e.key === "Escape" && onClose()}
        >
            <header class="panel-head">
                <h3 class="flex-1 min-w-0 truncate text-[13px] font-semibold">{title}</h3>
                <button class="icon-btn" onclick={onClose} aria-label="Close">
                    <Icon name="close" size={15} />
                </button>
            </header>

            <!-- Where the move would land, and how to go back up. -->
            <nav class="flex items-center gap-1 px-3 py-2 text-xs border-b border-[color:var(--border)] overflow-x-auto scrollbar-none">
                <button
                    class="shrink-0 px-1 rounded hover:text-[color:var(--text)] {browsing
                        ? 'text-[color:var(--text-faint)]'
                        : 'text-[color:var(--text)]'}"
                    onclick={() => (browsing = null)}>Library</button
                >
                {#each trail as folder (folder.id)}
                    <Icon name="chevronRight" size={11} />
                    <button
                        class="shrink-0 px-1 rounded truncate max-w-[140px] hover:text-[color:var(--text)] {browsing?.id ===
                        folder.id
                            ? 'text-[color:var(--text)]'
                            : 'text-[color:var(--text-faint)]'}"
                        onclick={() => (browsing = folder)}>{folder.name}</button
                    >
                {/each}
            </nav>

            <div class="flex-1 min-h-0 overflow-y-auto scroll-thin">
                {#each shown as folder (folder.id)}
                    <div class="flex items-center border-b border-[color:var(--border)]">
                        <button
                            class="row row-touch md:row flex-1 min-w-0"
                            style="min-height: 48px"
                            onclick={() => (browsing = folder)}
                        >
                            <Icon name="folder" size={16} />
                            <span class="flex-1 truncate text-[13px] text-left">{folder.name}</span>
                            <Icon name="chevronRight" size={14} />
                        </button>
                        <button
                            class="btn btn-ghost shrink-0 mr-2"
                            style="color: var(--accent)"
                            onclick={() => folder.id !== undefined && pick(folder.id)}
                        >
                            Move here
                        </button>
                    </div>
                {/each}
                {#if shown.length === 0}
                    <p class="px-4 py-8 text-center text-sm text-[color:var(--text-faint)]">
                        No folders inside {browsing?.name ?? "the library"}
                    </p>
                {/if}
            </div>

            <footer class="shrink-0 p-3 border-t border-[color:var(--border)]">
                <button class="btn btn-primary w-full" style="height: 40px" onclick={() => pick(browsing?.id ?? null)}>
                    Move into {browsing ? `“${browsing.name}”` : "the library root"}
                </button>
            </footer>
        </div>
    </div>
{/if}
