<script lang="ts">
    import { appState } from "./state.svelte";
    import {
        type Video,
        type Folder,
        deleteFolder,
        deleteVideo,
        renameFolder,
        renameVideo,
        restoreFolder,
        restoreVideo,
        saveFolder,
        updateFolderParent,
        updateSortOrder,
        updateVideoFolder,
    } from "./db";
    import { formatClock } from "./youtube.svelte";
    import FolderPicker from "./FolderPicker.svelte";
    import Icon from "./Icon.svelte";
    import Thumbnail from "./Thumbnail.svelte";

    /// The Finder-style browser, shown when nothing is playing and the whole
    /// window is available. Each column is one folder; the selection path is
    /// shared with the rail and the title bar, so all three agree on where you
    /// are.
    type Item = { kind: "video"; video: Video } | { kind: "folder"; folder: Folder };

    let renamingId = $state<string | null>(null);
    let renamingText = $state("");
    let creatingIn = $state<number | null | undefined>(undefined);
    let newFolderName = $state("");

    let isFolderPickerOpen = $state(false);
    let folderPickerExcludedFolder = $state<number | undefined>(undefined);
    let folderPickerTitle = $state("Move to folder");
    let folderPickerRef = $state<
        | {
              moveVideo: (v: Video) => void;
              moveFolder: (f: Folder) => void;
              setSelectedVideo: (v: Video) => void;
          }
        | undefined
    >();

    const columns = $derived.by(() => {
        const parents: (number | null)[] = [null, ...appState.selectionPath];
        return parents.map((parentId) => ({
            id: parentId,
            name:
                parentId === null
                    ? "Library"
                    : (appState.folders.find((f) => f.id === parentId)?.name ?? "Folder"),
            items: itemsIn(parentId),
        }));
    });

    function itemsIn(parentId: number | null): Item[] {
        const out: Item[] = [];
        for (const folder of appState.folders
            .filter((f) => f.parent_id === parentId)
            .sort((a, b) => a.sort_order - b.sort_order)) {
            out.push({ kind: "folder", folder });
        }
        for (const video of appState.videos
            .filter((v) => v.folder_id === parentId)
            .sort((a, b) => a.sort_order - b.sort_order)) {
            out.push({ kind: "video", video });
        }
        return out;
    }

    function keyOf(item: Item): string {
        return item.kind === "folder" ? `f-${item.folder.id}` : `v-${item.video.id}`;
    }

    function childCount(folderId: number): number {
        return (
            appState.folders.filter((f) => f.parent_id === folderId).length +
            appState.videos.filter((v) => v.folder_id === folderId).length
        );
    }

    function selectFolder(folder: Folder, depth: number) {
        appState.openFolder([...appState.selectionPath.slice(0, depth), folder.id!]);
    }

    function deselect(depth: number) {
        appState.openFolder(appState.selectionPath.slice(0, depth));
    }

    // ── mutations ───────────────────────────────────────────────────────

    function startRename(item: Item) {
        renamingId = keyOf(item);
        renamingText = item.kind === "folder" ? item.folder.name : item.video.title;
    }

    async function commitRename(item: Item) {
        const name = renamingText.trim();
        const previous = item.kind === "folder" ? item.folder.name : item.video.title;
        renamingId = null;
        if (!name || name === previous) return;
        try {
            if (item.kind === "folder") {
                await renameFolder(item.folder.id!, name);
                await appState.refreshFolders();
            } else {
                await renameVideo(item.video.id, name);
                await appState.refreshVideos();
            }
        } catch (e) {
            appState.showToast(`Could not rename: ${String(e)}`, "error");
        }
    }

    function focusOnMount(node: HTMLInputElement) {
        queueMicrotask(() => {
            node.focus();
            node.select();
        });
    }

    function startCreateFolder() {
        creatingIn =
            appState.selectionPath.length > 0
                ? appState.selectionPath[appState.selectionPath.length - 1]
                : null;
        newFolderName = "";
    }

    async function commitCreateFolder() {
        const name = newFolderName.trim();
        const parentId = creatingIn === undefined ? null : creatingIn;
        creatingIn = undefined;
        newFolderName = "";
        if (!name) return;
        const maxOrder = appState.folders
            .filter((f) => f.parent_id === parentId)
            .reduce((max, f) => Math.max(max, f.sort_order), -1);
        try {
            await saveFolder({
                name,
                created_at: Date.now(),
                parent_id: parentId,
                sort_order: maxOrder + 1,
            });
            await appState.refreshFolders();
        } catch (e) {
            appState.showToast(`Could not create the folder: ${String(e)}`, "error");
        }
    }

    async function removeFolder(folder: Folder) {
        const snapshotFolders = appState.folders;
        const snapshotVideos = appState.videos;
        const snapshotPath = appState.selectionPath;
        await deleteFolder(folder.id!);
        await appState.refreshFolders();
        await appState.refreshVideos();
        const idx = appState.selectionPath.indexOf(folder.id!);
        if (idx !== -1) appState.openFolder(appState.selectionPath.slice(0, idx));

        appState.showUndo(`Deleted folder "${folder.name}"`, async () => {
            await restoreFolder(folder.id!);
            appState.folders = snapshotFolders;
            appState.videos = snapshotVideos;
            appState.openFolder(snapshotPath);
            await appState.refreshFolders();
            await appState.refreshVideos();
        });
    }

    async function removeVideo(video: Video) {
        const snapshot = appState.videos;
        await deleteVideo(video.id);
        await appState.refreshVideos();
        appState.showUndo(`Deleted video "${video.title}"`, async () => {
            await restoreVideo(video.id);
            appState.videos = snapshot;
            await appState.refreshVideos();
        });
    }

    function askMove(item: Item) {
        if (item.kind === "folder") {
            folderPickerTitle = `Move folder "${item.folder.name}"`;
            folderPickerExcludedFolder = item.folder.id;
            folderPickerRef?.moveFolder(item.folder);
        } else {
            folderPickerTitle = `Move "${item.video.title}"`;
            folderPickerExcludedFolder = undefined;
            folderPickerRef?.moveVideo(item.video);
        }
        isFolderPickerOpen = true;
    }

    function menuItems(item: Item, depth: number) {
        if (item.kind === "video") {
            const video = item.video;
            return [
                { label: "Play", action: () => appState.openVideo(video) },
                { label: "Rename", action: () => startRename(item) },
                {
                    label: "Edit trim…",
                    action: () => {
                        appState.videoToEdit = { ...video };
                        appState.isEditVideoModalOpen = true;
                    },
                },
                { label: "Move to…", action: () => askMove(item) },
                { label: "Delete", danger: true, action: () => removeVideo(video) },
            ];
        }
        const folder = item.folder;
        return [
            { label: "Open", action: () => selectFolder(folder, depth) },
            { label: "Rename", action: () => startRename(item) },
            { label: "Move to…", action: () => askMove(item) },
            { label: "Delete", danger: true, action: () => removeFolder(folder) },
        ];
    }

    // ── drag and drop ───────────────────────────────────────────────────

    let dragging = $state<Item | null>(null);
    let dropTarget = $state<string | null>(null);
    let dropMode = $state<"into" | "before" | "after" | null>(null);
    let dropColumn = $state<number | null | undefined>(undefined);

    function clearDrag() {
        dragging = null;
        dropTarget = null;
        dropMode = null;
        dropColumn = undefined;
    }

    function onDragOverItem(e: DragEvent, item: Item) {
        if (!dragging || keyOf(dragging) === keyOf(item)) return;
        e.preventDefault();
        e.stopPropagation();
        const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
        const offset = (e.clientY - rect.top) / rect.height;
        dropMode =
            item.kind === "folder"
                ? offset < 0.25
                    ? "before"
                    : offset > 0.75
                      ? "after"
                      : "into"
                : offset < 0.5
                  ? "before"
                  : "after";
        dropTarget = keyOf(item);
        dropColumn = undefined;
    }

    /// Renumbers a whole column after a move, so orders never collide.
    async function commitOrder(parentId: number | null, ordered: Item[]) {
        const folders: { id: number; sort_order: number }[] = [];
        const videos: { id: string; sort_order: number }[] = [];
        ordered.forEach((item, position) => {
            if (item.kind === "folder" && item.folder.id !== undefined) {
                folders.push({ id: item.folder.id, sort_order: position });
            } else if (item.kind === "video") {
                videos.push({ id: item.video.id, sort_order: position });
            }
        });
        await updateSortOrder(folders, videos);
        await appState.refreshAll();
    }

    async function onDropItem(e: DragEvent, item: Item, parentId: number | null) {
        e.preventDefault();
        e.stopPropagation();
        const source = dragging;
        const mode = dropMode;
        clearDrag();
        if (!source || !mode) return;

        try {
            if (mode === "into" && item.kind === "folder") {
                if (source.kind === "folder") {
                    if (source.folder.id === item.folder.id) return;
                    await updateFolderParent(source.folder.id!, item.folder.id!);
                } else {
                    await updateVideoFolder(source.video.id, item.folder.id!);
                }
                await appState.refreshAll();
                return;
            }

            // Reordering only makes sense inside one column; a row dragged in
            // from another folder is moved there first, then placed.
            const sourceParent =
                source.kind === "folder" ? source.folder.parent_id : source.video.folder_id;
            if (sourceParent !== parentId) {
                if (source.kind === "folder") await updateFolderParent(source.folder.id!, parentId);
                else await updateVideoFolder(source.video.id, parentId);
                await appState.refreshAll();
            }

            const ordered = itemsIn(parentId);
            const from = ordered.findIndex((i) => keyOf(i) === keyOf(source));
            let to = ordered.findIndex((i) => keyOf(i) === keyOf(item));
            if (from === -1 || to === -1) return;
            const [lifted] = ordered.splice(from, 1);
            if (mode === "after") to += 1;
            if (from < to) to -= 1;
            ordered.splice(to, 0, lifted);
            await commitOrder(parentId, ordered);
        } catch (err) {
            appState.showToast(`Could not move: ${String(err)}`, "error");
        }
    }

    async function onDropColumn(parentId: number | null) {
        const source = dragging;
        clearDrag();
        if (!source) return;
        try {
            if (source.kind === "folder") {
                if (source.folder.id === parentId) return;
                await updateFolderParent(source.folder.id!, parentId);
            } else {
                await updateVideoFolder(source.video.id, parentId);
            }
            await appState.refreshAll();
        } catch (e) {
            appState.showToast(`Could not move: ${String(e)}`, "error");
        }
    }
