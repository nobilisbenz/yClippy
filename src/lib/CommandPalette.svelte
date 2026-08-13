<script lang="ts">
    import { appState } from "./state.svelte";
    import { onMount, tick } from "svelte";
    import type { Video, Clip, Folder } from "./db";

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

    function runItem(item: Item, withCtrlEnter: boolean) {
        if (item.kind === "video") {
            appState.openVideo(item.video);
        } else if (item.kind === "folder") {
            appState.openFolder([item.folder.id!]);
        } else if (item.kind === "clip") {
            appState.openVideo(item.video);
            appState.seekToTime = item.clip.start_time;
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
            if (item) {
                runItem(item, e.ctrlKey || e.metaKey);
            }
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
            return `${item.video.last_position > 0 ? `${Math.floor(item.video.last_position)}s · ` : ""}Video`;
        }
        if (item.kind === "folder") return `${item.childCount} item${item.childCount === 1 ? "" : "s"}`;
        if (item.kind === "clip") {
            return `${Math.floor(item.clip.start_time)}s–${Math.floor(item.clip.end_time)}s · Clip`;
        }
        return "Action";
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
        <input
            bind:this={inputEl}
            bind:value={query}
            type="text"
            placeholder="Search videos, folders, clips, or run a command…"
            class="w-full bg-transparent text-white text-lg px-4 py-4 outline-none border-b border-[color:var(--border)]"
        />

        <div class="max-h-[50vh] overflow-y-auto">
            {#if items.length === 0}
                <div class="px-4 py-8 text-center text-zinc-500 text-sm">
                    No matches
                </div>
            {:else}
                {#each items as item, i (i + "-" + labelFor(item))}
                    <button
                        type="button"
                        onclick={() => runItem(item, false)}
                        onmouseenter={() => (selectedIndex = i)}
                        class="w-full flex items-center justify-between gap-3 px-4 py-2 text-left text-sm transition-colors {i === selectedIndex
                            ? 'bg-[color:var(--accent)] text-white'
                            : 'text-zinc-200 hover:bg-[color:var(--surface-hi)]'}"
                    >
                        <div class="flex items-center gap-2 truncate">
                            {#if item.kind === "folder"}
                                <svg class="size-4 opacity-70" fill="currentColor" viewBox="0 0 24 24"><path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" /></svg>
                            {:else if item.kind === "clip"}
                                <svg class="size-4 opacity-70" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" /><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>
                            {:else if item.kind === "video"}
                                <svg class="size-4 opacity-70" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" /></svg>
                            {:else}
                                <svg class="size-4 opacity-70" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" /></svg>
                            {/if}
                            <span class="truncate">{labelFor(item)}</span>
                        </div>
                        <span class="text-xs {i === selectedIndex ? 'text-white/80' : 'text-zinc-500'}">
                            {hintFor(item)}
                        </span>
                    </button>
                {/each}
            {/if}
        </div>

        <div class="px-4 py-2 border-t border-[color:var(--border)] text-xs text-zinc-500 flex justify-between">
            <span>↑↓ navigate · Enter open · Ctrl+Enter play clip</span>
            <span>Esc close</span>
        </div>
    </div>
</div>