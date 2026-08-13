<script lang="ts">
    import { appState } from "./state.svelte";
    import { type Video, type Folder, formatTime } from "./db";
    import Thumbnail from "./Thumbnail.svelte";
    import FolderPicker from "./FolderPicker.svelte";

    let { compact = false }: { compact?: boolean } = $props();

    let isAndroid = $derived.by(() => {
        if (typeof window === "undefined") return false;
        const ua = navigator.userAgent || "";
        return /android/i.test(ua);
    });

    let longPressTimer: number | undefined;
    let longPressTargetEl: HTMLElement | undefined;
    let renamingFolder = $state<Folder | null>(null);
    let renamingVideo = $state<Video | null>(null);
    let renamingText = $state("");
    let creatingFolder = $state(false);
    let newFolderName = $state("");
    let isFolderPickerOpen = $state(false);
    let folderPickerExcludedFolder = $state<number | undefined>(undefined);
    let folderPickerTitle = $state("Move to folder");
    let folderPickerRef = $state<{
        moveVideo: (v: Video) => void;
        moveFolder: (f: Folder) => void;
        setSelectedVideo: (v: Video) => void;
    } | undefined>();

    type Item = { kind: "video"; video: Video } | { kind: "folder"; folder: Folder };
    let breadcrumbFolders = $derived.by(() => {
        const chain: Folder[] = [];
        for (const id of appState.selectionPath) {
            const f = appState.folders.find((x) => x.id === id);
            if (f) chain.push(f);
        }
        return chain;
    });

    let currentFolderId = $derived(
        appState.selectionPath.length > 0
            ? appState.selectionPath[appState.selectionPath.length - 1]
            : null,
    );

    let items = $derived.by<Item[]>(() => {
        const out: Item[] = [];
        for (const f of appState.folders
            .filter((f) => f.parent_id === currentFolderId)
            .sort((a, b) => a.sort_order - b.sort_order)) {
            out.push({ kind: "folder", folder: f });
        }
        for (const v of appState.videos
            .filter((v) => v.folder_id === currentFolderId)
            .sort((a, b) => a.sort_order - b.sort_order)) {
            out.push({ kind: "video", video: v });
        }
        return out;
    });

    function startLongPress(target: Video | Folder, e: TouchEvent | MouseEvent) {
        longPressTargetEl = e.currentTarget as HTMLElement;
        const isVideo = "title" in target && "thumbnail_url" in target;
        longPressTimer = setTimeout(() => {
            let items: { label: string; action: () => void; danger?: boolean }[];
            if (isVideo) {
                const video = target as Video;
                items = [
                    { label: "Play", action: () => appState.openVideo(video) },
                    {
                        label: "Move to…",
                        action: () => {
                            folderPickerTitle = `Move "${video.title}"`;
                            if (folderPickerRef) folderPickerRef.moveVideo(video);
                            isFolderPickerOpen = true;
                        },
                    },
                    {
                        label: "Delete",
                        danger: true,
                        action: () => handleDeleteVideo(video.id, new MouseEvent("click")),
                    },
                ];
            } else {
                const folder = target as Folder;
                items = [
                    { label: "Open", action: () => appState.openFolder([...appState.selectionPath, folder.id!]) },
                    {
                        label: "Move to…",
                        action: () => {
                            folderPickerTitle = `Move folder "${folder.name}"`;
                            folderPickerExcludedFolder = folder.id;
                            if (folderPickerRef) folderPickerRef.moveFolder(folder);
                            isFolderPickerOpen = true;
                        },
                    },
                    {
                        label: "Delete",
                        danger: true,
                        action: () => handleDeleteFolder(folder.id!, new MouseEvent("click")),
                    },
                ];
            }
            const rect = longPressTargetEl?.getBoundingClientRect();
            const cx = rect ? rect.left + rect.width / 2 : 100;
            const cy = rect ? rect.top + rect.height / 2 : 100;
            appState.contextMenu = { x: cx, y: cy, items, show: true };
            if (navigator.vibrate) navigator.vibrate(50);
        }, 500) as unknown as number;
    }

    function cancelLongPress() {
        if (longPressTimer !== undefined) {
            clearTimeout(longPressTimer);
            longPressTimer = undefined;
        }
    }

    function startRenameFolder(folder: Folder) {
        renamingFolder = folder;
        renamingVideo = null;
        renamingText = folder.name;
    }

    function startRenameVideo(video: Video) {
        renamingVideo = video;
        renamingFolder = null;
        renamingText = video.title;
    }

    async function commitRename() {
        const newName = renamingText.trim();
        if (!newName) {
            cancelRename();
            return;
        }
        if (renamingFolder && renamingFolder.id !== undefined && newName !== renamingFolder.name) {
            const fId = renamingFolder.id;
            await import("./db").then((m) =>
                m.renameFolder(fId, newName),
            );
            await appState.refreshFolders();
        } else if (renamingVideo && newName !== renamingVideo.title) {
            const v = renamingVideo;
            await import("./db").then((m) =>
                m.renameVideo(v.id, newName),
            );
            await appState.refreshVideos();
        }
        cancelRename();
    }

    function cancelRename() {
        renamingFolder = null;
        renamingVideo = null;
        renamingText = "";
    }

    function openVideoEdit(video: Video) {
        appState.videoToEdit = { ...video };
        appState.isEditVideoModalOpen = true;
    }

    async function handleDeleteFolder(id: number, e: MouseEvent) {
        e.stopPropagation();
        const folder = appState.folders.find((f) => f.id === id);
        if (!folder) return;

        const snapshotFolders = appState.folders;
        const snapshotPath = appState.selectionPath;
        const { deleteFolder, restoreFolder, saveFolder } = await import("./db");

        await deleteFolder(id);
        await appState.refreshFolders();
        await appState.refreshVideos();
        const idx = appState.selectionPath.indexOf(id);
        if (idx !== -1) {
            appState.openFolder(appState.selectionPath.slice(0, idx));
        }

        appState.showUndo(`Deleted folder "${folder.name}"`, async () => {
            await restoreFolder(id);
            appState.folders = snapshotFolders;
            appState.openFolder(snapshotPath);
            await appState.refreshFolders();
        });
    }

    async function handleDeleteVideo(id: string, e: MouseEvent) {
        e.stopPropagation();
        const video = appState.videos.find((v) => v.id === id);
        if (!video) return;

        const snapshot = appState.videos;
        const { deleteVideo, restoreVideo } = await import("./db");

        await deleteVideo(id);
        await appState.refreshVideos();

        appState.showUndo(`Deleted video "${video.title}"`, async () => {
            await restoreVideo(id);
            appState.videos = snapshot;
            await appState.refreshVideos();
        });
    }

    function focusOnMount(node: HTMLInputElement) {
        queueMicrotask(() => {
            node.focus();
            node.select();
        });
    }

    function handleCreateFolder() {
        creatingFolder = true;
        newFolderName = "";
    }

    async function commitCreateFolder() {
        const name = newFolderName.trim();
        creatingFolder = false;
        newFolderName = "";
        if (!name) return;
        const { saveFolder } = await import("./db");
        const parentId = currentFolderId;
        const maxOrder = appState.folders
            .filter((f) => f.parent_id === parentId)
            .reduce((max, f) => Math.max(max, f.sort_order), -1);
        await saveFolder({
            name,
            created_at: Date.now(),
            parent_id: parentId,
            sort_order: maxOrder + 1,
        });
        await appState.refreshFolders();
    }

    function goBackBreadcrumb() {
        if (appState.selectionPath.length > 0) {
            appState.openFolder(appState.selectionPath.slice(0, -1));
        }
    }

    function jumpToBreadcrumb(i: number) {
        appState.openFolder(appState.selectionPath.slice(0, i + 1));
    }
