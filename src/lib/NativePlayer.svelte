<script lang="ts">
    import { onMount, onDestroy, untrack } from "svelte";
    import { openUrl } from "@tauri-apps/plugin-opener";
    import { saveClip, type Video } from "./db";
    import { appState } from "./state.svelte";
    import { YouTubeController, formatClock } from "./youtube.svelte";
    import ClipList from "./ClipList.svelte";
    import Icon from "./Icon.svelte";
    import Track from "./Track.svelte";

    /// Touch shell.
    ///
    /// The phone's scarce resource is reach, not space, so the track and the
    /// transport live in the bottom third permanently and the video floats
    /// above them. Marking a clip is one wide button at the bottom edge —
    /// the whole flow works with one thumb, which two-button in/out never did.
    let { video, seekToTime, seekTo = $bindable(), onSeekConsumed } = $props<{
        video: Video;
        seekToTime?: number;
        seekTo?: (t: number) => void;
        onSeekConsumed?: () => void;
    }>();

    const initialSeek: number | undefined = untrack(() => seekToTime);
    const yt = new YouTubeController();

    let sheetOpen = $state(false);
    let naming = $state(false);
    let clipTitle = $state("");
    let nameField = $state<HTMLInputElement | null>(null);
    let landscape = $state(false);
    let rate = $state(1);

    const RATES = [1, 1.25, 1.5, 2, 0.75];

    onMount(async () => {
        const media = window.matchMedia("(orientation: landscape)");
        landscape = media.matches;
        media.addEventListener("change", onOrientation);
        await yt.mount(video, initialSeek);
        onSeekConsumed?.();
    });

    onDestroy(async () => {
        window.matchMedia("(orientation: landscape)").removeEventListener("change", onOrientation);
        const latest = appState.videos.find((v) => v.id === video.id) ?? video;
        await yt.destroy(latest);
        await appState.refreshVideos();
    });

    function onOrientation(e: MediaQueryListEvent) {
        landscape = e.matches;
        if (landscape) sheetOpen = false;
    }

    $effect(() => {
        if (seekToTime !== undefined && yt.player) {
            yt.seek(seekToTime);
            onSeekConsumed?.();
        }
    });

    $effect(() => {
        seekTo = (t: number) => yt.seek(t);
    });

    /// Back closes what is on top before it leaves the video, so the gesture
    /// always undoes the most recent thing rather than skipping a level.
    function handleBack() {
        if (naming) {
            naming = false;
            yt.clearPending();
        } else if (sheetOpen) {
            sheetOpen = false;
        } else {
            appState.goBack();
        }
    }

    function onMarkTap() {
        if (yt.pendingStart === null) {
            yt.markIn();
            navigator.vibrate?.(20);
        } else {
            yt.markOut();
            clipTitle = "";
            naming = true;
            queueMicrotask(() => nameField?.focus());
        }
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

    function cycleRate() {
        rate = RATES[(RATES.indexOf(rate) + 1) % RATES.length];
        yt.setRate(rate);
    }

    /// ReVanced if this device has it, the browser otherwise — and either way
    /// carrying the position, so a handoff at 12:30 does not restart at 0:00.
    async function openExternally() {
        const native = (window as any).yClippyNative;
        const at = Math.floor(yt.currentTime);
        if (native?.openInRevanced) {
            native.openInRevanced(video.id, at);
            return;
        }
        try {
            await openUrl(yt.watchUrl);
        } catch (e) {
            appState.showToast(`Could not open the video: ${String(e)}`, "error");
        }
    }
</script>

<div class="flex flex-col h-full min-h-0 w-full bg-black relative">
    {#if !landscape}
        <header
            class="shrink-0 flex items-center gap-1 h-14 px-1 border-b border-[color:var(--border)] bg-[color:var(--surface)]"
        >
            <button
                onclick={handleBack}
                class="icon-btn icon-btn-touch"
                aria-label="Back to the library"
            >
                <Icon name="back" size={22} />
            </button>
            <span class="flex-1 min-w-0 truncate text-[15px] text-[color:var(--text)]">
                {video.title}
            </span>
            <button class="icon-btn icon-btn-touch t-num text-xs" onclick={cycleRate} aria-label="Playback speed">
                {rate}×
            </button>
            <button
                onclick={openExternally}
                class="icon-btn icon-btn-touch"
                aria-label="Open in another player"
            >
                <Icon name="external" size={20} />
            </button>
        </header>
    {/if}

    <!-- Sticky player. -->
    <div class="shrink-0 bg-black relative" style={landscape ? "flex: 1 1 auto" : "aspect-ratio: 16/9"}>
        <div id={yt.elementId} class="w-full h-full"></div>

        {#if !yt.isReady && !yt.error}
            <div class="absolute inset-0 grid place-items-center pointer-events-none">
                <span class="text-xs text-[color:var(--text-faint)] animate-pulse">Loading player…</span>
            </div>
        {/if}

        {#if yt.error}
            <div class="absolute inset-0 grid place-items-center bg-[color:var(--bg)]/95 px-6">
                <div class="flex flex-col items-center gap-3 text-center">
                    <span class="text-[color:var(--danger)]"><Icon name="alert" size={22} /></span>
                    <p class="text-sm">{yt.error}</p>
                    <div class="flex gap-2">
                        <button class="btn" style="height: 44px" onclick={() => yt.retry()}>
                            Try again
                        </button>
                        <button class="btn btn-primary" style="height: 44px" onclick={openExternally}>
                            Open externally
                        </button>
                    </div>
                </div>
            </div>
        {/if}
    </div>

    {#if !landscape}
        <!-- Transport, in the bottom third where thumbs are. -->
        <div class="flex-1 min-h-0 flex flex-col justify-end px-3 pt-3">
            <Track
                clips={appState.activeClips}
                duration={yt.duration}
                currentTime={yt.currentTime}
                watched={video.last_position}
                loaded={yt.loaded}
                pendingStart={yt.pendingStart}
                pendingEnd={yt.pendingEnd}
                height={56}
                onSeek={(t) => yt.seek(t)}
                onClipTap={(clip) => yt.seek(clip.start_time)}
                onPendingChange={(start, end) => {
                    yt.pendingStart = start;
                    yt.pendingEnd = end;
                }}
            />

            <div class="flex items-center justify-center gap-6 py-3">
                <button
                    class="icon-btn icon-btn-touch"
                    style="--size: 56px"
                    onclick={() => yt.skip(-10)}
                    aria-label="Back 10 seconds"
                >
                    <Icon name="back10" size={26} />
                </button>
                <button
                    class="icon-btn icon-btn-touch text-[color:var(--text)]"
                    style="--size: 64px; background: var(--surface-hi); border-radius: 999px"
                    onclick={() => yt.toggle()}
                    aria-label={yt.isPaused ? "Play" : "Pause"}
                >
                    <Icon name={yt.isPaused ? "play" : "pause"} size={28} />
                </button>
                <button
                    class="icon-btn icon-btn-touch"
                    style="--size: 56px"
                    onclick={() => yt.skip(10)}
                    aria-label="Forward 10 seconds"
                >
                    <Icon name="forward10" size={26} />
                </button>
            </div>
        </div>

        <!-- The clip flow: one wide target at the thumb's resting place. -->
        {#if naming}
            <form
                class="shrink-0 flex items-center gap-2 p-2 bg-[color:var(--surface)] border-t border-[color:var(--border-hi)]"
                onsubmit={(e) => {
                    e.preventDefault();
                    commitClip();
                }}
            >
                <span class="chip t-num shrink-0" style="background: var(--accent-soft); color: var(--accent)">
                    {formatClock(yt.pendingStart ?? 0)} – {formatClock(yt.pendingEnd ?? yt.currentTime)}
                </span>
                <input
                    bind:this={nameField}
                    bind:value={clipTitle}
                    placeholder="Name this clip…"
                    class="field flex-1 min-w-0"
                    style="height: 48px"
                />
                <button type="submit" class="btn btn-primary shrink-0" style="height: 48px">Save</button>
                <button
                    type="button"
                    class="icon-btn shrink-0"
                    style="--size: 48px"
                    aria-label="Discard clip"
                    onclick={() => {
                        naming = false;
                        yt.clearPending();
                    }}><Icon name="close" size={20} /></button
                >
            </form>
        {:else}
            <button
                class="shrink-0 w-full h-14 flex items-center justify-center gap-2 text-sm tracking-[0.12em] uppercase transition-colors
                       {yt.pendingStart !== null
                    ? 'bg-[color:var(--accent)] text-white'
                    : 'bg-[color:var(--surface-hi)] text-[color:var(--text-dim)]'}"
                onclick={onMarkTap}
            >
                <Icon name={yt.pendingStart === null ? "markIn" : "markOut"} size={18} />
                {yt.pendingStart === null ? "Mark in" : "Mark out"}
                {#if yt.pendingStart !== null}
                    <span class="t-num opacity-80 normal-case tracking-normal">
                        from {formatClock(yt.pendingStart)}
                    </span>
                {/if}
            </button>
        {/if}

        <!-- Clips: a peek bar that never covers the transport, expanding to a
             sheet with a scrim that back and a tap outside both dismiss. -->
        <button
            class="shrink-0 h-14 px-4 flex items-center gap-3 border-t border-[color:var(--border)] bg-[color:var(--surface)] text-left"
            onclick={() => (sheetOpen = true)}
            aria-expanded={sheetOpen}
        >
            <Icon name="scissors" size={16} />
            <span class="text-sm text-[color:var(--text)]">Clips</span>
            <span class="chip">{appState.activeClips.length}</span>
            <span class="flex-1"></span>
            {#if appState.activeClips[0]}
                <span class="text-[11px] t-num text-[color:var(--text-faint)]">
                    first {formatClock(appState.activeClips[0].start_time)}
                </span>
            {/if}
            <Icon name="chevronUp" size={16} />
        </button>

        {#if sheetOpen}
            <button
                class="absolute inset-0 z-20 bg-black/60"
                aria-label="Close clips"
                onclick={() => (sheetOpen = false)}
            ></button>
            <div
                class="absolute inset-x-0 bottom-0 z-30 flex flex-col rounded-t-2xl border-t border-[color:var(--border-hi)] bg-[color:var(--surface)]"
                style="height: min(70dvh, 560px)"
            >
                <button
                    class="shrink-0 h-14 px-4 flex items-center gap-3"
                    onclick={() => (sheetOpen = false)}
                    aria-label="Collapse clips"
                >
                    <span class="block w-9 h-1 rounded-full bg-[color:var(--border-hi)]"></span>
                    <span class="text-sm text-[color:var(--text)]">Clips</span>
                    <span class="chip">{appState.activeClips.length}</span>
                    <span class="flex-1"></span>
                    <Icon name="chevronDown" size={18} />
                </button>
                <div class="flex-1 min-h-0">
                    <ClipList videoId={video.id} touch seekTo={(t) => {
                        yt.seek(t);
                        sheetOpen = false;
                    }} />
                </div>
            </div>
        {/if}
    {:else}
        <!-- Rotating means you stopped clipping and started watching. -->
        <button
            class="absolute top-2 left-2 z-30 icon-btn icon-btn-touch bg-black/60 text-white rounded-full"
            onclick={handleBack}
            aria-label="Back"
        >
            <Icon name="back" size={22} />
        </button>
    {/if}
</div>
