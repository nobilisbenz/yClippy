<script lang="ts">
    import { onMount, onDestroy, untrack } from "svelte";
    import { saveClip, type Video } from "./db";
    import { appState } from "./state.svelte";
    import { YouTubeController, formatClock } from "./youtube.svelte";
    import Track from "./Track.svelte";

    /// Desktop shell. Playback lives in YouTubeController; this file is layout,
    /// the transport, and keys.
    let { video, seekToTime, seekTo = $bindable(), onSeekConsumed } = $props<{
        video: Video;
        seekToTime?: number;
        seekTo?: (t: number) => void;
        onSeekConsumed?: () => void;
    }>();

    const initialSeek: number | undefined = untrack(() => seekToTime);
    const yt = new YouTubeController();

    let naming = $state(false);
    let clipTitle = $state("");
    let nameField = $state<HTMLInputElement | null>(null);
    let rate = $state(1);

    const RATES = [0.75, 1, 1.25, 1.5, 2];

    onMount(async () => {
        await yt.mount(video, initialSeek);
        onSeekConsumed?.();
        window.addEventListener("keydown", handleKeyDown);
    });

    onDestroy(async () => {
        window.removeEventListener("keydown", handleKeyDown);
        // Pass the current row rather than the prop captured at mount, so a
        // title or folder change that arrived from a sync is not reverted.
        const latest = appState.videos.find((v) => v.id === video.id) ?? video;
        await yt.destroy(latest);
        await appState.refreshVideos();
    });

    // A play request for the video already open: no remount, so seek directly.
    $effect(() => {
        if (seekToTime !== undefined && yt.player) {
            yt.seek(seekToTime);
            onSeekConsumed?.();
        }
    });

    $effect(() => {
        seekTo = (t: number) => yt.seek(t);
    });

    function beginNaming() {
        if (!yt.pendingIsComplete) yt.markOut();
        clipTitle = "";
        naming = true;
        queueMicrotask(() => nameField?.focus());
    }

    async function commitClip() {
        if (yt.pendingStart === null) return;
        const end = yt.pendingEnd ?? yt.currentTime;
        try {
            await saveClip({
                video_id: video.id,
                start_time: Math.floor(yt.pendingStart),
                end_time: Math.floor(end),
                title: clipTitle.trim() || `Clip at ${formatClock(yt.pendingStart)}`,
                created_at: Date.now(),
                sort_order: 0,
            });
            await appState.refreshActiveClips();
            appState.showToast("Clip saved", "success");
        } catch (e) {
            appState.showToast(`Could not save the clip: ${String(e)}`, "error");
        }
        naming = false;
        yt.clearPending();
    }

    function discardClip() {
        naming = false;
        yt.clearPending();
    }

    function cycleRate() {
        rate = RATES[(RATES.indexOf(rate) + 1) % RATES.length];
        yt.setRate(rate);
    }

    function handleKeyDown(e: KeyboardEvent) {
        const target = e.target as HTMLElement | null;
        if (
            target instanceof HTMLInputElement ||
            target instanceof HTMLTextAreaElement ||
            target?.isContentEditable
        ) {
            if (e.key === "Escape") discardClip();
            return;
        }
        if (e.ctrlKey || e.metaKey || e.altKey) return;

        switch (e.key) {
            case " ":
                e.preventDefault();
                yt.toggle();
                break;
            case "j":
                yt.skip(e.shiftKey ? -60 : -10);
                break;
            case "l":
                yt.skip(e.shiftKey ? 60 : 10);
                break;
            case "ArrowLeft":
                yt.skip(-5);
                break;
            case "ArrowRight":
                yt.skip(5);
                break;
            case ",":
                yt.skip(-0.5);
                break;
            case ".":
                yt.skip(0.5);
                break;
            case "[":
                yt.markIn();
                break;
            case "]":
                yt.markOut();
                break;
            case "Enter":
                if (yt.hasPending) {
                    e.preventDefault();
                    beginNaming();
                }
                break;
            case "Escape":
                if (yt.hasPending) discardClip();
                else appState.goBack();
                break;
            default:
                // 1–9 jump to clip N.
                if (/^[1-9]$/.test(e.key)) {
                    const clip = appState.activeClips[Number(e.key) - 1];
                    if (clip) yt.seek(clip.start_time);
                }
        }
    }
</script>

