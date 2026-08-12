<script lang="ts">
    import { appState } from "./state.svelte";
    import { onDestroy } from "svelte";
    import {
        type Video,
        type Folder,
        saveFolder,
        updateVideoFolder,
        updateFolderParent,
        deleteFolder,
        deleteVideo,
        renameFolder,
        renameVideo,
        updateSortOrder,
        formatTime,
    } from "./db";

    let draggingVideoId = $state<string | null>(null);
    let draggingFolderId = $state<number | null>(null);
    let draggingOverId = $state<number | string | null>(null); // For visual feedback
    let draggingOverType = $state<
        "move-inside" | "reorder-before" | "reorder-after" | null
    >(null);

    // Dynamic Columns Generation
    // Column 0: Root Items (parent_id === null)
    // Column N: Items where parent_id === selectionPath[N-1]
    let columns = $derived.by(() => {
        const cols = [];

        // Root Column
        cols.push({
            id: null as number | null, // Root
            folders: appState.folders
                .filter((f) => f.parent_id === null)
                .sort((a, b) => a.sort_order - b.sort_order),
            videos: appState.videos
                .filter((v) => v.folder_id === null)
                .sort((a, b) => a.sort_order - b.sort_order),
        });

        // Nested Columns
        for (let i = 0; i < appState.selectionPath.length; i++) {
            const parentId = appState.selectionPath[i];
            const folders = appState.folders
                .filter((f) => f.parent_id === parentId)
                .sort((a, b) => a.sort_order - b.sort_order);
            const videos = appState.videos
                .filter((v) => v.folder_id === parentId)
                .sort((a, b) => a.sort_order - b.sort_order);

            cols.push({
                id: parentId,
                folders,
                videos,
            });
        }

        return cols;
    });

    async function handleCreateFolder() {
        // Create in the currently active folder (the last one in selectionPath)
        const parentId =
            appState.selectionPath.length > 0
                ? appState.selectionPath[appState.selectionPath.length - 1]
                : null;
        const name = prompt("Folder Name:");
        if (name) {
            await saveFolder({
                name,
                created_at: Date.now(),
                parent_id: parentId,
                sort_order: 0,
            });
            await appState.refreshFolders();
        }
    }

    function selectFolder(folderId: number, depth: number) {
        // Truncate path to current depth, then append new folder
        const newPath = appState.selectionPath.slice(0, depth);
        newPath.push(folderId);
        appState.openFolder(newPath);
    }

    function deselectColumn(depth: number) {
        // Clicked on empty space in column 'depth'. Keep path up to depth.
        const newPath = appState.selectionPath.slice(0, depth);
        appState.openFolder(newPath);
    }

    async function handleDeleteFolder(id: number, e: MouseEvent) {
        e.stopPropagation();
        if (confirm("Delete folder and move items to root?")) {
            await deleteFolder(id);
            await appState.refreshFolders();
            await appState.refreshVideos();

            // If the deleted folder was in our path, we must go up.
            const idx = appState.selectionPath.indexOf(id);
            if (idx !== -1) {
                const newPath = appState.selectionPath.slice(0, idx);
                appState.openFolder(newPath);
            }
        }
    }

    async function handleDeleteVideo(id: string, e: MouseEvent) {
        e.stopPropagation();
        if (confirm("Delete video and all its clips?")) {
            await deleteVideo(id);
            await appState.refreshVideos();
        }
    }

    // Drag & Drop
    function onDragStartVideo(e: DragEvent, videoId: string) {
        appState.contextMenu.show = false;
        if (e.dataTransfer) {
            e.dataTransfer.setData(
                "text/plain",
                JSON.stringify({ type: "video", id: videoId }),
            );
            e.dataTransfer.effectAllowed = "move";
        }
        draggingVideoId = videoId;
        draggingFolderId = null;
    }

    function onDragStartFolder(e: DragEvent, folderId: number) {
        appState.contextMenu.show = false;
        if (e.dataTransfer) {
            e.dataTransfer.setData(
                "text/plain",
                JSON.stringify({ type: "folder", id: folderId }),
            );
            e.dataTransfer.effectAllowed = "move";
        }
        draggingFolderId = folderId;
        draggingVideoId = null;
    }

    async function handleReorder(
        draggedId: number | string,
        targetId: number | string | null,
        type: "video" | "folder",
        parentId: number | null,
    ) {
        // Get all items in current context
        let items: { id: number | string | null; sort_order: number }[] = [];

        if (type === "folder") {
            const list = appState.folders.filter(
                (f) => f.parent_id === parentId,
            );
            items = list.map((f) => ({ id: f.id!, sort_order: f.sort_order }));
        } else {
            const list = appState.videos.filter(
                (v) => v.folder_id === parentId,
            );
            items = list.map((v) => ({ id: v.id!, sort_order: v.sort_order }));
        }

        // Sort current list by sort_order
        items.sort((a, b) => a.sort_order - b.sort_order);

        const draggedIndex = items.findIndex((i) => i.id === draggedId);
        if (draggedIndex === -1) return;

        // Remove dragged item
        const [draggedItem] = items.splice(draggedIndex, 1);

        // Find insertion index
        let insertIndex = items.length; // Default to end
        if (targetId !== null) {
            insertIndex = items.findIndex((i) => i.id === targetId);
            if (insertIndex === -1) insertIndex = items.length;
        }

        // Insert
        items.splice(insertIndex, 0, draggedItem);

        // Reassign sort orders
        const updates: any[] = [];
        items.forEach((item, index) => {
            if (item.sort_order !== index) {
                updates.push({ id: item.id!, sort_order: index });
            }
        });

        if (updates.length > 0) {
            if (type === "folder") {
                await updateSortOrder(updates, []);
            } else {
                await updateSortOrder([], updates);
            }
            if (type === "video") await appState.refreshVideos();
            else await appState.refreshFolders();
        }
    }

    let clearDragTimeout: ReturnType<typeof setTimeout>;

    onDestroy(() => {
        clearTimeout(clearDragTimeout);
    });

    function handleDragOverItem(
        e: DragEvent,
        itemId: number | string,
        type: "video" | "folder",
    ) {
        e.preventDefault();
        e.stopPropagation();
        clearTimeout(clearDragTimeout);

        const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
        const offsetY = e.clientY - rect.top;
        const height = rect.height;

        let action: "reorder-before" | "reorder-after" | "move-inside" | null =
            null;

        if (type === "folder") {
            if (offsetY < height * 0.25) {
                action = "reorder-before";
            } else if (offsetY > height * 0.75) {
                action = "reorder-after";
            } else {
                action = "move-inside";
            }
        } else {
            if (offsetY < height * 0.5) {
                action = "reorder-before";
            } else {
                action = "reorder-after";
            }
        }

        if (draggingOverId !== itemId || draggingOverType !== action) {
            draggingOverId = itemId;
            draggingOverType = action;
        }
    }

    async function handleDropOnItem(
        e: DragEvent,
        targetId: number | string,
        targetType: "video" | "folder",
        parentId: number | null,
    ) {
        e.stopPropagation();
        clearTimeout(clearDragTimeout);

        if (!draggingOverType) {
            draggingOverId = null;
            draggingOverType = null;
            return;
        }

        const action = draggingOverType;

        // Cleanup visuals
        draggingOverId = null;
        draggingOverType = null;

        const draggedId = draggingVideoId ?? draggingFolderId;
        if (!draggedId) return;
        const draggedType = draggingVideoId ? "video" : "folder";

        if (action === "reorder-before" || action === "reorder-after") {
            // Handle Reorder
            // Can't reorder video relative to folder or vice versa easily in current list logic
            // But UI separates them?
            // Logic: `handleReorder` sorts items of `type` relative to `targetId`.
            // If I drag Video -> Folder (reorder), it sends "video" type and "folder" ID?
            // No, `handleReorder` expects `type` and `targetId`.
            // But my lists are separated: Folders first, then Videos.
            // If I drag Video onto Folder "Reorder", what does that mean?
            // It means putting Video in the Folder list? No.
            // If I drag Video onto Folder "Inside" -> Move.

            // Constraint: Can only reorder Video rel to Video, Folder rel to Folder.
            if (draggedType === targetType) {
                let actualTargetId: number | string | null = targetId;

                if (action === "reorder-after") {
                    // Find next item
                    let items: { id: number | string | null; sort_order: number }[] = [];
                    if (targetType === "folder") {
                        const list = appState.folders.filter(
                            (f) => f.parent_id === parentId,
                        );
                        items = list.map((f) => ({
                            id: f.id!,
                            sort_order: f.sort_order,
                        }));
                    } else {
                        const list = appState.videos.filter(
                            (v) => v.folder_id === parentId,
                        );
                        items = list.map((v) => ({
                            id: v.id!,
                            sort_order: v.sort_order,
                        }));
                    }
                    items.sort((a, b) => a.sort_order - b.sort_order);
                    const currentIndex = items.findIndex(
                        (i) => i.id === targetId,
                    );
                    if (
                        currentIndex !== -1 &&
                        currentIndex < items.length - 1
                    ) {
                        actualTargetId = items[currentIndex + 1].id;
                    } else {
                        actualTargetId = null; // Sort at end
                    }
                }

                await handleReorder(
                    draggedId,
                    actualTargetId,
                    draggedType,
                    parentId,
                );
            } else {
                // Mismatch type reordering ignored
            }
        } else if (action === "move-inside") {
            if (targetType === "folder") {
                // Move dragged item (video or folder) into target folder
                if (draggedType === "folder") {
                    draggingFolderId = draggedId as number;
                    await updateFolderParent(draggedId as number, targetId as number | null);
                } else {
                    draggingVideoId = draggedId as string;
                    await updateVideoFolder(draggedId as string, targetId as number | null);
                }
                await appState.refreshFolders();
                await appState.refreshVideos();
            }
        }
    }

    function handleDragLeaveItem(e: DragEvent, itemId: number | string) {
        // Only clear if we are leaving the item entirely, not entering a child
        const currentTarget = e.currentTarget as HTMLElement;
        const relatedTarget = e.relatedTarget as Node | null;

        if (currentTarget.contains(relatedTarget)) {
            return;
        }

        clearTimeout(clearDragTimeout);
        clearDragTimeout = setTimeout(() => {
            if (draggingOverId === itemId) {
                draggingOverId = null;
                draggingOverType = null;
            }
        }, 50);
    }

    async function handleDropOnColumn(columnId: number | null) {
        clearTimeout(clearDragTimeout);
        draggingOverId = null;
        draggingOverType = null;

        if (draggingVideoId) {
            await updateVideoFolder(draggingVideoId, columnId);
            await appState.refreshVideos();
            draggingVideoId = null;
        } else if (draggingFolderId) {
            if (draggingFolderId === columnId) return;
            await updateFolderParent(draggingFolderId, columnId);
            await appState.refreshFolders();
            draggingFolderId = null;
        }
    }
