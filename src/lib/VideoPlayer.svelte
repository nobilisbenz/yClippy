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
    import ScrubBar from "./ScrubBar.svelte";

    let { video, seekToTime, seekTo = $bindable() } = $props<{
        video: Video;
        seekToTime?: number;
        seekTo?: (t: number) => void;
    }>();
    let player: any = $state(null);
    let currentTime = $state(0);
    let videoDuration = $state(0);
    let playerContainer: HTMLElement;
    let playerContainerId = `player-${Math.random().toString(36).slice(2, 9)}`;
    let isPaused = $state(true);

    export function getPlayer(): any {
        return player;
    }

    let startTimestamp = $state<number | null>(null);
    let endTimestamp = $state<number | null>(null);
    let isSavingClip = $state(false);

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

        const t = setInterval(() => {
            if (player && player.getCurrentTime) {
                currentTime = player.getCurrentTime();
            }
            if (player?.getDuration) {
                const d = player.getDuration();
                if (d > 0) videoDuration = d;
            }
        }, 1000);
        timer = t;

        window.addEventListener("keydown", handleKeyDown);
    });

    let timer: number;

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

    function setStart() {
        startTimestamp = currentTime;
    }

    function setEnd() {
        if (startTimestamp === null) {
            startTimestamp = Math.max(0, currentTime - 5);
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

    function onPlayerStateChange(event: any) {
        if (player && player.getCurrentTime) {
            currentTime = player.getCurrentTime();
        }
        if (player?.getDuration) {
            const d = player.getDuration();
            if (d > 0) videoDuration = d;
        }
        if (event.data === 1) {
            isPaused = false;
        } else if (event.data === 0 || event.data === 2) {
            isPaused = true;
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

    function skip(delta: number) {
        if (player?.seekTo && player?.getCurrentTime) {
            const next = Math.max(0, player.getCurrentTime() + delta);
            player.seekTo(next, true);
            currentTime = next;
        }
    }

    function handleKeyDown(e: KeyboardEvent) {
        if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
        switch (e.key) {
            case " ":
                e.preventDefault();
                togglePlay();
                break;
            case "j":
                skip(-10);
                break;
            case "l":
                skip(10);
                break;
            case "ArrowLeft":
                skip(-5);
                break;
            case "ArrowRight":
                skip(5);
                break;
            case "[":
                setStart();
                break;
            case "]":
                setEnd();
                break;
            case "Escape":
                handleBack();
                break;
        }
    }

    onDestroy(async () => {
        clearInterval(timer);
        window.removeEventListener("keydown", handleKeyDown);
        if (player && player.destroy) {
            try {
                player.destroy();
            } catch (e) {
                console.error("Failed to destroy player:", e);
            }
            player = null;
        }
        if (video && video.id) {
            const positionUpdate: Video = { ...video, last_position: Math.floor(currentTime) };
            try {
                await saveVideo(positionUpdate);
                await appState.refreshVideos();
            } catch (e) {
                console.error("Failed to save video position:", e);
            }
        }
    });

    function handleBack() {
        appState.goBack();
    }

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
        if (diff > 50) {
            appState.isClipsSidebarOpen = true;
        }
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
            <div id={playerContainerId} class="absolute inset-0 w-full h-full"></div>
            {#if isPaused}
                <div
                    class="absolute inset-0 z-20 bg-black/20 flex flex-col items-center justify-center cursor-pointer transition-opacity duration-200"
                    onclick={togglePlay}
                    role="button"
                    tabindex="0"
                    onkeydown={(e) => e.key === "Enter" && togglePlay()}
                >
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
            class="border-t border-zinc-900 bg-zinc-950 px-4 py-3 flex flex-col gap-2 z-10 shrink-0"
            role="region"
            aria-label="Transport bar"
        >
            <ScrubBar
                {player}
                {currentTime}
                duration={videoDuration}
                clips={appState.activeClips}
                startMarker={startTimestamp}
                endMarker={endTimestamp}
                onSeek={(t) => {
                    if (player?.seekTo) {
                        player.seekTo(t, true);
                        currentTime = t;
                    }
                }}
            />
            <div class="flex items-center justify-between gap-2 flex-wrap">
                <div class="flex items-center gap-2">
                    <span class="text-lg font-mono text-white t-num">
                        {formatTime(currentTime)}
                    </span>
                    <span class="text-xs text-zinc-600">/</span>
                    <span class="text-xs font-mono text-zinc-500 t-num">
                        {formatTime(videoDuration)}
                    </span>
                    {#if startTimestamp !== null}
                        <span class="text-xs text-blue-400 ml-2">
                            In: {startTimestamp.toFixed(1)}s
                            {#if endTimestamp !== null}
                                Out: {endTimestamp.toFixed(1)}s
                            {/if}
                        </span>
                    {/if}
                </div>

                <div class="flex items-center gap-2">
                    <button
                        onclick={handleBack}
                        class="px-3 py-1.5 bg-zinc-800 hover:bg-zinc-700 text-zinc-300 rounded text-sm transition-colors border border-zinc-700"
                        aria-label="Back"
                        title="Esc"
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
                                d="M10 19l-7-7m0 0l7-7m-7 7h18"
                            />
                        </svg>
                    </button>
                    <button
                        onclick={() => skip(-10)}
                        class="px-3 py-1.5 bg-zinc-800 hover:bg-zinc-700 text-zinc-300 rounded text-sm transition-colors border border-zinc-700"
                        aria-label="Back 10 seconds"
                        title="j"
                    >
                        −10s
                    </button>
                    <button
                        onclick={togglePlay}
                        class="px-3 py-1.5 bg-zinc-800 hover:bg-zinc-700 text-zinc-300 rounded text-sm transition-colors border border-zinc-700"
                        aria-label={isPaused ? "Play" : "Pause"}
                        title="Space"
                    >
                        {isPaused ? "▶" : "❚❚"}
                    </button>
                    <button
                        onclick={() => skip(10)}
                        class="px-3 py-1.5 bg-zinc-800 hover:bg-zinc-700 text-zinc-300 rounded text-sm transition-colors border border-zinc-700"
                        aria-label="Forward 10 seconds"
                        title="l"
                    >
                        +10s
                    </button>
                    <button
                        onclick={setStart}
                        class="px-3 py-1.5 bg-zinc-800 hover:bg-zinc-700 text-white rounded text-sm transition-colors border border-zinc-700"
                        title="["
                    >
                        {startTimestamp !== null ? "↺ In" : "[ In"}
                    </button>
                    <button
                        onclick={setEnd}
                        class="px-3 py-1.5 bg-red-600 hover:bg-red-700 text-white rounded text-sm transition-colors shadow-lg shadow-red-900/20"
                        title="]"
                    >
                        ] Out
                    </button>
                </div>
            </div>
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