<div class="flex flex-col h-full min-h-0 bg-black">
    <!-- Stage -->
    <div class="flex-1 min-h-0 relative bg-black">
        <div class="absolute inset-0 flex items-center justify-center">
            <div class="w-full h-full max-h-full" style="aspect-ratio: 16/9; max-width: 100%">
                <div id={yt.elementId} class="w-full h-full"></div>
            </div>
        </div>

        {#if yt.isPaused && yt.isReady}
            <button
                class="absolute top-3 left-3 z-20 px-3 py-1.5 rounded-md bg-black/70 border border-[color:var(--border-hi)] text-xs text-[color:var(--text-dim)] hover:text-[color:var(--text)] transition"
                onclick={() => appState.goBack()}
            >
                ← Library
            </button>
        {/if}
    </div>

    <!-- Transport -->
    <div
        class="shrink-0 border-t border-[color:var(--border)] bg-[color:var(--surface)] px-4 pt-3 pb-2"
    >
        <Track
            clips={appState.activeClips}
            duration={yt.duration}
            currentTime={yt.currentTime}
            watched={video.last_position}
            pendingStart={yt.pendingStart}
            pendingEnd={yt.pendingEnd}
            onSeek={(t) => yt.seek(t)}
            onClipTap={(clip) => yt.seek(clip.start_time)}
            onPendingChange={(start, end) => {
                yt.pendingStart = start;
                yt.pendingEnd = end;
            }}
        />

        {#if naming}
            <!-- Named in place, on the track, rather than in a modal that hides
                 the range you are naming. -->
            <form
                class="flex items-center gap-2 mt-2"
                onsubmit={(e) => {
                    e.preventDefault();
                    commitClip();
                }}
            >
                <span class="text-[11px] t-num text-[color:var(--accent)] shrink-0">
                    {formatClock(yt.pendingStart ?? 0)} – {formatClock(
                        yt.pendingEnd ?? yt.currentTime,
                    )}
                </span>
                <input
                    bind:this={nameField}
                    bind:value={clipTitle}
                    placeholder="Name this clip…"
                    class="flex-1 min-w-0 bg-[color:var(--bg)] border border-[color:var(--border-hi)] rounded px-2 py-1 text-sm text-[color:var(--text)] focus:outline-none focus:border-[color:var(--accent)]"
                />
                <button
                    type="submit"
                    class="px-3 py-1 text-xs rounded bg-[color:var(--accent)] text-white"
                >
                    Save
                </button>
                <button
                    type="button"
                    onclick={discardClip}
                    class="px-2 py-1 text-xs text-[color:var(--text-faint)] hover:text-[color:var(--text)]"
                >
                    Cancel
                </button>
            </form>
        {:else}
            <div class="flex items-center gap-1 mt-2">
                <button
                    class="px-2 py-1 rounded text-[color:var(--text-dim)] hover:text-[color:var(--text)] hover:bg-[color:var(--surface-hi)] transition"
                    onclick={() => yt.skip(-10)}
                    title="Back 10s (j)"
                    aria-label="Back 10 seconds">⏮</button
                >
                <button
                    class="px-3 py-1 rounded text-[color:var(--text)] hover:bg-[color:var(--surface-hi)] transition"
                    onclick={() => yt.toggle()}
                    title="Play / pause (space)"
                    aria-label={yt.isPaused ? "Play" : "Pause"}
                >
                    {yt.isPaused ? "▶" : "❚❚"}
                </button>
                <button
                    class="px-2 py-1 rounded text-[color:var(--text-dim)] hover:text-[color:var(--text)] hover:bg-[color:var(--surface-hi)] transition"
                    onclick={() => yt.skip(10)}
                    title="Forward 10s (l)"
                    aria-label="Forward 10 seconds">⏭</button
                >

                <div class="w-px h-4 bg-[color:var(--border)] mx-2"></div>

                <button
                    class="px-2 py-1 text-xs rounded border transition {yt.pendingStart !== null
                        ? 'border-[color:var(--accent)] text-[color:var(--accent)]'
                        : 'border-[color:var(--border-hi)] text-[color:var(--text-dim)] hover:text-[color:var(--text)]'}"
                    onclick={() => yt.markIn()}
                    title="Mark in ([)">[ in</button
                >
                <button
                    class="px-2 py-1 text-xs rounded border border-[color:var(--border-hi)] text-[color:var(--text-dim)] hover:text-[color:var(--text)] transition"
                    onclick={() => yt.markOut()}
                    title="Mark out (])">out ]</button
                >
                {#if yt.hasPending}
                    <button
                        class="px-2 py-1 text-xs rounded bg-[color:var(--accent)] text-white"
                        onclick={beginNaming}
                        title="Name and save (Enter)">Save clip</button
                    >
                    <button
                        class="px-2 py-1 text-xs text-[color:var(--text-faint)] hover:text-[color:var(--text)]"
                        onclick={discardClip}
                        title="Discard (Esc)">✕</button
                    >
                {/if}

                <div class="flex-1"></div>

                <button
                    class="px-2 py-1 text-xs t-num rounded border border-[color:var(--border-hi)] text-[color:var(--text-dim)] hover:text-[color:var(--text)] transition"
                    onclick={cycleRate}
                    title="Playback speed">{rate}×</button
                >
            </div>
        {/if}
    </div>
</div>
