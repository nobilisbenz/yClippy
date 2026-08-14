<script lang="ts">
    import { appState } from "./state.svelte";
    import { type Video } from "./db";
    import { formatClock } from "./youtube.svelte";
    import Thumbnail from "./Thumbnail.svelte";

    /// Where did I leave off? The tree answers "what do I have"; this answers
    /// the question you actually arrive with.
    ///
    /// As a `strip` it sits above the columns and takes only the height of one
    /// row of cards, so the library stays the main event.
    let { variant = "grid" }: { variant?: "grid" | "strip" } = $props();

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

{#if variant === "strip"}
    {#if recent.length > 0}
        <section
            class="shrink-0 border-b border-[color:var(--border)] bg-[color:var(--surface)] px-3 py-2.5"
        >
            <h2 class="section-label mb-2">Continue watching</h2>
            <div class="flex gap-2 overflow-x-auto scrollbar-none pb-0.5">
                {#each recent as video (video.id)}
                    <button
                        class="group shrink-0 w-[190px] text-left rounded-[6px] overflow-hidden border border-[color:var(--border)] hover:border-[color:var(--border-hi)] bg-[color:var(--bg)] transition-colors"
                        onclick={() => appState.openVideo(video)}
                        title={video.title}
                    >
                        <div class="relative aspect-video bg-black">
                            <Thumbnail
                                videoId={video.id}
                                alt=""
                                className="w-full h-full object-cover opacity-90 group-hover:opacity-100 transition-opacity"
                            />
                            <div class="absolute inset-x-0 bottom-0 h-[3px] bg-black/70">
                                <div
                                    class="h-full bg-[color:var(--accent)]"
                                    style="width: {progress(video) || 6}%"
                                ></div>
                            </div>
                            <span
                                class="absolute bottom-1.5 right-1.5 px-1 rounded bg-black/80 text-[10px] t-num text-[color:var(--text-dim)]"
                            >
                                {formatClock(video.last_position)}
                            </span>
                        </div>
                        <div
                            class="px-2 py-1.5 text-[12px] leading-snug text-[color:var(--text-dim)] group-hover:text-[color:var(--text)] truncate transition-colors"
                        >
                            {video.title}
                        </div>
                    </button>
                {/each}
            </div>
        </section>
    {/if}
{:else}
    <div class="flex-1 min-h-0 overflow-y-auto scroll-thin p-8">
        {#if recent.length === 0}
            <div class="empty">
                <p>Nothing playing.</p>
                <p>
                    Pick a video from the library, or press
                    <kbd class="kbd">Ctrl</kbd>
                    <kbd class="kbd">K</kbd>
                </p>
            </div>
        {:else}
            <h2 class="section-label mb-4">Continue watching</h2>
            <div
                class="grid gap-4"
                style="grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));"
            >
                {#each recent as video (video.id)}
                    <button
                        class="group text-left bg-[color:var(--surface)] border border-[color:var(--border)] hover:border-[color:var(--border-hi)] rounded-[6px] overflow-hidden transition-colors"
                        onclick={() => appState.openVideo(video)}
                    >
                        <div class="relative aspect-video bg-black">
                            <Thumbnail
                                videoId={video.id}
                                alt={video.title}
                                className="w-full h-full object-cover"
                            />
                            <div class="absolute inset-x-0 bottom-0 h-[3px] bg-black/70">
                                <div
                                    class="h-full bg-[color:var(--accent)]"
                                    style="width: {progress(video) || 6}%"
                                ></div>
                            </div>
                        </div>
                        <div class="p-3">
                            <div class="text-sm text-[color:var(--text)] line-clamp-2 leading-snug">
                                {video.title}
                            </div>
                            <div class="mt-1.5 text-[11px] t-num text-[color:var(--text-faint)]">
                                {formatClock(video.last_position)}
                                {#if video.end_time > 0 || video.duration > 0}
                                    / {formatClock(
                                        video.end_time > 0 ? video.end_time : video.duration,
                                    )}
                                {/if}
                            </div>
                        </div>
                    </button>
                {/each}
            </div>
        {/if}
    </div>
{/if}
