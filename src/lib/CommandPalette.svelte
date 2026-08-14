<script lang="ts">
    import { appState } from "./state.svelte";
    import { onMount, tick } from "svelte";
    import type { Video, Clip, Folder } from "./db";
    import Icon from "./Icon.svelte";
    import { formatClock } from "./youtube.svelte";

    let { onClose }: { onClose: () => void } = $props();

    let query = $state("");
    let selectedIndex = $state(0);
    let inputEl: HTMLInputElement | undefined = $state();

    type Item =
        | { kind: "video"; video: Video; clipCount: number }
        | { kind: "folder"; folder: Folder; childCount: number }
        | { kind: "clip"; clip: Clip; video: Video }
        | { kind: "action"; id: string; label: string; run: () => void };

    let items = $derived.by<Item[]>(() => {
        const q = query.trim().toLowerCase();
        const results: Item[] = [];

        if (q === "" || "library".startsWith(q)) {
            results.push({
                kind: "action",
                id: "go-library",
                label: "Go to library",
                run: () => {
                    appState.openFolder([]);
                },
            });
        }
        if (q === "" || "add video".startsWith(q) || "new video".startsWith(q)) {
            results.push({
                kind: "action",
                id: "add-video",
                label: "Add video…",
                run: () => {
                    appState.isAddVideoModalOpen = true;
                },
            });
        }
        if (q === "" || "settings".startsWith(q)) {
            results.push({
                kind: "action",
                id: "open-settings",
                label: "Open settings",
                run: () => {
                    appState.isSettingsModalOpen = true;
                },
            });
        }
        if (q === "" || "sync".startsWith(q)) {
            results.push({
                kind: "action",
                id: "sync",
                label: "Sync to GitHub",
                run: () => appState.triggerSync(),
            });
        }

        for (const folder of appState.folders) {
            if (!q || folder.name.toLowerCase().includes(q)) {
                const childCount =
                    appState.videos.filter((v) => v.folder_id === folder.id).length +
                    appState.folders.filter((f) => f.parent_id === folder.id).length;
                results.push({ kind: "folder", folder, childCount });
            }
        }

        for (const video of appState.videos) {
            if (!q || video.title.toLowerCase().includes(q)) {
                const clipCount = appState.activeClips.length > 0 &&
                    appState.activeVideo?.id === video.id
                    ? appState.activeClips.length
                    : 0;
                results.push({ kind: "video", video, clipCount });
            }
        }

        if (appState.activeVideo) {
            for (const clip of appState.activeClips) {
                if (!q || clip.title.toLowerCase().includes(q)) {
                    results.push({ kind: "clip", clip, video: appState.activeVideo });
                }
            }
        }

        return results.slice(0, 50);
    });

    $effect(() => {
        items;
        selectedIndex = 0;
    });

    onMount(async () => {
        await tick();
        inputEl?.focus();
    });

    function runItem(item: Item) {
        if (item.kind === "video") {
            appState.consumeSeek();
            appState.openVideo(item.video);
        } else if (item.kind === "folder") {
            appState.openFolder([item.folder.id!]);
        } else if (item.kind === "clip") {
            // Set before opening: the player captures it at init.
            appState.seekToTime = item.clip.start_time;
            appState.openVideo(item.video);
        } else if (item.kind === "action") {
            item.run();
        }
        onClose();
    }

    function handleKeyDown(e: KeyboardEvent) {
        if (e.key === "Escape") {
            e.preventDefault();
            onClose();
        } else if (e.key === "ArrowDown") {
            e.preventDefault();
            selectedIndex = Math.min(items.length - 1, selectedIndex + 1);
        } else if (e.key === "ArrowUp") {
            e.preventDefault();
            selectedIndex = Math.max(0, selectedIndex - 1);
        } else if (e.key === "Enter") {
            e.preventDefault();
            const item = items[selectedIndex];
            if (item) runItem(item);
        }
    }

    function labelFor(item: Item): string {
        if (item.kind === "video") return item.video.title;
        if (item.kind === "folder") return item.folder.name;
        if (item.kind === "clip") return item.clip.title;
        return item.label;
    }

    function hintFor(item: Item): string {
        if (item.kind === "video") {
            return item.video.last_position > 0
                ? `at ${formatClock(item.video.last_position)}`
                : "video";
        }
        if (item.kind === "folder") return `${item.childCount} item${item.childCount === 1 ? "" : "s"}`;
        if (item.kind === "clip") {
            return `${formatClock(item.clip.start_time)} – ${formatClock(item.clip.end_time)}`;
        }
        return "action";
    }
</script>

<div
    class="fixed inset-0 z-[300] bg-black/60 backdrop-blur-sm flex items-start justify-center pt-[12vh] px-4"
    onclick={onClose}
    role="presentation"
>
    <div
        class="w-full max-w-xl bg-[color:var(--surface)] border border-[color:var(--border)] rounded-xl shadow-2xl overflow-hidden"
        onclick={(e) => e.stopPropagation()}
        onkeydown={handleKeyDown}
        role="dialog"
        aria-label="Command palette"
        aria-modal="true"
        tabindex="-1"
    >
        <div class="flex items-center gap-3 px-4 border-b border-[color:var(--border)]">
            <span class="text-[color:var(--text-faint)] shrink-0">
                <Icon name="search" size={16} />
            </span>
            <input
                bind:this={inputEl}
                bind:value={query}
                type="text"
                placeholder="Search videos, folders and clips, or run a command…"
                class="flex-1 min-w-0 bg-transparent text-[color:var(--text)] text-base py-4 outline-none placeholder:text-[color:var(--text-faint)]"
            />
        </div>

        <div class="max-h-[50vh] overflow-y-auto">
            {#if items.length === 0}
                <div class="px-4 py-8 text-center text-[color:var(--text-faint)] text-sm">
                    No matches
                </div>
            {:else}
                {#each items as item, i (i + "-" + labelFor(item))}
                    <button
                        type="button"
                        onclick={() => runItem(item)}
                        onmouseenter={() => (selectedIndex = i)}
                        class="w-full flex items-center justify-between gap-3 px-4 py-2 text-left text-sm transition-colors {i ===
                        selectedIndex
                            ? 'bg-[color:var(--accent)] text-white'
                            : 'text-[color:var(--text-dim)] hover:bg-[color:var(--surface-hi)]'}"
                    >
                        <div class="flex items-center gap-2.5 truncate">
                            <span class="opacity-80 shrink-0">
                                <Icon
                                    name={item.kind === "folder"
                                        ? "folder"
                                        : item.kind === "clip"
                                          ? "scissors"
                                          : item.kind === "video"
                                            ? "video"
                                            : "check"}
                                    size={15}
                                />
                            </span>
                            <span class="truncate">{labelFor(item)}</span>
                        </div>
                        <span
                            class="text-[11px] t-num shrink-0 {i === selectedIndex
                                ? 'text-white/80'
                                : 'text-[color:var(--text-faint)]'}"
                        >
                            {hintFor(item)}
                        </span>
                    </button>
                {/each}
            {/if}
        </div>

        <div
            class="px-4 py-2 border-t border-[color:var(--border)] text-[11px] text-[color:var(--text-faint)] flex justify-between items-center gap-2"
        >
            <span class="flex items-center gap-1.5">
                <kbd class="kbd">↑</kbd><kbd class="kbd">↓</kbd> move
                <kbd class="kbd">↵</kbd> open
            </span>
            <span class="flex items-center gap-1.5"><kbd class="kbd">Esc</kbd> close</span>
        </div>
    </div>
</div>