</script>

<div class="h-full flex flex-col bg-black overflow-hidden">
    <!-- Miller Columns Container -->
    <div class="flex-1 flex overflow-x-auto overflow-y-hidden bg-black border-b border-zinc-900">
        {#each columns as col, depth}
            <div
                class="
                    flex flex-col border-r border-zinc-800 bg-zinc-950/30 overflow-y-auto outline-none
                    {depth === columns.length - 1
                    ? 'flex-1 w-full min-w-[280px]'
                    : 'hidden md:flex w-[280px] min-w-[280px] max-w-[280px]'}
                "
                ondragover={(e) => e.preventDefault()}
                ondrop={(e) => {
                    e.stopPropagation();
                    handleDropOnColumn(col.id);
                }}
                onclick={() => deselectColumn(depth)}
                onkeydown={(e) => e.key === "Enter" && deselectColumn(depth)}
                role="button"
                aria-label="Column {depth} (Click to Deselect)"
                tabindex="0"
            >
                <!-- Folders List -->
                {#each col.folders as folder (folder.id)}
                    <div
                        role="button"
                        tabindex="0"
                        draggable="true"
                        ondragstart={(e) => onDragStartFolder(e, folder.id!)}
                        onclick={(e) => {
                            e.stopPropagation();
                            selectFolder(folder.id!, depth);
                        }}
                        onkeydown={(e) =>
                            e.key === "Enter" &&
                            selectFolder(folder.id!, depth)}
                        ondragover={(e) =>
                            handleDragOverItem(e, folder.id!, "folder")}
                        ondragleave={(e) => handleDragLeaveItem(e, folder.id!)}
                        ondrop={(e) =>
                            handleDropOnItem(e, folder.id!, "folder", col.id)}
                        class="h-10 px-3 flex items-center justify-between cursor-default select-none group shrink-0 relative transition-colors {appState
                            .selectionPath[depth] === folder.id
                            ? 'bg-blue-600 text-white'
                            : 'hover:bg-zinc-800 text-zinc-300'}
                             {draggingOverId === folder.id! &&
                        draggingOverType === 'move-inside'
                            ? '!bg-blue-600/30'
                            : ''}
                             {draggingOverId === folder.id! &&
                        draggingOverType === 'reorder-before'
                            ? '!border-t-4 !border-blue-500 z-10'
                            : 'border-t-2 border-transparent'}
                            {draggingOverId === folder.id! &&
                        draggingOverType === 'reorder-after'
                            ? '!border-b-4 !border-blue-500 z-10'
                            : 'border-b-2 border-transparent'}"
                    >
                        <div
                            class="flex items-center gap-2 truncate pointer-events-none"
                        >
                            <svg
                                class="size-4 opacity-70 flex-shrink-0"
                                fill="currentColor"
                                viewBox="0 0 24 24"
                                ><path
                                    d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
                                /></svg
                            >
                            <span class="truncate text-sm">{folder.name}</span>
                        </div>

                        <div class="flex items-center pointer-events-auto">
                            {#if appState.selectionPath.includes(folder.id!)}
                                <svg
                                    class="size-4 opacity-50"
                                    fill="none"
                                    viewBox="0 0 24 24"
                                    stroke="currentColor"
                                    ><path
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        stroke-width="2"
                                        d="M9 5l7 7-7 7"
                                    /></svg
                                >
                            {:else}
                                <button
                                    onclick={(e) =>
                                        handleDeleteFolder(folder.id!, e)}
                                    class="p-1 hover:bg-black/20 rounded mr-1 opacity-50 hover:opacity-100 transition-opacity"
                                    aria-label="Delete Folder"
                                >
                                    <svg
                                        class="size-3"
                                        fill="none"
                                        viewBox="0 0 24 24"
                                        stroke="currentColor"
                                        ><path
                                            stroke-linecap="round"
                                            stroke-linejoin="round"
                                            stroke-width="2"
                                            d="M6 18L18 6M6 6l12 12"
                                        /></svg
                                    >
                                </button>
                            {/if}
                        </div>
                    </div>
                {/each}

                <!-- Videos -->
                {#each col.videos as video (video.id)}
                    <div
                        role="button"
                        tabindex="0"
                        draggable="true"
                        ondragstart={(e) => onDragStartVideo(e, video.id!)}
                        onclick={(e) => {
                            e.stopPropagation();
                            appState.openVideo(video);
                        }}
                        onkeydown={(e) =>
                            e.key === "Enter" && appState.openVideo(video)}
                        ondragover={(e) =>
                            handleDragOverItem(e, video.id!, "video")}
                        ondragleave={(e) => handleDragLeaveItem(e, video.id!)}
                        ondrop={(e) =>
                            handleDropOnItem(e, video.id!, "video", col.id)}
                        class="h-10 px-3 flex items-center justify-between cursor-pointer select-none group hover:bg-zinc-800 text-zinc-400 hover:text-zinc-200 shrink-0 relative transition-colors
                            {draggingOverId === video.id! &&
                        draggingOverType === 'reorder-before'
                            ? '!border-t-4 !border-blue-500 z-10'
                            : 'border-t-2 border-transparent'}
                            {draggingOverId === video.id! &&
                        draggingOverType === 'reorder-after'
                            ? '!border-b-4 !border-blue-500 z-10'
                            : 'border-b-2 border-transparent'}"
                    >
                        <!-- Video Drop Zone (Implicit: Drop on Video = Reorder Before) -->

                        <div
                            class="flex items-center gap-2 truncate flex-1 pointer-events-none"
                        >
                            <img
                                src={video.thumbnail_url}
                                alt=""
                                class="w-8 h-5 object-cover rounded bg-zinc-800"
                            />
                            <span class="truncate text-sm font-medium"
                                >{video.title}</span
                            >
                            <span class="text-xs text-zinc-600"
                                >{video.last_position > 0 ? formatTime(video.last_position) : (video.duration > 0 ? formatTime(video.duration) : "")}</span
                            >
                        </div>

                        <div class="flex items-center pointer-events-auto">
                            <button
                                onclick={(e) => handleDeleteVideo(video.id!, e)}
                                class="p-1 hover:bg-black/20 rounded mr-1 opacity-50 hover:opacity-100 transition-opacity"
                                aria-label="Delete Video"
                            >
                                <svg
                                    class="size-3"
                                    fill="none"
                                    viewBox="0 0 24 24"
                                    stroke="currentColor"
                                    ><path
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        stroke-width="2"
                                        d="M6 18L18 6M6 6l12 12"
                                    /></svg
                                >
                            </button>
                        </div>
                    </div>
                {/each}

                {#if col.folders.length === 0 && col.videos.length === 0}
                    <div
                        class="flex-1 flex items-center justify-center pointer-events-none opacity-20"
                    >
                        <span class="text-xs">Empty</span>
                    </div>
                {/if}
            </div>
        {/each}
    </div>

    <!-- Header Controls (Footer) -->
    <div
        class="min-h-12 flex items-center justify-between px-4 border-t border-zinc-800 bg-zinc-900 select-none grid-area-footer flex-wrap gap-2"
    >
        <div class="flex items-center gap-2 overflow-hidden">
            <button
                onclick={() => appState.openFolder([])}
                class="text-xs font-bold text-zinc-500 tracking-wider uppercase hover:text-zinc-300 transition-colors"
                >Library</button
            >
            {#if appState.selectionPath.length > 0}
                <svg
                    class="size-3 text-zinc-600"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                    ><path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M9 5l7 7-7 7"
                    /></svg
                >
                <div
                    class="flex gap-1 text-xs text-zinc-400 overflow-x-auto whitespace-nowrap scrollbar-none"
                >
                    {#each appState.selectionPath as fid, i}
                        <button
                            onclick={() => selectFolder(fid, i)}
                            class="hover:text-white transition-colors cursor-pointer"
                        >
                            {appState.folders.find((f) => f.id === fid)?.name}
                        </button>
                        {#if i < appState.selectionPath.length - 1}
                            <span class="text-zinc-600">/</span>
                        {/if}
                    {/each}
                </div>
            {/if}
        </div>

        <div class="flex gap-4 items-center">
            <!-- Sync Status -->
            {#if appState.syncStatus === "syncing"}
                <div class="flex items-center gap-2 text-xs text-blue-500">
                    <svg
                        class="size-3 animate-spin"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                        ><path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                        /></svg
                    >
                    <span>Syncing...</span>
                </div>
            {:else if appState.syncStatus === "success"}
                <div class="flex items-center gap-2 text-xs text-green-500">
                    <svg
                        class="size-3"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                        ><path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M5 13l4 4L19 7"
                        /></svg
                    >
                    <span>Synced</span>
                </div>
            {:else if appState.syncStatus === "error"}
                <div
                    class="flex items-center gap-2 text-xs text-red-500"
                    title={appState.syncError || "Unknown error"}
                >
                    <svg
                        class="size-3"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                        ><path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                        /></svg
                    >
                    <span>Sync Error</span>
                </div>
            {/if}

            <div class="flex gap-2">
                <button
                    onclick={() => {
                        appState.addVideoFolderId = appState.selectionPath.length > 0 
                            ? appState.selectionPath[appState.selectionPath.length - 1] 
                            : null;
                        appState.isAddVideoModalOpen = true;
                    }}
                    class="p-1 hover:bg-zinc-800 rounded text-zinc-400 hover:text-white"
                    title="Add Video"
                >
                    <svg
                        class="size-5"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                        ><path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M12 4v16m8-8H4"
                        /></svg
                    >
                </button>
                {#if appState.settings.githubToken && appState.settings.githubRepo}
                    <button
                        onclick={() => appState.triggerSync()}
                        disabled={appState.syncStatus === "syncing"}
                        class="p-1 hover:bg-zinc-800 rounded text-zinc-400 hover:text-white disabled:opacity-50 disabled:cursor-not-allowed"
                        title="Sync to GitHub"
                    >
                        <svg
                            class="size-5 {appState.syncStatus === 'syncing' ? 'animate-spin' : ''}"
                            fill="none"
                            viewBox="0 0 24 24"
                            stroke="currentColor"
                        >
                            <path
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                stroke-width="2"
                                d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                            />
                        </svg>
                    </button>
                {/if}
                <button
                    onclick={handleCreateFolder}
                    class="p-1 hover:bg-zinc-800 rounded text-zinc-400 hover:text-white"
                    title="New Folder"
                >
                    <svg
                        class="size-5"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                        ><path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M9 13h6m-3-3v6m-9 1V7a2 2 0 012-2h6l2 2h6a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2z"
                        /></svg
                    >
                </button>
                <button
                    onclick={() => (appState.isSettingsModalOpen = true)}
                    class="p-1 hover:bg-zinc-800 rounded text-zinc-400 hover:text-white"
                    title="Settings"
                >
                    <svg
                        class="size-5"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                        ><path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                        /></svg
                    >
                </button>
            </div>
        </div>
    </div>
</div>
