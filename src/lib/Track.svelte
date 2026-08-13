<script lang="ts">
    import type { Clip } from "./db";
    import { formatClock } from "./youtube.svelte";

    /// The timeline, and a map of what you have already captured.
    ///
    /// The clip list and this bar are two projections of one thing: hovering a
    /// clip lights its range here, and dragging a range here becomes a clip
    /// row. That reciprocity is what turns a video player into a clipping tool
    /// — the timeline stops being a control and starts being the document.
    let {
        clips = [],
        duration = 0,
        currentTime = 0,
        watched = 0,
        pendingStart = null,
        pendingEnd = null,
        highlightId = null,
        height = 44,
        onSeek,
        onClipTap,
        onPendingChange,
    } = $props<{
        clips?: Clip[];
        duration?: number;
        currentTime?: number;
        watched?: number;
        pendingStart?: number | null;
        pendingEnd?: number | null;
        highlightId?: number | null;
        height?: number;
        onSeek?: (seconds: number) => void;
        onClipTap?: (clip: Clip) => void;
        onPendingChange?: (start: number, end: number | null) => void;
    }>();

    let bar = $state<HTMLElement | null>(null);
    let dragging = $state<null | "scrub" | "in" | "out">(null);
    let hoverAt = $state<number | null>(null);

    const pct = (seconds: number) =>
        duration > 0 ? Math.min(100, Math.max(0, (seconds / duration) * 100)) : 0;

    function secondsAt(clientX: number): number {
        if (!bar || duration <= 0) return 0;
        const box = bar.getBoundingClientRect();
        const ratio = (clientX - box.left) / box.width;
        return Math.min(duration, Math.max(0, ratio * duration));
    }

    /// Clips stack into lanes so overlapping ranges stay legible instead of
    /// painting over each other.
    const lanes = $derived.by(() => {
        const ordered = [...clips].sort((a, b) => a.start_time - b.start_time);
        const ends: number[] = [];
        const placed: { clip: Clip; lane: number }[] = [];
        for (const clip of ordered) {
            const end = clip.end_time > clip.start_time ? clip.end_time : clip.start_time + 1;
            let lane = ends.findIndex((busyUntil) => clip.start_time >= busyUntil);
            if (lane === -1) {
                lane = Math.min(ends.length, 2);
                if (ends.length < 3) ends.push(end);
                else ends[lane] = Math.max(ends[lane], end);
            } else {
                ends[lane] = end;
            }
            placed.push({ clip, lane });
        }
        return placed;
    });

    const laneCount = $derived(Math.max(1, ...lanes.map((l: { lane: number }) => l.lane + 1)));

    function beginDrag(kind: "scrub" | "in" | "out", event: PointerEvent) {
        event.preventDefault();
        (event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId);
        dragging = kind;
        applyDrag(event.clientX);
    }

    function applyDrag(clientX: number) {
        const at = secondsAt(clientX);
        if (dragging === "scrub") {
            onSeek?.(at);
        } else if (dragging === "in") {
            onPendingChange?.(Math.min(at, (pendingEnd ?? duration) - 1), pendingEnd);
        } else if (dragging === "out") {
            onPendingChange?.(pendingStart ?? 0, Math.max(at, (pendingStart ?? 0) + 1));
        }
    }

    function onPointerMove(event: PointerEvent) {
        hoverAt = secondsAt(event.clientX);
        if (dragging) applyDrag(event.clientX);
    }

    function onPointerUp() {
        dragging = null;
    }

    function onTrackKey(event: KeyboardEvent) {
        const step = event.shiftKey ? 60 : 5;
        if (event.key === "ArrowLeft") {
            event.preventDefault();
            onSeek?.(Math.max(0, currentTime - step));
        } else if (event.key === "ArrowRight") {
            event.preventDefault();
            onSeek?.(Math.min(duration, currentTime + step));
        }
    }
</script>

<svelte:window on:pointerup={onPointerUp} />