</script>

<div class="h-full flex flex-col min-h-0 bg-[color:var(--bg)]">
    <div class="flex-1 min-h-0 flex overflow-x-auto scroll-thin">
        {#each columns as col, depth (depth + ":" + (col.id ?? "root"))}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
                class="flex flex-col min-h-0 border-r border-[color:var(--border)] bg-[color:var(--surface)]
                       {depth === columns.length - 1
                    ? 'flex-1 min-w-[260px]'
                    : 'w-[260px] min-w-[260px]'}
                       {dropColumn === col.id ? 'bg-[color:var(--accent-soft)]' : ''}"
                ondragover={(e) => {
                    e.preventDefault();
                    dropColumn = col.id;
                    dropTarget = null;
                }}
                ondragleave={() => {
                    if (dropColumn === col.id) dropColumn = undefined;
                }}
                ondrop={(e) => {
                    e.stopPropagation();
                    onDropColumn(col.id);
                }}
            >
                <div class="panel-head" style="height: 32px">
                    <span class="section-label flex-1 truncate">{col.name}</span>
                    <span class="chip">{col.items.length}</span>
                </div>

                <div class="flex-1 min-h-0 overflow-y-auto scroll-thin py-1 flex flex-col">
                    {#each col.items as item (keyOf(item))}
                        {@const isRenaming = renamingId === keyOf(item)}
                        {@const isTarget = dropTarget === keyOf(item)}
                        {@const selected =
                            item.kind === "folder" &&
                            appState.selectionPath[depth] === item.folder.id}
                        <div
                            role="button"
                            tabindex="0"
                            draggable={!isRenaming}
                            aria-current={selected ? "true" : undefined}
                            data-selected={selected ? "true" : undefined}
                            ondragstart={() => (dragging = item)}
                            ondragover={(e) => onDragOverItem(e, item)}
                            ondragleave={() => {
                                if (dropTarget === keyOf(item)) dropTarget = null;
                            }}
                            ondrop={(e) => onDropItem(e, item, col.id)}
                            ondragend={clearDrag}
                            onclick={(e) => {
                                e.stopPropagation();
                                if (isRenaming) return;
                                if (item.kind === "folder") selectFolder(item.folder, depth);
                                else appState.openVideo(item.video);
                            }}
                            onkeydown={(e) => {
                                if (e.key === "Enter") {
                                    if (item.kind === "folder") selectFolder(item.folder, depth);
                                    else appState.openVideo(item.video);
                                } else if (e.key === "F2") {
                                    startRename(item);
                                }
                            }}
                            oncontextmenu={(e) => {
                                e.preventDefault();
                                e.stopPropagation();
                                appState.contextMenu = {
                                    x: e.clientX,
                                    y: e.clientY,
                                    show: true,
                                    items: menuItems(item, depth),
                                };
                            }}
                            class="row cursor-pointer
                                   {dragging && keyOf(dragging) === keyOf(item) ? 'opacity-40' : ''}
                                   {isTarget && dropMode === 'into'
                                ? 'bg-[color:var(--accent-soft)]'
                                : ''}"
                            style={isTarget && dropMode === "before"
                                ? "border-top-color: var(--accent)"
                                : isTarget && dropMode === "after"
                                  ? "border-bottom-color: var(--accent)"
                                  : ""}
                        >
                            {#if item.kind === "folder"}
                                <span class="shrink-0 opacity-80"><Icon name="folder" size={15} /></span>
                            {:else}
                                <Thumbnail
                                    videoId={item.video.id}
                                    alt=""
                                    className="w-11 h-[25px] object-cover rounded-[3px] bg-black shrink-0"
                                />
                            {/if}

                            {#if isRenaming}
                                <input
                                    bind:value={renamingText}
                                    class="field flex-1 min-w-0 py-1 text-sm"
                                    use:focusOnMount
                                    onclick={(e) => e.stopPropagation()}
                                    onblur={() => commitRename(item)}
                                    onkeydown={(e) => {
                                        e.stopPropagation();
                                        if (e.key === "Enter") commitRename(item);
                                        else if (e.key === "Escape") renamingId = null;
                                    }}
                                />
                            {:else}
                                <div class="flex-1 min-w-0">
                                    <div class="truncate text-[13px]">
                                        {item.kind === "folder"
                                            ? item.folder.name
                                            : item.video.title}
                                    </div>
                                    {#if item.kind === "video" && item.video.last_position > 0}
                                        <div
                                            class="text-[11px] t-num opacity-70 flex items-center gap-1"
                                        >
                                            <Icon name="play" size={9} />
                                            {formatClock(item.video.last_position)}
                                        </div>
                                    {/if}
                                </div>

                                {#if item.kind === "folder"}
                                    <span class="chip">{childCount(item.folder.id!)}</span>
                                    <span class="shrink-0 opacity-70">
                                        <Icon name="chevronRight" size={14} />
                                    </span>
                                {:else}
                                    <div class="row-actions">
                                        <button
                                            class="icon-btn"
                                            style="--size: 24px"
                                            onclick={(e) => {
                                                e.stopPropagation();
                                                appState.contextMenu = {
                                                    x: e.clientX,
                                                    y: e.clientY,
                                                    show: true,
                                                    items: menuItems(item, depth),
                                                };
                                            }}
                                            title="More…"
                                            aria-label="More actions"
                                        >
                                            <Icon name="more" size={14} />
                                        </button>
                                    </div>
                                {/if}
                            {/if}
                        </div>
                    {/each}

                    {#if creatingIn === col.id && depth === columns.length - 1}
                        <div class="row">
                            <span class="shrink-0 opacity-80"><Icon name="folder" size={15} /></span>
                            <input
                                bind:value={newFolderName}
                                placeholder="New folder name"
                                class="field flex-1 min-w-0 py-1 text-sm"
                                use:focusOnMount
                                onclick={(e) => e.stopPropagation()}
                                onblur={commitCreateFolder}
                                onkeydown={(e) => {
                                    e.stopPropagation();
                                    if (e.key === "Enter") commitCreateFolder();
                                    else if (e.key === "Escape") {
                                        creatingIn = undefined;
                                        newFolderName = "";
                                    }
                                }}
                            />
                        </div>
                    {/if}

                    {#if col.items.length === 0 && creatingIn !== col.id}
                        <p class="px-3 py-6 text-center text-xs text-[color:var(--text-faint)]">
                            Empty
                        </p>
                    {/if}

                    <!-- Clicking the empty space below a column closes the
                         columns to its right, the way a file browser does. -->
                    <button
                        class="w-full min-h-[80px] flex-1 cursor-default"
                        tabindex="-1"
                        aria-label="Clear the selection in {col.name}"
                        onclick={() => deselect(depth)}
                    ></button>
                </div>
            </div>
        {/each}
    </div>

    <div
        class="shrink-0 h-11 border-t border-[color:var(--border)] bg-[color:var(--surface)] flex items-center gap-2 px-3"
    >
        <button
            class="btn"
            onclick={() => {
                appState.addVideoFolderId =
                    appState.selectionPath.length > 0
                        ? appState.selectionPath[appState.selectionPath.length - 1]
                        : null;
                appState.isAddVideoModalOpen = true;
            }}
        >
            <Icon name="plus" size={14} />
            Add video
        </button>
        <button class="btn" onclick={startCreateFolder}>
            <Icon name="folderPlus" size={14} />
            New folder
        </button>

        <div class="flex-1"></div>

        <span class="text-[11px] text-[color:var(--text-faint)] hidden lg:flex items-center gap-1.5">
            <kbd class="kbd">Ctrl</kbd><kbd class="kbd">K</kbd> search ·
            <kbd class="kbd">F2</kbd> rename · drag to move
        </span>
    </div>
</div>

<FolderPicker
    bind:this={folderPickerRef}
    open={isFolderPickerOpen}
    title={folderPickerTitle}
    excludeFolderId={folderPickerExcludedFolder}
    onClose={() => (isFolderPickerOpen = false)}
/>
