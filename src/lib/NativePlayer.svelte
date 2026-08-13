<script lang="ts">
    import { onMount, onDestroy, untrack } from "svelte";
    import { saveClip, type Video } from "./db";
    import { appState } from "./state.svelte";
    import { YouTubeController, formatClock } from "./youtube.svelte";
    import Track from "./Track.svelte";
    import ClipList from "./ClipList.svelte";

    /// Android shell.
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

    /// Peek shows the count without covering the transport; the old panel was
    /// absolutely positioned over it with no way out.
    type Detent = "closed" | "peek" | "half" | "full";
    let sheet = $state<Detent>("peek");
    let naming = $state(false);
    let clipTitle = $state("");
    let nameField = $state<HTMLInputElement | null>(null);
    let landscape = $state(false);

    const SHEET_HEIGHT: Record<Detent, string> = {
        closed: "0px",
        peek: "72px",
        half: "45dvh",
        full: "80dvh",
    };

    onMount(async () => {
        await yt.mount(video, initialSeek);
        onSeekConsumed?.();
        const media = window.matchMedia("(orientation: landscape)");
        landscape = media.matches;
        media.addEventListener("change", onOrientation);
    });

    onDestroy(async () => {
        window.matchMedia("(orientation: landscape)").removeEventListener("change", onOrientation);
        const latest = appState.videos.find((v) => v.id === video.id) ?? video;
        await yt.destroy(latest);
        await appState.refreshVideos();
    });

    function onOrientation(e: MediaQueryListEvent) {
        landscape = e.matches;
        if (landscape) sheet = "closed";
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

    /// Back closes the sheet before it leaves the video, so the gesture always
    /// undoes the most recent thing rather than skipping a level.
    function handleBack() {
        if (naming) {
            naming = false;
            yt.clearPending();
        } else if (sheet === "full" || sheet === "half") {
            sheet = "peek";
        } else {
            appState.goBack();
        }
    }

    function onMarkTap() {
        if (yt.pendingStart === null) {
            yt.markIn();
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

    function openInExternal() {
        const native = (window as any).yClippyNative;
        native?.openInRevanced?.(video.id, Math.floor(yt.currentTime));
    }
</script>

<div class="flex flex-col h-full min-h-0 bg-black relative">
    {#if !landscape}
        <header
            class="shrink-0 flex items-center gap-2 h-12 px-2 border-b border-[color:var(--border)] bg-[color:var(--surface)]"
        >
            <button
                onclick={handleBack}
                class="size-11 grid place-items-center rounded text-[color:var(--text-dim)]"
                aria-label="Back">‹</button
            >
            <span class="flex-1 min-w-0 truncate text-sm text-[color:var(--text)]">
                {video.title}
            </span>
            <button
                onclick={openInExternal}
                class="size-11 grid place-items-center rounded text-[color:var(--text-dim)]"
                aria-label="Open in another player">⧉</button
            >
        </header>
    {/if}

    <!-- Sticky player. -->
    <div class="shrink-0 bg-black" style="aspect-ratio: 16/9">
        <div id={yt.elementId} class="w-full h-full"></div>
    </div>

    {#if !landscape}
        <!-- Transport, in the bottom third where thumbs are. -->
        <div class="flex-1 min-h-0 flex flex-col justify-end">
            <div class="px-3 pt-3 pb-1">
                <Track
                    clips={appState.activeClips}
                    duration={yt.duration}
                    currentTime={yt.currentTime}
                    watched={video.last_position}
                    pendingStart={yt.pendingStart}
                    pendingEnd={yt.pendingEnd}
                    height={64}
                    onSeek={(t) => yt.seek(t)}
                    onClipTap={(clip) => yt.seek(clip.start_time)}
                    onPendingChange={(start, end) => {
                        yt.pendingStart = start;
                        yt.pendingEnd = end;
                    }}
                />
            </div>

            <div class="flex items-center justify-center gap-2 px-3 pb-2">
                <button
                    class="size-14 grid place-items-center rounded-full text-[color:var(--text-dim)] active:bg-[color:var(--surface-hi)]"
                    onclick={() => yt.skip(-10)}
                    aria-label="Back 10 seconds">⏮</button
                >
                <button
                    class="size-14 grid place-items-center rounded-full text-2xl text-[color:var(--text)] active:bg-[color:var(--surface-hi)]"
                    onclick={() => yt.toggle()}
                    aria-label={yt.isPaused ? "Play" : "Pause"}
                >
                    {yt.isPaused ? "▶" : "❚❚"}
                </button>
                <button
                    class="size-14 grid place-items-center rounded-full text-[color:var(--text-dim)] active:bg-[color:var(--surface-hi)]"
                    onclick={() => yt.skip(10)}
                    aria-label="Forward 10 seconds">⏭</button
                >
            </div>
        </div>
    {/if}

    <!-- Clips sheet: a scrim and three detents, never covering the transport
         at peek, and back dismisses it. -->
    {#if !landscape && sheet !== "closed"}
        {#if sheet === "half" || sheet === "full"}
            <button
                class="absolute inset-0 z-20 bg-black/50"
                aria-label="Close clips"
                onclick={() => (sheet = "peek")}
            ></button>
        {/if}
        <div
            class="absolute inset-x-0 bottom-0 z-30 flex flex-col rounded-t-2xl border-t border-[color:var(--border-hi)] bg-[color:var(--surface)] transition-[height] duration-200"
            style="height: {SHEET_HEIGHT[sheet]}; padding-bottom: var(--safe-bottom)"
        >
            <button
                class="shrink-0 h-[72px] px-4 flex items-center gap-3 text-left"
                onclick={() => (sheet = sheet === "peek" ? "half" : "peek")}
                aria-label={sheet === "peek" ? "Open clips" : "Collapse clips"}
            >
                <span class="block w-9 h-1 rounded-full bg-[color:var(--border-hi)]"></span>
                <span class="text-sm text-[color:var(--text)]">
                    Clips <span class="text-[color:var(--text-faint)]"
                        >{appState.activeClips.length}</span
                    >
                </span>
                <span class="flex-1"></span>
                {#if appState.activeClips[0]}
                    <span class="text-[11px] t-num text-[color:var(--text-faint)]">
                        next {formatClock(appState.activeClips[0].start_time)}
                    </span>
                {/if}
            </button>
            {#if sheet !== "peek"}
                <div class="flex-1 min-h-0 overflow-y-auto">
                    <ClipList videoId={video.id} seekTo={(t) => yt.seek(t)} />
                </div>
            {/if}
        </div>
    {/if}

    <!-- One button clips. Tap for in, tap again for out, then name it. -->
    {#if !landscape}
        <div
            class="absolute inset-x-0 bottom-0 z-40"
            style="padding-bottom: var(--safe-bottom)"
        >
            {#if naming}
                <form
                    class="flex items-center gap-2 p-2 bg-[color:var(--surface)] border-t border-[color:var(--border-hi)]"
                    onsubmit={(e) => {
                        e.preventDefault();
                        commitClip();
                    }}
                >
                    <input
                        bind:this={nameField}
                        bind:value={clipTitle}
                        placeholder="Name this clip…"
                        class="flex-1 min-w-0 h-12 bg-[color:var(--bg)] border border-[color:var(--border-hi)] rounded px-3 text-[color:var(--text)] focus:outline-none focus:border-[color:var(--accent)]"
                    />
                    <button
                        type="submit"
                        class="h-12 px-4 rounded bg-[color:var(--accent)] text-white text-sm shrink-0"
                        >Save</button
                    >
                    <button
                        type="button"
                        class="h-12 px-3 text-[color:var(--text-faint)] shrink-0"
                        onclick={() => {
                            naming = false;
                            yt.clearPending();
                        }}>✕</button
                    >
                </form>
            {:else if sheet === "peek" || sheet === "closed"}
                <button
                    class="w-full h-14 text-sm tracking-[0.15em] uppercase transition-colors {yt.pendingStart !==
                    null
                        ? 'bg-[color:var(--accent)] text-white'
                        : 'bg-[color:var(--surface-hi)] text-[color:var(--text-dim)]'}"
                    style="margin-bottom: 72px"
                    onclick={onMarkTap}
                >
                    {yt.pendingStart === null ? "Mark in" : "Mark out"}
                </button>
            {/if}
        </div>
    {/if}

    {#if landscape}
        <!-- Rotating means you stopped clipping and started watching. -->
        <button
            class="absolute top-2 left-2 z-30 size-11 grid place-items-center rounded-full bg-black/60 text-white"
            onclick={handleBack}
            aria-label="Back">‹</button
        >
    {/if}
</div>