<div class="w-full select-none" style="--track-h: {height}px">
    <div
        bind:this={bar}
        role="slider"
        tabindex="0"
        aria-label="Timeline"
        aria-valuemin={0}
        aria-valuemax={Math.floor(duration)}
        aria-valuenow={Math.floor(currentTime)}
        aria-valuetext={formatClock(currentTime)}
        class="relative w-full cursor-pointer rounded-sm bg-[color:var(--surface-hi)] overflow-hidden touch-none"
        style="height: var(--track-h)"
        onpointerdown={(e) => beginDrag("scrub", e)}
        onpointermove={onPointerMove}
        onpointerleave={() => (hoverAt = null)}
        onkeydown={onTrackKey}
    >
        <!-- How far you actually got. This is what `last_position` means, shown
             as a shape rather than an unlabelled number in the library. -->
        <div
            class="absolute inset-y-0 left-0 bg-white/[0.06] pointer-events-none"
            style="width: {pct(watched)}%"
        ></div>

        <!-- Existing clips, as ranges. -->
        {#each lanes as { clip, lane } (clip.id ?? clip.uid ?? clip.start_time)}
            {@const end = clip.end_time > clip.start_time ? clip.end_time : clip.start_time + 1}
            <button
                type="button"
                title={`${clip.title || "Clip"} · ${formatClock(clip.start_time)}`}
                aria-label={`Play clip ${clip.title || ""} at ${formatClock(clip.start_time)}`}
                class="absolute rounded-[2px] transition-opacity hover:opacity-100 {highlightId ===
                clip.id
                    ? 'opacity-100 ring-1 ring-white/70'
                    : 'opacity-70'}"
                style="left: {pct(clip.start_time)}%;
                       width: max(3px, {pct(end) - pct(clip.start_time)}%);
                       top: calc({lane} * (var(--track-h) / {laneCount}) + 2px);
                       height: calc(var(--track-h) / {laneCount} - 4px);
                       background: var(--accent);"
                onpointerdown={(e) => e.stopPropagation()}
                onclick={(e) => {
                    e.stopPropagation();
                    onClipTap?.(clip);
                }}
            ></button>
        {/each}

        <!-- The clip being marked, before it is named. Draggable by either
             edge, because you mark roughly and adjust precisely. -->
        {#if pendingStart !== null}
            {@const pendEnd = pendingEnd ?? currentTime}
            <div
                class="absolute inset-y-0 border-x-2 pointer-events-none"
                style="left: {pct(pendingStart)}%;
                       width: max(2px, {pct(Math.max(pendEnd, pendingStart)) - pct(pendingStart)}%);
                       background: color-mix(in srgb, var(--accent) 45%, transparent);
                       border-color: var(--text);"
            ></div>
            <button
                type="button"
                aria-label="Drag clip start"
                class="absolute inset-y-0 w-3 -translate-x-1/2 cursor-ew-resize bg-transparent"
                style="left: {pct(pendingStart)}%"
                onpointerdown={(e) => {
                    e.stopPropagation();
                    beginDrag("in", e);
                }}
            ></button>
            {#if pendingEnd !== null}
                <button
                    type="button"
                    aria-label="Drag clip end"
                    class="absolute inset-y-0 w-3 -translate-x-1/2 cursor-ew-resize bg-transparent"
                    style="left: {pct(pendingEnd)}%"
                    onpointerdown={(e) => {
                        e.stopPropagation();
                        beginDrag("out", e);
                    }}
                ></button>
            {/if}
        {/if}

        <!-- Playhead. -->
        <div
            class="absolute inset-y-0 w-[2px] bg-[color:var(--text)] pointer-events-none"
            style="left: {pct(currentTime)}%"
        ></div>

        {#if hoverAt !== null && duration > 0}
            <div
                class="absolute inset-y-0 w-px bg-white/30 pointer-events-none"
                style="left: {pct(hoverAt)}%"
            ></div>
        {/if}
    </div>

    <div
        class="flex items-baseline justify-between mt-1 text-[11px] t-num text-[color:var(--text-faint)]"
    >
        <span class="text-[color:var(--text-dim)]">{formatClock(currentTime)}</span>
        {#if hoverAt !== null && duration > 0}
            <span>{formatClock(hoverAt)}</span>
        {/if}
        <span>{formatClock(duration)}</span>
    </div>
</div>
