<script lang="ts">
    import type { Clip } from "./db";

    let {
        player,
        currentTime,
        duration,
        clips = [],
        startMarker = null,
        endMarker = null,
        onSeek,
    } = $props<{
        player: any;
        currentTime: number;
        duration: number;
        clips?: Clip[];
        startMarker?: number | null;
        endMarker?: number | null;
        onSeek: (t: number) => void;
    }>();

    let trackEl: HTMLElement | undefined = $state();
    let isDragging = $state(false);

    let progressPct = $derived(duration > 0 ? (currentTime / duration) * 100 : 0);

    let clipMarkers = $derived(
        clips
            .filter((c: Clip) => duration > 0)
            .map((c: Clip) => ({
                id: c.id,
                startPct: Math.max(0, Math.min(100, (c.start_time / duration) * 100)),
                endPct: Math.max(0, Math.min(100, (c.end_time / duration) * 100)),
                title: c.title,
            })),
    );

    function seekFromEvent(e: MouseEvent | TouchEvent) {
        if (!trackEl || duration <= 0) return;
        const rect = trackEl.getBoundingClientRect();
        const clientX = "touches" in e ? e.touches[0]?.clientX : (e as MouseEvent).clientX;
        if (clientX === undefined) return;
        const ratio = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
        onSeek(ratio * duration);
    }

    function handlePointerDown(e: MouseEvent | TouchEvent) {
        isDragging = true;
        seekFromEvent(e);
    }

    function handlePointerMove(e: MouseEvent | TouchEvent) {
        if (!isDragging) return;
        seekFromEvent(e);
    }

    function handlePointerUp() {
        isDragging = false;
    }

    $effect(() => {
        if (!isDragging) return;
        const onMove = (e: MouseEvent) => handlePointerMove(e);
        const onUp = () => handlePointerUp();
        window.addEventListener("mousemove", onMove);
        window.addEventListener("mouseup", onUp);
        return () => {
            window.removeEventListener("mousemove", onMove);
            window.removeEventListener("mouseup", onUp);
        };
    });

    function handleKeyDown(e: KeyboardEvent) {
        if (e.target instanceof HTMLInputElement) return;
        if (e.key === "ArrowLeft") {
            e.preventDefault();
            onSeek(Math.max(0, currentTime - 5));
        } else if (e.key === "ArrowRight") {
            e.preventDefault();
            onSeek(Math.min(duration, currentTime + 5));
        }
    }
</script>

<svelte:window onkeydown={handleKeyDown} />

<div
    bind:this={trackEl}
    class="relative h-7 bg-zinc-900 rounded-full cursor-pointer group select-none touch-none"
    onmousedown={handlePointerDown}
    ontouchstart={handlePointerDown}
    role="slider"
    aria-label="Video position"
    aria-valuemin="0"
    aria-valuemax={Math.max(1, duration)}
    aria-valuenow={currentTime}
    aria-valuetext="{Math.floor(currentTime)} seconds of {Math.floor(duration)}"
    tabindex="0"
>
    <div class="absolute inset-y-0 left-0 bg-zinc-700/40 rounded-full" style="width: 100%"></div>

    {#if duration > 0}
        {#each clipMarkers as marker (marker.id)}
            <div
                class="absolute inset-y-1 bg-blue-500/70 rounded-sm pointer-events-none"
                style="left: {marker.startPct}%; width: {Math.max(0.5, marker.endPct - marker.startPct)}%;"
                title={marker.title}
                aria-label="Clip marker"
            ></div>
        {/each}
    {/if}

    {#if startMarker !== null && duration > 0}
        {@const sPct = Math.max(0, Math.min(100, (startMarker / duration) * 100))}
        <div
            class="absolute inset-y-0 w-0.5 bg-blue-400 pointer-events-none"
            style="left: {sPct}%"
            aria-label="In point"
        ></div>
    {/if}
    {#if endMarker !== null && duration > 0}
        {@const ePct = Math.max(0, Math.min(100, (endMarker / duration) * 100))}
        <div
            class="absolute inset-y-0 w-0.5 bg-red-400 pointer-events-none"
            style="left: {ePct}%"
            aria-label="Out point"
        ></div>
    {/if}

    <div
        class="absolute inset-y-0 left-0 bg-[color:var(--accent)] rounded-full pointer-events-none"
        style="width: {progressPct}%"
    ></div>

    <div
        class="absolute top-1/2 -translate-y-1/2 -translate-x-1/2 size-4 bg-white rounded-full shadow-md opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none"
        style="left: {progressPct}%"
    ></div>
</div>

{#if duration <= 0}
    <p class="text-xs text-zinc-600">Duration not loaded yet</p>
{/if}