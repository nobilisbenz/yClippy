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

    /// One library, two densities.
    ///
    /// The desktop rail and the phone were separate components that had to be
    /// kept in step by hand and never were: the phone could not move an item
    /// and the rail could not be reached with a thumb. This is one drill-down
    /// tree — the Miller model with the ancestors folded into the breadcrumb —
    /// where `density` is the only thing that differs between them.
    let {
        density = "compact",
        showActions = true,
    }: {
        density?: "compact" | "touch";
        showActions?: boolean;
    } = $props();

    const touch = $derived(density === "touch");

    let renamingId = $state<string | null>(null);
    let renamingText = $state("");
    let creatingFolder = $state(false);
    let newFolderName = $state("");
    let reordering = $state(false);
    let query = $state("");
    let searching = $state(false);
    let listEl = $state<HTMLElement | null>(null);

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

    type Item = { kind: "video"; video: Video } | { kind: "folder"; folder: Folder };

    const currentFolderId = $derived(
        appState.selectionPath.length > 0
            ? appState.selectionPath[appState.selectionPath.length - 1]
            : null,
    );

    const currentFolder = $derived(
        appState.folders.find((f) => f.id === currentFolderId) ?? null,
    );

    const items = $derived.by<Item[]>(() => {
        const q = query.trim().toLowerCase();
        const out: Item[] = [];
        for (const f of appState.folders
            .filter((f) => f.parent_id === currentFolderId)
            .sort((a, b) => a.sort_order - b.sort_order)) {
            if (!q || f.name.toLowerCase().includes(q)) out.push({ kind: "folder", folder: f });
        }
        for (const v of appState.videos
            .filter((v) => v.folder_id === currentFolderId)
            .sort((a, b) => a.sort_order - b.sort_order)) {
            if (!q || v.title.toLowerCase().includes(q)) out.push({ kind: "video", video: v });
        }
        return out;
    });

    function childCount(folderId: number): number {
        return (
            appState.folders.filter((f) => f.parent_id === folderId).length +
            appState.videos.filter((v) => v.folder_id === folderId).length
        );
    }

    function keyOf(item: Item): string {
        return item.kind === "folder" ? `f-${item.folder.id}` : `v-${item.video.id}`;
    }

    // ── navigation ──────────────────────────────────────────────────────

    function openFolder(folder: Folder) {
        appState.openFolder([...appState.selectionPath, folder.id!]);
        query = "";
    }

    function goUp() {
        if (appState.selectionPath.length > 0) {
            appState.openFolder(appState.selectionPath.slice(0, -1));
        }
    }

    /// Up and down move focus, left goes up a level, right or Enter opens.
    /// Without this the tree is only reachable by Tab, one row at a time.
    function onListKeyDown(e: KeyboardEvent) {
        if (e.key !== "ArrowDown" && e.key !== "ArrowUp" && e.key !== "ArrowLeft") return;
        const rows = Array.from(
            listEl?.querySelectorAll<HTMLElement>("[data-row]") ?? [],
        );
        const active = document.activeElement as HTMLElement | null;
        const index = rows.findIndex((row) => row === active || row.contains(active));
        if (e.key === "ArrowLeft") {
            e.preventDefault();
            goUp();
            return;
        }
        e.preventDefault();
        const next = e.key === "ArrowDown" ? index + 1 : index - 1;
        rows[Math.min(rows.length - 1, Math.max(0, next))]?.focus();
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

    async function commitCreateFolder() {
        const name = newFolderName.trim();
        creatingFolder = false;
        newFolderName = "";
        if (!name) return;
        const maxOrder = appState.folders
            .filter((f) => f.parent_id === currentFolderId)
            .reduce((max, f) => Math.max(max, f.sort_order), -1);
        try {
            await saveFolder({
                name,
                created_at: Date.now(),
                parent_id: currentFolderId,
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

    /// Renumbers the whole list on every move; writing only the two swapped
    /// rows leaves the rest with stale or duplicate orders.
    async function reorder(from: number, to: number) {
        if (to < 0 || to >= items.length || from === to) return;
        const ordered = [...items];
        const [lifted] = ordered.splice(from, 1);
        ordered.splice(to, 0, lifted);

        const folders: { id: number; sort_order: number }[] = [];
        const videos: { id: string; sort_order: number }[] = [];
        ordered.forEach((item, position) => {
            if (item.kind === "folder" && item.folder.id !== undefined) {
                folders.push({ id: item.folder.id, sort_order: position });
            } else if (item.kind === "video") {
                videos.push({ id: item.video.id, sort_order: position });
            }
        });

        try {
            await updateSortOrder(folders, videos);
            await appState.refreshAll();
        } catch (e) {
            appState.showToast(`Could not reorder: ${String(e)}`, "error");
        }
    }

    // ── menus ───────────────────────────────────────────────────────────

    function menuItems(item: Item) {
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
            { label: "Open", action: () => openFolder(folder) },
            { label: "Rename", action: () => startRename(item) },
            { label: "Move to…", action: () => askMove(item) },
            { label: "Delete", danger: true, action: () => removeFolder(folder) },
        ];
    }

    function openMenuAt(item: Item, x: number, y: number) {
        appState.contextMenu = { x, y, items: menuItems(item), show: true };
    }

    let longPressTimer: number | undefined;
    function startLongPress(item: Item, e: TouchEvent) {
        if (reordering) return;
        const el = e.currentTarget as HTMLElement;
        longPressTimer = setTimeout(() => {
            const rect = el.getBoundingClientRect();
            openMenuAt(item, rect.left + rect.width / 2, rect.top + rect.height / 2);
            navigator.vibrate?.(40);
        }, 450) as unknown as number;
    }
    function cancelLongPress() {
        if (longPressTimer !== undefined) {
            clearTimeout(longPressTimer);
            longPressTimer = undefined;
        }
    }

    // ── drag and drop (pointer devices only) ────────────────────────────

    let dragging = $state<Item | null>(null);
    let dropTarget = $state<string | null>(null);
    let dropMode = $state<"into" | "before" | "after" | null>(null);

    function onDragStart(item: Item) {
        appState.contextMenu.show = false;
        dragging = item;
    }

    function onDragOver(e: DragEvent, item: Item) {
        if (!dragging || keyOf(dragging) === keyOf(item)) return;
        e.preventDefault();
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
    }

    async function onDrop(e: DragEvent, item: Item) {
        e.preventDefault();
        e.stopPropagation();
        const source = dragging;
        const mode = dropMode;
        dragging = null;
        dropTarget = null;
        dropMode = null;
        if (!source || !mode) return;

        if (mode === "into" && item.kind === "folder") {
            try {
                if (source.kind === "folder") {
                    if (source.folder.id === item.folder.id) return;
                    await updateFolderParent(source.folder.id!, item.folder.id!);
                } else {
                    await updateVideoFolder(source.video.id, item.folder.id!);
                }
                await appState.refreshAll();
            } catch (err) {
                appState.showToast(`Could not move: ${String(err)}`, "error");
            }
            return;
        }

        const from = items.findIndex((i) => keyOf(i) === keyOf(source));
        let to = items.findIndex((i) => keyOf(i) === keyOf(item));
        if (from === -1 || to === -1) return;
        if (mode === "after") to += 1;
        if (from < to) to -= 1;
        await reorder(from, to);
    }

    async function dropOnParent() {
        const source = dragging;
        dragging = null;
        dropTarget = null;
        dropMode = null;
        if (!source) return;
        const parent =
            appState.selectionPath.length > 1
                ? appState.selectionPath[appState.selectionPath.length - 2]
                : null;
        try {
            if (source.kind === "folder") await updateFolderParent(source.folder.id!, parent);
            else await updateVideoFolder(source.video.id, parent);
            await appState.refreshAll();
            appState.showToast("Moved up one level", "success");
        } catch (e) {
            appState.showToast(`Could not move: ${String(e)}`, "error");
        }
    }
</script>

<div class="h-full flex flex-col min-h-0 bg-[color:var(--surface)]">
    <!-- Where you are, and the way back. The breadcrumb used to sit in a
         wrapping footer, which is the one place a back control cannot be. -->
    <header class="panel-head gap-1" style={touch ? "height: 56px" : ""}>
        {#if searching}
            <!-- svelte-ignore a11y_autofocus -->
            <input
                bind:value={query}
                autofocus
                placeholder="Filter this folder…"
                class="field py-1 text-sm"
                onkeydown={(e) => {
                    if (e.key === "Escape") {
                        query = "";
                        searching = false;
                    }
                }}
            />
            <button
                class="icon-btn"
                class:icon-btn-touch={touch}
                onclick={() => {
                    query = "";
                    searching = false;
                }}
                aria-label="Close filter"><Icon name="close" size={15} /></button
            >
        {:else}
            {#if appState.selectionPath.length > 0}
                <button
                    class="icon-btn"
                    class:icon-btn-touch={touch}
                    onclick={goUp}
                    ondragover={(e) => e.preventDefault()}
                    ondrop={dropOnParent}
                    title="Up one level"
                    aria-label="Up one level"
                >
                    <Icon name="chevronLeft" size={touch ? 20 : 16} />
                </button>
            {/if}
            <button
                class="flex-1 min-w-0 text-left truncate {touch ? 'text-base' : 'text-[13px]'}"
                onclick={() => appState.openFolder([])}
                title="Go to the top of the library"
            >
                {#if currentFolder}
                    <span class="text-[color:var(--text)]">{currentFolder.name}</span>
                {:else}
                    <span class="section-label">Library</span>
                {/if}
            </button>
            <span class="chip">{items.length}</span>
            <button
                class="icon-btn"
                class:icon-btn-touch={touch}
                onclick={() => (searching = true)}
                title="Filter this folder"
                aria-label="Filter this folder"><Icon name="search" size={15} /></button
            >
        {/if}
    </header>

    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
        bind:this={listEl}
        class="flex-1 min-h-0 overflow-y-auto scroll-thin"
        onkeydown={onListKeyDown}
    >
        {#if items.length === 0 && !creatingFolder}
            <div class="empty">
                <span class="text-[color:var(--text-faint)]"><Icon name="folder" size={24} /></span>
                {#if query}
                    <p>Nothing matches “{query}”</p>
                {:else}
                    <p>This folder is empty</p>
                    <p class="text-[color:var(--text-faint)]">
                        Add a video with the + below.
                    </p>
                {/if}
            </div>
        {/if}

        {#each items as item, index (keyOf(item))}
            {@const isRenaming = renamingId === keyOf(item)}
            {@const isTarget = dropTarget === keyOf(item)}
            <div class="flex items-stretch relative">
                {#if reordering}
                    <div class="flex flex-col justify-center shrink-0 border-r border-[color:var(--border)]">
                        <button
                            class="icon-btn"
                            style="--size: 36px"
                            disabled={index === 0}
                            onclick={() => reorder(index, index - 1)}
                            aria-label="Move up"><Icon name="arrowUp" size={15} /></button
                        >
                        <button
                            class="icon-btn"
                            style="--size: 36px"
                            disabled={index === items.length - 1}
                            onclick={() => reorder(index, index + 1)}
                            aria-label="Move down"><Icon name="arrowDown" size={15} /></button
                        >
                    </div>
                {/if}

                <div
                    data-row
                    role="button"
                    tabindex="0"
                    draggable={!touch && !reordering && !isRenaming}
                    ondragstart={() => onDragStart(item)}
                    ondragover={(e) => onDragOver(e, item)}
                    ondragleave={() => {
                        if (dropTarget === keyOf(item)) dropTarget = null;
                    }}
                    ondrop={(e) => onDrop(e, item)}
                    ondragend={() => {
                        dragging = null;
                        dropTarget = null;
                    }}
                    ontouchstart={(e) => startLongPress(item, e)}
                    ontouchend={cancelLongPress}
                    ontouchmove={cancelLongPress}
                    onclick={() => {
                        if (isRenaming) return;
                        if (item.kind === "folder") openFolder(item.folder);
                        else appState.openVideo(item.video);
                    }}
                    onkeydown={(e) => {
                        if (e.key === "Enter" || e.key === "ArrowRight") {
                            if (item.kind === "folder") openFolder(item.folder);
                            else if (e.key === "Enter") appState.openVideo(item.video);
                        } else if (e.key === "F2") {
                            startRename(item);
                        } else if (e.key === "Delete") {
                            if (item.kind === "folder") removeFolder(item.folder);
                            else removeVideo(item.video);
                        }
                    }}
                    oncontextmenu={(e) => {
                        e.preventDefault();
                        e.stopPropagation();
                        openMenuAt(item, e.clientX, e.clientY);
                    }}
                    class="row flex-1 min-w-0 cursor-pointer
                           {touch ? 'row-touch' : ''}
                           {dragging && keyOf(dragging) === keyOf(item) ? 'opacity-40' : ''}
                           {isTarget && dropMode === 'into' ? 'bg-[color:var(--accent-soft)]' : ''}"
                    style={isTarget && dropMode === "before"
                        ? "border-top-color: var(--accent)"
                        : isTarget && dropMode === "after"
                          ? "border-bottom-color: var(--accent)"
                          : ""}
                >
                    {#if item.kind === "folder"}
                        <span class="shrink-0 text-[color:var(--text-faint)]">
                            <Icon name="folder" size={touch ? 20 : 15} />
                        </span>
                    {:else}
                        <Thumbnail
                            videoId={item.video.id}
                            alt=""
                            className="{touch
                                ? 'w-20 h-11'
                                : 'w-11 h-[25px]'} object-cover rounded-[3px] bg-black shrink-0"
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
                            <div class="truncate {touch ? 'text-[15px]' : 'text-[13px]'}">
                                {item.kind === "folder" ? item.folder.name : item.video.title}
                            </div>
                            {#if item.kind === "video" && item.video.last_position > 0}
                                <div
                                    class="text-[11px] t-num text-[color:var(--text-faint)] flex items-center gap-1"
                                >
                                    <Icon name="play" size={9} />
                                    {formatClock(item.video.last_position)}
                                </div>
                            {/if}
                        </div>

                        {#if item.kind === "folder"}
                            <span class="chip">{childCount(item.folder.id!)}</span>
                            <span class="shrink-0 text-[color:var(--text-faint)]">
                                <Icon name="chevronRight" size={touch ? 18 : 14} />
                            </span>
                        {:else}
                            <div class="row-actions">
                                <button
                                    class="icon-btn"
                                    class:icon-btn-touch={touch}
                                    onclick={(e) => {
                                        e.stopPropagation();
                                        openMenuAt(item, e.clientX, e.clientY);
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
            </div>
        {/each}

        {#if creatingFolder}
            <div class="row {touch ? 'row-touch' : ''}">
                <span class="shrink-0 text-[color:var(--text-faint)]">
                    <Icon name="folder" size={touch ? 20 : 15} />
                </span>
                <input
                    bind:value={newFolderName}
                    placeholder="New folder name"
                    class="field flex-1 min-w-0 py-1 text-sm"
                    use:focusOnMount
                    onblur={commitCreateFolder}
                    onkeydown={(e) => {
                        if (e.key === "Enter") commitCreateFolder();
                        else if (e.key === "Escape") {
                            creatingFolder = false;
                            newFolderName = "";
                        }
                    }}
                />
            </div>
        {/if}
    </div>

    {#if showActions}
        <footer
            class="shrink-0 border-t border-[color:var(--border)] flex items-center gap-1 px-2"
            style="height: {touch ? 'var(--tap)' : '40px'}"
        >
            <button
                class="icon-btn"
                class:icon-btn-touch={touch}
                onclick={() => {
                    appState.addVideoFolderId = currentFolderId;
                    appState.isAddVideoModalOpen = true;
                }}
                title="Add a video"
                aria-label="Add a video"><Icon name="plus" size={touch ? 20 : 16} /></button
            >
            <button
                class="icon-btn"
                class:icon-btn-touch={touch}
                onclick={() => {
                    creatingFolder = true;
                    newFolderName = "";
                }}
                title="New folder"
                aria-label="New folder"><Icon name="folderPlus" size={touch ? 20 : 16} /></button
            >
            <button
                class="icon-btn"
                class:icon-btn-touch={touch}
                style={reordering
                    ? "color: var(--accent); background: var(--accent-soft)"
                    : ""}
                onclick={() => (reordering = !reordering)}
                title={reordering ? "Done reordering" : "Reorder"}
                aria-pressed={reordering}
                aria-label="Reorder"><Icon name="reorder" size={touch ? 20 : 16} /></button
            >

            <div class="flex-1"></div>

            {#if appState.settings.githubTokenPresent && appState.settings.githubRepo}
                <button
                    class="icon-btn"
                    class:icon-btn-touch={touch}
                    onclick={() => appState.triggerSync()}
                    disabled={appState.syncStatus === "syncing"}
                    title={appState.syncStatus === "error"
                        ? appState.syncError || "Sync failed"
                        : "Sync"}
                    aria-label="Sync"
                >
                    <span
                        class:animate-spin={appState.syncStatus === "syncing"}
                        style={appState.syncStatus === "error"
                            ? "color: var(--danger)"
                            : appState.syncStatus === "success"
                              ? "color: var(--success)"
                              : ""}
                    >
                        <Icon name="sync" size={touch ? 20 : 16} />
                    </span>
                </button>
            {/if}
            <button
                class="icon-btn"
                class:icon-btn-touch={touch}
                onclick={() => (appState.isSettingsModalOpen = true)}
                title="Settings"
                aria-label="Settings"><Icon name="settings" size={touch ? 20 : 16} /></button
            >
        </footer>
    {/if}
</div>

<FolderPicker
    bind:this={folderPickerRef}
    open={isFolderPickerOpen}
    title={folderPickerTitle}
    excludeFolderId={folderPickerExcludedFolder}
    onClose={() => (isFolderPickerOpen = false)}
/>
