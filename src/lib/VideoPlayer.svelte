<script lang="ts">
    import { onMount, onDestroy, untrack } from "svelte";
    import { openUrl } from "@tauri-apps/plugin-opener";
    import { saveClip, type Video } from "./db";
    import { appState } from "./state.svelte";
    import { YouTubeController, formatClock } from "./youtube.svelte";
    import CodecNotice from "./CodecNotice.svelte";
    import Icon from "./Icon.svelte";
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
    let stageHover = $state(false);

    const RATES = [0.5, 0.75, 1, 1.25, 1.5, 2];

    onMount(async () => {
        window.addEventListener("keydown", handleKeyDown);
        await yt.mount(video, initialSeek);
        onSeekConsumed?.();
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
                sort_order: appState.activeClips.length,
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

    async function openOnYouTube() {
        try {
            await openUrl(yt.watchUrl);
        } catch (e) {
            appState.showToast(`Could not open the browser: ${String(e)}`, "error");
        }
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
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
        class="flex-1 min-h-0 relative bg-black"
        onmouseenter={() => (stageHover = true)}
        onmouseleave={() => (stageHover = false)}
    >
        <div class="absolute inset-0 flex items-center justify-center p-2">
            <div
                class="relative w-full h-full max-h-full max-w-full"
                style="aspect-ratio: 16/9"
            >
                <div id={yt.elementId} class="w-full h-full"></div>

                {#if !yt.isReady && !yt.error}
                    <!-- The stage is black either way; say which black it is. -->
                    <div
                        class="absolute inset-0 grid place-items-center pointer-events-none"
                    >
                        <span
                            class="text-xs text-[color:var(--text-faint)] animate-pulse"
                        >
                            Loading player…
                        </span>
                    </div>
                {/if}

                {#if yt.error}
                    <div
                        class="absolute inset-0 grid place-items-center bg-[color:var(--bg)]/95 px-6"
                    >
                        <div class="flex flex-col items-center gap-3 text-center max-w-sm">
                            <span class="text-[color:var(--danger)]">
                                <Icon name="alert" size={22} />
                            </span>
                            <p class="text-sm text-[color:var(--text)]">{yt.error}</p>
                            <div class="flex items-center gap-2">
                                <button class="btn" onclick={() => yt.retry()}>
                                    <Icon name="sync" size={14} />
                                    Try again
                                </button>
                                <button class="btn btn-primary" onclick={openOnYouTube}>
                                    <Icon name="external" size={14} />
                                    Open on YouTube
                                </button>
                            </div>
                        </div>
                    </div>
                {/if}
            </div>
        </div>

        <!-- Leaving is a top-level move, so the way out is always on screen —
             it used to appear only while paused. -->
        <div
            class="absolute top-0 inset-x-0 z-20 p-2 flex items-center gap-2 transition-opacity duration-200"
            class:opacity-0={!stageHover && !yt.isPaused && !yt.error}
        >
            <button
                class="btn btn-ghost bg-black/70 backdrop-blur-sm"
                onclick={() => appState.goBack()}
                title="Back to the library (Esc)"
            >
                <Icon name="back" size={14} />
                Library
            </button>
            <div class="flex-1"></div>
            <button
                class="icon-btn bg-black/70 backdrop-blur-sm"
                onclick={openOnYouTube}
                title="Open on YouTube at the current time"
                aria-label="Open on YouTube"
            >
                <Icon name="external" size={15} />
            </button>
        </div>

        <div class="absolute left-3 bottom-3 z-20">
            <CodecNotice />
        </div>
    </div>

    <!-- Transport -->
    <div
        class="shrink-0 border-t border-[color:var(--border)] bg-[color:var(--surface)] px-3 pt-2.5 pb-2"
    >
        <Track
            clips={appState.activeClips}
            duration={yt.duration}
            currentTime={yt.currentTime}
            watched={video.last_position}
            loaded={yt.loaded}
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
                <span
                    class="chip t-num shrink-0"
                    style="background: var(--accent-soft); color: var(--accent)"
                >
                    {formatClock(yt.pendingStart ?? 0)} – {formatClock(
                        yt.pendingEnd ?? yt.currentTime,
                    )}
                </span>
                <input
                    bind:this={nameField}
                    bind:value={clipTitle}
                    placeholder="Name this clip…"
                    class="field flex-1 min-w-0 py-1.5 text-sm"
                />
                <button type="submit" class="btn btn-primary shrink-0">Save clip</button>
                <button
                    type="button"
                    onclick={discardClip}
                    class="icon-btn shrink-0"
                    aria-label="Discard clip"
                    title="Discard (Esc)"
                >
                    <Icon name="close" size={15} />
                </button>
            </form>
        {:else}
            <div class="flex items-center gap-1 mt-2">
                <button
                    class="icon-btn"
                    style="--size: 32px"
                    onclick={() => yt.skip(-10)}
                    title="Back 10 seconds (j)"
                    aria-label="Back 10 seconds"
                >
                    <Icon name="back10" size={18} />
                </button>
                <button
                    class="icon-btn text-[color:var(--text)]"
                    style="--size: 36px"
                    onclick={() => yt.toggle()}
                    title={yt.isPaused ? "Play (space)" : "Pause (space)"}
                    aria-label={yt.isPaused ? "Play" : "Pause"}
                >
                    <Icon name={yt.isPaused ? "play" : "pause"} size={20} />
                </button>
                <button
                    class="icon-btn"
                    style="--size: 32px"
                    onclick={() => yt.skip(10)}
                    title="Forward 10 seconds (l)"
                    aria-label="Forward 10 seconds"
                >
                    <Icon name="forward10" size={18} />
                </button>

                <div class="w-px h-5 bg-[color:var(--border)] mx-2"></div>

                <button
                    class="btn"
                    style={yt.pendingStart !== null
                        ? "border-color: var(--accent); color: var(--accent)"
                        : ""}
                    onclick={() => yt.markIn()}
                    title="Mark the clip's start at the playhead ([)"
                >
                    <Icon name="markIn" size={14} />
                    In
                </button>
                <button
                    class="btn"
                    style={yt.pendingEnd !== null
                        ? "border-color: var(--accent); color: var(--accent)"
                        : ""}
                    onclick={() => yt.markOut()}
                    title="Mark the clip's end at the playhead (])"
                >
                    <Icon name="markOut" size={14} />
                    Out
                </button>
                {#if yt.hasPending}
                    <span class="chip t-num ml-1">
                        {formatClock(yt.pendingStart ?? 0)} – {formatClock(
                            yt.pendingEnd ?? yt.currentTime,
                        )}
                    </span>
                    <button class="btn btn-primary" onclick={beginNaming} title="Name and save (Enter)">
                        <Icon name="scissors" size={14} />
                        Save clip
                    </button>
                    <button
                        class="icon-btn"
                        onclick={discardClip}
                        title="Discard (Esc)"
                        aria-label="Discard clip"
                    >
                        <Icon name="close" size={15} />
                    </button>
                {/if}

                <div class="flex-1"></div>

                <!-- The keys are the point of the app; showing them is cheaper
                     than a help screen nobody opens. -->
                <div
                    class="hidden xl:flex items-center gap-1.5 text-[11px] text-[color:var(--text-faint)] mr-2"
                >
                    <kbd class="kbd">space</kbd>
                    <span>play</span>
                    <kbd class="kbd">j</kbd><kbd class="kbd">l</kbd>
                    <span>±10s</span>
                    <kbd class="kbd">[</kbd><kbd class="kbd">]</kbd>
                    <span>mark</span>
                </div>

                <button class="btn t-num" onclick={cycleRate} title="Playback speed">
                    {rate}×
                </button>
            </div>
        {/if}
    </div>
</div>
