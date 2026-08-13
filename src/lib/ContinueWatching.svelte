<script lang="ts">
    import { appState } from "./state.svelte";
    import { formatTime, type Video } from "./db";
    import Thumbnail from "./Thumbnail.svelte";

    // The desktop main column when nothing is playing. The rail already holds
    // the tree, so this answers a different question: where did I leave off?
    const recent = $derived(
        appState.videos
            .filter((v) => v.last_position > 0)
            .sort((a, b) => b.last_position - a.last_position)
            .slice(0, 12),
    );

    function progress(v: Video): number {
        const total = v.end_time > 0 ? v.end_time : v.duration;
        if (!total || total <= 0) return 0;
        return Math.min(100, (v.last_position / total) * 100);
    }
</script>

<div class="flex-1 min-h-0 overflow-y-auto p-8">
    {#if recent.length === 0}
        <div class="h-full flex flex-col items-center justify-center gap-3 text-center">
            <div class="text-[color:var(--text-faint)] text-sm">
                Nothing playing.
            </div>
            <div class="text-[color:var(--text-faint)] text-xs">
                Pick a video from the library, or press
                <kbd
                    class="px-1.5 py-0.5 border border-[color:var(--border-hi)] rounded text-[color:var(--text-dim)]"
                    >Ctrl</kbd
                >
                <kbd
                    class="px-1.5 py-0.5 border border-[color:var(--border-hi)] rounded text-[color:var(--text-dim)]"
                    >K</kbd
                >
            </div>
        </div>
    {:else}
        <h2
            class="text-[11px] uppercase tracking-[0.15em] text-[color:var(--text-faint)] mb-4"
        >
            Continue watching
        </h2>
        <div
            class="grid gap-4"
            style="grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));"
        >
            {#each recent as video (video.id)}
                <button
                    class="group text-left bg-[color:var(--surface)] border border-[color:var(--border)] hover:border-[color:var(--border-hi)] rounded overflow-hidden transition-colors"
                    onclick={() => appState.openVideo(video)}
                >
                    <div class="relative aspect-video bg-black">
                        <Thumbnail
                            videoId={video.id}
                            alt={video.title}
                            className="w-full h-full object-cover"
                        />
                        <div
                            class="absolute bottom-0 left-0 right-0 h-[3px] bg-black/60"
                        >
                            <div
                                class="h-full bg-[color:var(--accent)]"
                                style="width: {progress(video)}%"
                            ></div>
                        </div>
                    </div>
                    <div class="p-3">
                        <div
                            class="text-sm text-[color:var(--text)] line-clamp-2 leading-snug"
                        >
                            {video.title}
                        </div>
                        <div
                            class="mt-1.5 text-[11px] t-num text-[color:var(--text-faint)]"
                        >
                            {formatTime(video.last_position)}
                            {#if video.end_time > 0 || video.duration > 0}
                                / {formatTime(
                                    video.end_time > 0
                                        ? video.end_time
                                        : video.duration,
                                )}
                            {/if}
                        </div>
                    </div>
                </button>
            {/each}
        </div>
    {/if}
</div>
