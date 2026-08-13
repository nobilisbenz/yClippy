<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { type Video, saveVideo, formatTime } from "./db";
    import { appState } from "./state.svelte";
    import ClipSaveModal from "./ClipSaveModal.svelte";
    import ClipList from "./ClipList.svelte";

    let { video, seekToTime, seekTo = $bindable() } = $props<{
        video: Video;
        seekToTime?: number;
        seekTo?: (t: number) => void;
    }>();

    let currentTime = $state(0);
    let isPaused = $state(true);
    let isReady = $state(false);
    let controlsVisible = $state(true);
    let hideControlsTimer: number | undefined;

    function resetHideControlsTimer() {
        if (hideControlsTimer !== undefined) {
            clearTimeout(hideControlsTimer);
        }
        hideControlsTimer = setTimeout(() => {
            if (!isPaused) controlsVisible = false;
        }, 3000) as unknown as number;
    }

    function showControls() {
        controlsVisible = true;
        resetHideControlsTimer();
    }

    let startTimestamp = $state<number | null>(null);
    let endTimestamp = $state<number | null>(null);
    let isSavingClip = $state(false);

    let player: any;
    let timer: number;
    let playerContainerId = `native-player-${Math.random().toString(36).slice(2, 9)}`;

    function loadYouTubeApi(): Promise<void> {
        return new Promise((resolve) => {
            if (window.YT && window.YT.Player) {
                resolve();
                return;
            }
            const existing = document.querySelector('script[data-yt-api]');
            if (existing) {
                const prev = window.onYouTubeIframeAPIReady;
                window.onYouTubeIframeAPIReady = () => {
                    if (prev) prev();
                    resolve();
                };
                return;
            }
            const tag = document.createElement("script");
            tag.src = "https://www.youtube.com/iframe_api";
            tag.setAttribute("data-yt-api", "true");
            const firstScriptTag = document.getElementsByTagName("script")[0];
            firstScriptTag.parentNode?.insertBefore(tag, firstScriptTag);
            window.onYouTubeIframeAPIReady = () => resolve();
        });
    }

    onMount(async () => {
        await loadYouTubeApi();
        if (!video) return;
        initPlayer();

        timer = setInterval(() => {
            if (player?.getCurrentTime) {
                currentTime = player.getCurrentTime();
            }
        }, 1000);
    });

    function initPlayer() {
        const startSeconds = Math.max(video.last_position, video.start_time);
        const playerVars: any = {
            playsinline: 1,
            start: startSeconds,
            rel: 0,
            modestbranding: 1,
            iv_load_policy: 3,
        };
        if (video.end_time > 0) {
            playerVars.end = video.end_time;
        }

        player = new window.YT.Player(playerContainerId, {
            height: "100%",
            width: "100%",
            videoId: video.id,
            playerVars,
            events: {
                onReady: (event: any) => {
                    isReady = true;
                    if (seekToTime !== undefined && event.target?.seekTo) {
                        event.target.seekTo(seekToTime, true);
                    }
                },
                onStateChange: onPlayerStateChange,
            },
        });
    }

    $effect(() => {
        if (seekToTime !== undefined && player?.seekTo) {
            player.seekTo(seekToTime, true);
            currentTime = seekToTime;
        }
    });

    $effect(() => {
        seekTo = (t: number) => {
            if (player?.seekTo) {
                player.seekTo(t, true);
                currentTime = t;
            }
        };
    });

    function onPlayerStateChange(event: any) {
        if (player?.getCurrentTime) {
            currentTime = player.getCurrentTime();
        }
        if (event.data === 1) {
            isPaused = false;
        } else if (event.data === 0 || event.data === 2) {
            isPaused = true;
        }
    }

    function togglePlay() {
        if (!player) return;
        if (isPaused) {
            player.playVideo();
        } else {
            player.pauseVideo();
        }
    }

    function openInRevanced() {
        const native = window as any;
        const t = Math.floor(currentTime);
        if (native.yClippyNative?.openInRevanced) {
            native.yClippyNative.openInRevanced(video.id, t);
        } else {
            window.open(`https://www.youtube.com/watch?v=${video.id}&t=${t}s`, "_blank");
        }
    }

    function setStart() {
        startTimestamp = currentTime;
    }

    function setEnd() {
        if (startTimestamp === null) {
            startTimestamp = Math.max(0, currentTime - 5);
        }
        endTimestamp = currentTime;
        if (player?.pauseVideo) {
            player.pauseVideo();
        }
        isSavingClip = true;
    }

    function cancelClip() {
        isSavingClip = false;
        startTimestamp = null;
        endTimestamp = null;
    }

    onDestroy(() => {
        clearInterval(timer);
        if (hideControlsTimer !== undefined) clearTimeout(hideControlsTimer);
        if (player && player.destroy) {
            try {
                player.destroy();
            } catch (e) {
                console.error("Failed to destroy native player:", e);
            }
            player = null;
        }
        if (video && video.id) {
            const positionUpdate: Video = { ...video, last_position: Math.floor(currentTime) };
            saveVideo(positionUpdate).then(() => appState.refreshVideos()).catch((e) => {
                console.error("Failed to save native player position:", e);
            });
        }
    });

    function handleBack() {
        appState.goBack();
    }
</script>

