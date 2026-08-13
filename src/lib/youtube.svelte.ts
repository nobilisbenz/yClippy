import { saveVideo, type Video } from "./db";

/// Owns the YouTube iframe API and one player instance.
///
/// This exists because the desktop and Android players were 90% the same code
/// and had already drifted — they disagreed about whether "ended" counts as
/// paused, which is the kind of bug duplication guarantees eventually. There is
/// one implementation of playback now; the shells are layout only.
export class YouTubeController {
    player = $state<any>(null);
    currentTime = $state(0);
    duration = $state(0);
    /// True whenever the video is not actively playing — including ended,
    /// which is the case the two old players disagreed about.
    isPaused = $state(true);
    isReady = $state(false);
    hasEnded = $state(false);

    /// The pending clip, before it is named and saved. Keeping it as an object
    /// rather than two loose numbers is what lets the track draw it and lets
    /// you adjust either edge before committing.
    pendingStart = $state<number | null>(null);
    pendingEnd = $state<number | null>(null);

    private containerId = `yt-${Math.random().toString(36).slice(2, 9)}`;
    private ticker: number | undefined;
    private video: Video | null = null;

    get elementId() {
        return this.containerId;
    }

    /// Loads the iframe API once per document, and never leaves a global
    /// callback pointing at a destroyed component.
    private static apiReady: Promise<void> | null = null;
    private static loadApi(): Promise<void> {
        if (typeof window !== "undefined" && (window as any).YT?.Player) {
            return Promise.resolve();
        }
        if (YouTubeController.apiReady) return YouTubeController.apiReady;

        YouTubeController.apiReady = new Promise<void>((resolve) => {
            const previous = window.onYouTubeIframeAPIReady;
            window.onYouTubeIframeAPIReady = () => {
                previous?.();
                resolve();
            };
            if (!document.querySelector("script[data-yt-api]")) {
                const tag = document.createElement("script");
                tag.src = "https://www.youtube.com/iframe_api";
                tag.setAttribute("data-yt-api", "true");
                document.head.appendChild(tag);
            }
        });
        return YouTubeController.apiReady;
    }

    async mount(video: Video, startAt?: number) {
        this.video = video;
        await YouTubeController.loadApi();
        // The component may have unmounted while the script loaded.
        if (this.video !== video) return;

        const start = startAt ?? Math.max(video.last_position, video.start_time);
        const playerVars: Record<string, unknown> = {
            playsinline: 1,
            start: Math.floor(Math.max(0, start)),
            rel: 0,
            modestbranding: 1,
            iv_load_policy: 3,
            // Re-enabled: switching these off removed YouTube's own fullscreen
            // and keyboard control without replacing either.
            fs: 1,
        };
        if (video.end_time > 0) playerVars.end = video.end_time;

        this.player = new (window as any).YT.Player(this.containerId, {
            height: "100%",
            width: "100%",
            videoId: video.id,
            playerVars,
            events: {
                onReady: (event: any) => {
                    this.isReady = true;
                    if (startAt !== undefined) event.target?.seekTo?.(startAt, true);
                    this.sample();
                },
                onStateChange: (event: any) => {
                    // 1 = playing. Everything else — paused, ended, buffering,
                    // cued — is "not playing", stated once.
                    this.isPaused = event.data !== 1;
                    this.hasEnded = event.data === 0;
                    this.sample();
                },
            },
        });

        this.ticker = setInterval(() => this.sample(), 500) as unknown as number;
    }

    private sample() {
        const p = this.player;
        if (!p) return;
        if (typeof p.getCurrentTime === "function") {
            const t = p.getCurrentTime();
            if (typeof t === "number" && !Number.isNaN(t)) this.currentTime = t;
        }
        if (typeof p.getDuration === "function") {
            const d = p.getDuration();
            if (typeof d === "number" && d > 0) this.duration = d;
        }
    }

    seek(seconds: number) {
        const target = Math.max(0, seconds);
        this.player?.seekTo?.(target, true);
        this.currentTime = target;
    }

    skip(delta: number) {
        this.seek(this.currentTime + delta);
    }

    toggle() {
        if (!this.player) return;
        if (this.isPaused) this.player.playVideo?.();
        else this.player.pauseVideo?.();
    }

    pause() {
        this.player?.pauseVideo?.();
    }

    setRate(rate: number) {
        this.player?.setPlaybackRate?.(rate);
    }

    // ── the pending clip ───────────────────────────────────────────────────

    markIn(at?: number) {
        this.pendingStart = Math.max(0, at ?? this.currentTime);
        if (this.pendingEnd !== null && this.pendingEnd <= this.pendingStart) {
            this.pendingEnd = null;
        }
    }

    markOut(at?: number) {
        const end = Math.max(0, at ?? this.currentTime);
        // Marking out with no in-point is a common slip: you hear the end of
        // the thing you wanted. Assume the few seconds before rather than
        // discarding the gesture.
        if (this.pendingStart === null) this.pendingStart = Math.max(0, end - 15);
        this.pendingEnd = Math.max(end, this.pendingStart + 1);
        this.pause();
    }

    get hasPending() {
        return this.pendingStart !== null;
    }

    get pendingIsComplete() {
        return this.pendingStart !== null && this.pendingEnd !== null;
    }

    clearPending() {
        this.pendingStart = null;
        this.pendingEnd = null;
    }

    /// Persists the watch position and tears the iframe down.
    ///
    /// The position is written with a targeted update rather than by saving a
    /// copy of the whole row: the old code held a stale `video` object for the
    /// life of playback and wrote all of it back on exit, reverting any title,
    /// folder, or trim change that arrived from a sync in the meantime.
    async destroy(latest?: Video) {
        if (this.ticker !== undefined) {
            clearInterval(this.ticker);
            this.ticker = undefined;
        }
        const position = Math.floor(this.currentTime);
        const video = latest ?? this.video;
        this.video = null;

        if (this.player?.destroy) {
            try {
                this.player.destroy();
            } catch (e) {
                console.error("Failed to destroy the YouTube player:", e);
            }
        }
        this.player = null;
        this.isReady = false;

        if (video?.id && position > 0) {
            try {
                await saveVideo({ ...video, last_position: position });
            } catch (e) {
                console.error("Failed to save the watch position:", e);
            }
        }
    }
}

export function formatClock(totalSeconds: number): string {
    const total = Math.max(0, Math.floor(totalSeconds || 0));
    const h = Math.floor(total / 3600);
    const m = Math.floor((total % 3600) / 60);
    const s = total % 60;
    return h > 0
        ? `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`
        : `${m}:${String(s).padStart(2, "0")}`;
}