</script>

<div class="h-full flex flex-col bg-black overflow-hidden" class:bg-[color:var(--surface)]={compact}>
    {#if isAndroid && !compact}
        <header
            class="shrink-0 border-b border-[color:var(--border)] bg-[color:var(--surface)] px-3 py-2 flex items-center gap-2 min-h-[48px]"
        >
            {#if appState.selectionPath.length > 0}
                <button
                    onclick={goBackBreadcrumb}
                    class="min-w-[48px] min-h-[48px] flex items-center justify-center hover:bg-zinc-800 rounded-full -ml-2"
                    aria-label="Back"
                >
                    <svg class="size-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
                    </svg>
                </button>
            {/if}
            <div class="flex-1 min-w-0 flex items-center gap-1 overflow-x-auto scrollbar-none">
                <button
                    onclick={() => appState.openFolder([])}
                    class="text-sm font-bold text-zinc-400 uppercase tracking-wider shrink-0"
                >Library</button>
                {#each breadcrumbFolders as folder, i}
                    <svg class="size-3 text-zinc-600 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                    </svg>
                    <button
                        onclick={() => jumpToBreadcrumb(i)}
                        class="text-sm text-zinc-300 truncate shrink-0 hover:text-white"
                    >{folder.name}</button>
                {/each}
            </div>
        </header>
    {/if}

    <div class="flex-1 min-h-0 overflow-y-auto">
        {#if items.length === 0 && !creatingFolder}
            <div class="flex flex-col items-center justify-center h-full opacity-40 text-zinc-500 text-sm gap-2">
                <svg class="size-12" fill="currentColor" viewBox="0 0 24 24">
                    <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
                </svg>
                <span>Empty</span>
            </div>
        {:else}
            <div class="flex flex-col">
                {#each items as item (item.kind === "folder" ? `f-${item.folder.id}` : `v-${item.video.id}`)}
                    {#if item.kind === "folder"}
                        <div
                            role="button"
                            tabindex="0"
                            onclick={() => appState.openFolder([...appState.selectionPath, item.folder.id!])}
                            onkeydown={(e) => e.key === "Enter" && appState.openFolder([...appState.selectionPath, item.folder.id!])}
                            ontouchstart={(e) => startLongPress(item.folder, e)}
                            ontouchend={cancelLongPress}
                            ontouchmove={cancelLongPress}
                            oncontextmenu={(e) => {
                                e.preventDefault();
                                e.stopPropagation();
                                appState.contextMenu = {
                                    x: e.clientX,
                                    y: e.clientY,
                                    show: true,
                                    items: [
                                        { label: "Open", action: () => appState.openFolder([...appState.selectionPath, item.folder.id!]) },
                                        { label: "Rename", action: () => startRenameFolder(item.folder) },
                                        {
                                            label: "Move to…",
                                            action: () => {
                                                folderPickerTitle = `Move folder "${item.folder.name}"`;
                                                folderPickerExcludedFolder = item.folder.id;
                                                if (folderPickerRef) folderPickerRef.moveFolder(item.folder);
                                                isFolderPickerOpen = true;
                                            },
                                        },
                                        { label: "Delete", danger: true, action: () => handleDeleteFolder(item.folder.id!, new MouseEvent("click")) },
                                    ],
                                };
                            }}
                            class="min-h-[56px] px-3 flex items-center gap-3 cursor-pointer select-none shrink-0 border-b border-[color:var(--border)] active:bg-[color:var(--surface-hi)]"
                        >
                            {#if renamingFolder?.id === item.folder.id}
                                <input
                                    bind:value={renamingText}
                                    class="flex-1 bg-zinc-900 border border-blue-500 rounded px-2 py-1 text-base text-white outline-none"
                                    use:focusOnMount
                                    onkeydown={(e) => {
                                        if (e.key === "Enter") commitRename();
                                        else if (e.key === "Escape") cancelRename();
                                    }}
                                    onblur={commitRename}
                                    onclick={(e) => e.stopPropagation()}
                                />
                            {:else}
                                <svg class="size-6 opacity-70 shrink-0" fill="currentColor" viewBox="0 0 24 24">
                                    <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
                                </svg>
                                <span class="flex-1 truncate text-base">{item.folder.name}</span>
                                <svg class="size-5 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                                </svg>
                            {/if}
                        </div>
                    {:else}
                        <div
                            role="button"
                            tabindex="0"
                            onclick={() => appState.openVideo(item.video)}
                            onkeydown={(e) => e.key === "Enter" && appState.openVideo(item.video)}
                            ontouchstart={(e) => startLongPress(item.video, e)}
                            ontouchend={cancelLongPress}
                            ontouchmove={cancelLongPress}
                            oncontextmenu={(e) => {
                                e.preventDefault();
                                e.stopPropagation();
                                appState.contextMenu = {
                                    x: e.clientX,
                                    y: e.clientY,
                                    show: true,
                                    items: [
                                        { label: "Play", action: () => appState.openVideo(item.video) },
                                        { label: "Rename", action: () => startRenameVideo(item.video) },
                                        { label: "Edit Trim…", action: () => openVideoEdit(item.video) },
                                        {
                                            label: "Move to…",
                                            action: () => {
                                                folderPickerTitle = `Move "${item.video.title}"`;
                                                if (folderPickerRef) folderPickerRef.moveVideo(item.video);
                                                isFolderPickerOpen = true;
                                            },
                                        },
                                        { label: "Delete", danger: true, action: () => handleDeleteVideo(item.video.id!, new MouseEvent("click")) },
                                    ],
                                };
                            }}
                            class="min-h-[56px] px-3 flex items-center gap-3 cursor-pointer select-none shrink-0 border-b border-[color:var(--border)] active:bg-[color:var(--surface-hi)]"
                        >
                            {#if renamingVideo?.id === item.video.id}
                                <input
                                    bind:value={renamingText}
                                    class="flex-1 bg-zinc-900 border border-blue-500 rounded px-2 py-1 text-base text-white outline-none"
                                    use:focusOnMount
                                    onkeydown={(e) => {
                                        if (e.key === "Enter") commitRename();
                                        else if (e.key === "Escape") cancelRename();
                                    }}
                                    onblur={commitRename}
                                    onclick={(e) => e.stopPropagation()}
                                />
                            {:else}
                                <Thumbnail
                                    videoId={item.video.id}
                                    alt=""
                                    className="w-16 h-10 object-cover rounded bg-zinc-800 shrink-0"
                                />
                                <div class="flex-1 min-w-0">
                                    <div class="truncate text-base">{item.video.title}</div>
                                    <div class="text-xs text-zinc-500">
                                        {#if item.video.last_position > 0}
                                            ▶ {formatTime(item.video.last_position)}
                                        {:else if item.video.duration > 0}
                                            {formatTime(item.video.duration)}
                                        {:else}
                                            New
                                        {/if}
                                    </div>
                                </div>
                            {/if}
                        </div>
                    {/if}
                {/each}
            </div>
        {/if}
    </div>

    {#if !compact}
        <footer
            class="shrink-0 border-t border-[color:var(--border)] bg-[color:var(--surface)] px-3 py-2 flex items-center gap-2 min-h-[56px]"
        >
            {#if creatingFolder}
                <input
                    bind:value={newFolderName}
                    placeholder="New folder name"
                    class="flex-1 min-w-0 bg-zinc-950 border border-blue-500 rounded px-3 py-2 text-base text-white outline-none"
                    use:focusOnMount
                    onkeydown={(e) => {
                        if (e.key === "Enter") commitCreateFolder();
                        else if (e.key === "Escape") {
                            creatingFolder = false;
                            newFolderName = "";
                        }
                    }}
                    onblur={commitCreateFolder}
                />
            {:else}
                <button
                    onclick={handleCreateFolder}
                    class="min-w-[48px] min-h-[48px] flex items-center justify-center hover:bg-zinc-800 rounded-lg"
                    title="New Folder"
                    aria-label="New Folder"
                >
                    <svg class="size-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 13h6m-3-3v6m-9 1V7a2 2 0 012-2h6l2 2h6a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2z" />
                    </svg>
                </button>
                <button
                    onclick={() => {
                        appState.addVideoFolderId = currentFolderId;
                        appState.isAddVideoModalOpen = true;
                    }}
                    class="min-w-[48px] min-h-[48px] flex items-center justify-center hover:bg-zinc-800 rounded-lg"
                    title="Add Video"
                    aria-label="Add Video"
                >
                    <svg class="size-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
                    </svg>
                </button>
                <button
                    onclick={() => appState.triggerSync()}
                    disabled={appState.syncStatus === "syncing" || !appState.settings.githubTokenPresent || !appState.settings.githubRepo}
                    class="min-w-[48px] min-h-[48px] flex items-center justify-center hover:bg-zinc-800 disabled:opacity-50 rounded-lg"
                    title="Sync"
                    aria-label="Sync"
                >
                    <svg class="size-5 {appState.syncStatus === 'syncing' ? 'animate-spin' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                    </svg>
                </button>
                <button
                    onclick={() => (appState.isSettingsModalOpen = true)}
                    class="min-w-[48px] min-h-[48px] flex items-center justify-center hover:bg-zinc-800 rounded-lg"
                    title="Settings"
                    aria-label="Settings"
                >
                    <svg class="size-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                    </svg>
                </button>
            {/if}
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