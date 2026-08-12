<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import {
        type Video,
        saveClip,
        type Clip,
        saveVideo,
        formatTime,
    } from "./db";
    import { appState } from "./state.svelte";
    import ClipSaveModal from "./ClipSaveModal.svelte";
    import ClipList from "./ClipList.svelte";

    let { video } = $props<{ video: Video }>();
    let player: any;
    let currentTime = $state(0);
    let playerContainer: HTMLElement;

    // Clipping State
    let startTimestamp = $state<number | null>(null);
    let endTimestamp = $state<number | null>(null);
    let isSavingClip = $state(false);

    onMount(async () => {
        // Load YouTube API if not loaded
        if (!window.YT) {
            const tag = document.createElement("script");
            tag.src = "https://www.youtube.com/iframe_api";
            const firstScriptTag = document.getElementsByTagName("script")[0];
            firstScriptTag.parentNode?.insertBefore(tag, firstScriptTag);
            window.onYouTubeIframeAPIReady = initPlayer;
        } else {
            initPlayer();
        }
    });

    function initPlayer() {
        const startSeconds = Math.max(video.last_position, video.start_time);
        const playerVars: any = {
            playsinline: 1,
            start: startSeconds,
            rel: 0,
            modestbranding: 1,
            iv_load_policy: 3, // Hide annotations
            fs: 0, // Hide fullscreen button
            disablekb: 1, // Disable keyboard controls if we want strict control
        };
        if (video.end_time > 0) {
            playerVars.end = video.end_time;
        }

        player = new window.YT.Player("player", {
            height: "100%",
            width: "100%",
            videoId: video.id,
            playerVars,
            events: {
                onReady: onPlayerReady,
                onStateChange: onPlayerStateChange,
            },
        });
    }

    function onPlayerReady(event: any) {
        // player ready
    }

    function setStart() {
        startTimestamp = currentTime;
    }

    function setEnd() {
        if (startTimestamp === null) {
            startTimestamp = Math.max(0, currentTime - 5); // Default to 5s before if no start set
        }
        endTimestamp = currentTime;

        if (player && player.pauseVideo) {
            player.pauseVideo();
        }

        isSavingClip = true;
    }

    function cancelClip() {
        isSavingClip = false;
        startTimestamp = null;
        endTimestamp = null;
    }

    let isPaused = $state(false);

    function onPlayerStateChange(event: any) {
        if (player && player.getCurrentTime) {
            currentTime = player.getCurrentTime();
        }

        // 1 = playing, 2 = paused
        if (event.data === 2) {
            isPaused = true;
        } else if (event.data === 1) {
            isPaused = false;
        } else {
            isPaused = false;
        }
    }

    function togglePlay() {
        if (player) {
            if (isPaused) {
                player.playVideo();
            } else {
                player.pauseVideo();
            }
        }
    }

    // Polling for time
    let timer = setInterval(() => {
        if (player && player.getCurrentTime) {
            currentTime = player.getCurrentTime();
        }
    }, 1000);

    onDestroy(async () => {
        clearInterval(timer);
        if (video) {
            video.last_position = Math.floor(currentTime);
            saveVideo(video).then(() => appState.refreshVideos());
        }
    });

    function handleBack() {
        // Use history back for Android support
        appState.goBack();
    }

    // Swipe Gestures
    let touchStartX = 0;
    let touchEndX = 0;

    function handleTouchStart(e: TouchEvent) {
        touchStartX = e.changedTouches[0].screenX;
    }

    function handleTouchEnd(e: TouchEvent) {
        touchEndX = e.changedTouches[0].screenX;
        handleSwipe();
    }

    function handleSwipe() {
        const diff = touchStartX - touchEndX;
        // Swipe Left (drag right to left) -> Open Sidebar
        if (diff > 50) {
            appState.isClipsSidebarOpen = true;
        }
        // Swipe Right (drag left to right) -> Close Sidebar
        if (diff < -50) {
            appState.isClipsSidebarOpen = false;
        }
    }
</script>

<div
    class="flex flex-col md:flex-row h-full relative"
    ontouchstart={handleTouchStart}
    ontouchend={handleTouchEnd}
    role="presentation"