<div class="flex flex-col h-full" data-orientation="portrait">
    <div
        class="flex-1 bg-black relative min-h-0"
        onclick={showControls}
        onkeydown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                showControls();
            }
        }}
        role="button"
        tabindex="0"
        aria-label="Show player controls"
    >
        {#if !isReady}
            <div
                class="absolute inset-0 z-10 flex items-center justify-center bg-black"
            >
                <div class="text-white flex flex-col items-center gap-4">
                    <div
                        class="animate-spin rounded-full h-8 w-8 border-b-2 border-white"
                    ></div>
                    <div>Loading video…</div>
                </div>
            </div>
        {/if}

        <div id={playerContainerId} class="absolute inset-0 w-full h-full"></div>

        {#if isPaused && isReady}
            <div
                class="absolute inset-0 z-20 bg-black/30 flex items-center justify-center cursor-pointer"
                onclick={togglePlay}
                role="button"
                tabindex="0"
                onkeydown={(e) => e.key === "Enter" && togglePlay()}
            >
                <div
                    class="bg-black/70 rounded-full p-5 hover:scale-110 transition-transform shadow-2xl border border-zinc-600/50"
                >
                    <svg class="size-14 text-white" fill="currentColor" viewBox="0 0 24 24">
                        <path d="M8 5v14l11-7z" />
                    </svg>
                </div>
            </div>
        {/if}
    </div>

    <div
        class="border-t border-zinc-900 bg-zinc-950 p-3 flex flex-col gap-3 shrink-0 transition-opacity duration-300 {controlsVisible ? 'opacity-100' : 'opacity-0 pointer-events-none'}"
        style="padding-bottom: calc(12px + var(--safe-bottom));"
    >
        <div class="flex items-center justify-between gap-3">
            <div class="text-xl font-mono text-white t-num">
                {formatTime(currentTime)}
            </div>
            <div class="flex items-center gap-2">
                <button
                    onclick={handleBack}
                    class="min-w-[48px] min-h-[48px] flex items-center justify-center bg-zinc-800 hover:bg-zinc-700 rounded-full transition-colors"
                    aria-label="Back"
                >
                    <svg
                        class="size-5 text-zinc-300"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                    >
                        <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M10 19l-7-7m0 0l7-7m-7 7h18"
                        />
                    </svg>
                </button>

                <button
                    onclick={togglePlay}
                    class="min-w-[48px] min-h-[48px] flex items-center justify-center bg-zinc-800 hover:bg-zinc-700 rounded-full transition-colors"
                    aria-label={isPaused ? "Play" : "Pause"}
                >
                    {#if isPaused}
                        <svg class="size-5 text-zinc-300" fill="currentColor" viewBox="0 0 24 24">
                            <path d="M8 5v14l11-7z" />
                        </svg>
                    {:else}
                        <svg class="size-5 text-zinc-300" fill="currentColor" viewBox="0 0 24 24">
                            <path d="M6 4h4v16H6V4zm8 0h4v16h-4V4z" />
                        </svg>
                    {/if}
                </button>

                <button
                    onclick={setStart}
                    class="min-h-[48px] px-4 bg-zinc-700 hover:bg-zinc-600 text-white rounded-lg text-sm font-medium transition-colors"
                >
                    Set Start
                </button>

                <button
                    onclick={setEnd}
                    class="min-h-[48px] px-5 bg-blue-600 hover:bg-blue-500 text-white rounded-lg text-sm font-medium transition-colors"
                >
                    Clip
                </button>
            </div>
        </div>

        <button
            onclick={openInRevanced}
            class="min-h-[48px] w-full px-4 bg-red-600 hover:bg-red-500 text-white rounded-lg text-sm font-medium transition-colors flex items-center justify-center gap-2"
            title="Hand off to ReVanced Extended for background playback"
        >
            <svg class="size-4" fill="currentColor" viewBox="0 0 24 24">
                <path d="M10 9V15L15 12L10 9M12 2C6.48 2 2 6.48 2 12C2 17.52 6.48 22 12 22C17.52 22 22 17.52 22 12C22 6.48 17.52 2 12 2M12 20C7.59 20 4 16.41 4 12C4 7.59 7.59 4 12 4C16.41 4 20 7.59 20 12C20 16.41 16.41 20 12 20Z" />
            </svg>
            Open in ReVanced Extended
        </button>

        <button
            onclick={() => (appState.isClipsSidebarOpen = !appState.isClipsSidebarOpen)}
            class="min-h-[48px] w-full px-4 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded-lg text-sm font-medium transition-colors flex items-center justify-center gap-2"
            aria-label="Clips"
        >
            <svg
                class="size-4"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
            >
                <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
                />
            </svg>
            Clips ({appState.activeClips.length})
        </button>
    </div>

    {#if appState.isClipsSidebarOpen}
        <div
            class="fixed md:absolute inset-x-0 bottom-0 md:right-0 md:left-auto md:top-0 md:bottom-auto md:w-80 max-h-[70vh] md:max-h-full border-t md:border-t-0 md:border-l border-zinc-900 bg-zinc-950 shrink-0 overflow-y-auto z-40 shadow-2xl rounded-t-2xl md:rounded-none"
            style="padding-bottom: calc(12px + var(--safe-bottom));"
        >
            <div class="sticky top-0 bg-zinc-950 border-b border-zinc-900 p-4 flex items-center justify-between">
                <span class="font-bold">Clips</span>
                <button
                    onclick={() => (appState.isClipsSidebarOpen = false)}
                    class="min-w-[48px] min-h-[48px] flex items-center justify-center hover:bg-zinc-800 rounded-full"
                    aria-label="Close Clips"
                >
                    <svg class="size-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                    </svg>
                </button>
            </div>
            <ClipList videoId={video.id} seekTo={(t) => {
                currentTime = t;
                if (player?.seekTo) {
                    player.seekTo(t, true);
                }
            }} />
        </div>
    {/if}

    {#if isSavingClip && startTimestamp !== null && endTimestamp !== null}
        <ClipSaveModal
            {video}
            startTime={startTimestamp}
            endTime={endTimestamp}
            onClose={cancelClip}
            onSaved={cancelClip}
        />
    {/if}
</div>