>
    <div class="flex-1 flex flex-col relative z-0 min-h-0">
        <div class="flex-1 bg-black relative">
            <div id="player" class="absolute inset-0 w-full h-full"></div>
            <!-- Custom Pause Overlay to hide YouTube clutter -->
            {#if isPaused}
                <div
                    class="absolute inset-0 z-20 bg-black/20 flex flex-col items-center justify-center cursor-pointer transition-opacity duration-200"
                    onclick={togglePlay}
                    role="button"
                    tabindex="0"
                    onkeydown={(e) => e.key === "Enter" && togglePlay()}
                >
                    <!-- Back Button in Overlay -->
                    <button
                        onclick={(e) => {
                            e.stopPropagation();
                            handleBack();
                        }}
                        class="absolute top-4 left-4 p-3 bg-black/70 rounded-full hover:bg-black/90 transition-colors border border-zinc-600/50 shadow-lg group"
                        aria-label="Back to Library"
                    >
                        <svg
                            class="size-6 text-zinc-400 group-hover:text-white"
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

                    <div
                        class="bg-black/70 rounded-full p-4 hover:scale-110 transition-transform shadow-2xl border border-zinc-600/50"
                    >
                        <svg
                            class="size-12 text-white"
                            fill="currentColor"
                            viewBox="0 0 24 24"
                        >
                            <path d="M8 5v14l11-7z" />
                        </svg>
                    </div>
                </div>
            {/if}
        </div>
        <div
            class="h-24 border-t border-zinc-900 bg-zinc-950 p-4 flex items-center justify-between z-10 shrink-0"
        >
            <div class="flex flex-col">
                <div class="text-xl font-mono text-white">
                    {formatTime(currentTime)}
                </div>
                {#if startTimestamp !== null}
                    <div class="text-xs text-blue-400">
                        Start Set: {startTimestamp.toFixed(1)}s
                    </div>
                {/if}
            </div>

            <div class="flex gap-4">
                <button
                    onclick={handleBack}
                    class="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-zinc-300 rounded text-sm font-medium transition-colors border border-zinc-700 flex items-center gap-2"
                    aria-label="Back"
                >
                    <svg
                        class="size-5"
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
                    onclick={() =>
                        (appState.isClipsSidebarOpen =
                            !appState.isClipsSidebarOpen)}
                    class="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-zinc-300 rounded text-sm font-medium transition-colors border border-zinc-700 flex items-center gap-2"
                >
                    <svg
                        class="size-5"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                    >
                        <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M4 6h16M4 12h16M4 18h16"
                        />
                    </svg>
                    <span class="hidden sm:inline">Clips</span>
                </button>
                <button
                    onclick={setStart}
                    class="px-6 py-2 bg-zinc-800 hover:bg-zinc-700 text-white rounded text-sm font-medium transition-colors border border-zinc-700"
                >
                    {startTimestamp !== null ? "Reset" : "Set Start"}
                </button>
                <button
                    onclick={setEnd}
                    class="px-6 py-2 bg-red-600 hover:bg-red-700 text-white rounded text-sm font-medium transition-colors shadow-lg shadow-red-900/20"
                >
                    Clip
                </button>
            </div>
        </div>
    </div>

    <div
        class="{appState.isClipsSidebarOpen
            ? 'h-[50vh] opacity-100 md:h-full md:w-80 border-t md:border-t-0 md:border-l'
            : 'h-0 opacity-0 md:h-full md:opacity-100 md:w-0 border-t-0 md:border-l-0'} border-zinc-900 bg-zinc-950 flex flex-col z-10 transition-all duration-300 overflow-hidden shrink-0"
    >
        <div
            class="p-4 border-b border-zinc-900 font-bold flex justify-between items-center whitespace-nowrap"
        >
            <span>Clips</span>
            <button
                onclick={() => (appState.isClipsSidebarOpen = false)}
                class="p-1 hover:bg-zinc-800 rounded"
                aria-label="Close Clips Sidebar"
            >
                <svg
                    class="size-5"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                    ><path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M6 18L18 6M6 6l12 12"
                    /></svg
                >
            </button>
        </div>
        <div class="flex-1 w-full md:w-80 overflow-y-auto overflow-x-hidden">
            <ClipList
                videoId={video.id}
                seekTo={(t) => player?.seekTo(t, true)}
            />
        </div>
    </div>

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
