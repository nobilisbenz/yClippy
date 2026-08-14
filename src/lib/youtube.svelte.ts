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
    /// 0–1. Drawn behind the playhead so a stall reads as "still buffering"
    /// rather than "frozen".
    loaded = $state(0);
    /// True whenever the video is not actively playing — including ended,
    /// which is the case the two old players disagreed about.
    isPaused = $state(true);
    isReady = $state(false);
    hasEnded = $state(false);
    /// Set when the stage cannot show the video — the API script failed, or
    /// YouTube refused the embed. Without this a failure is a black rectangle
    /// with working buttons, which is indistinguishable from a video that has
    /// not started.
    error = $state<string | null>(null);

    /// The pending clip, before it is named and saved. Keeping it as an object
    /// rather than two loose numbers is what lets the track draw it and lets
    /// you adjust either edge before committing.
    pendingStart = $state<number | null>(null);
    pendingEnd = $state<number | null>(null);

    private containerId = `yt-${Math.random().toString(36).slice(2, 9)}`;
    private ticker: number | undefined;
    private video: Video | null = null;
    private lastStartAt: number | undefined;

    get elementId() {
        return this.containerId;
    }

    /// The privacy-enhanced host serves the *player*, but it does not serve
    /// the loader: `youtube-nocookie.com/iframe_api` is a 404 HTML page, which
    /// a `nosniff` webview refuses to execute, so `window.YT` never appears
    /// and every mount hangs on a black stage. The script comes from
    /// youtube.com; `host` below still puts the iframe itself on nocookie.
    private static readonly API_SRC = "https://www.youtube.com/iframe_api";
    private static readonly PLAYER_HOST = "https://www.youtube-nocookie.com";
    private static readonly API_TIMEOUT_MS = 15_000;

    /// Loads the iframe API once per document, and never leaves a global
    /// callback pointing at a destroyed component.
    private static apiReady: Promise<void> | null = null;
    private static loadApi(): Promise<void> {
        if (typeof window !== "undefined" && (window as any).YT?.Player) {
            return Promise.resolve();
        }
        if (YouTubeController.apiReady) return YouTubeController.apiReady;

        YouTubeController.apiReady = new Promise<void>((resolve, reject) => {
            let settled = false;
            // A rejected load must not be cached, or the retry button would
            // replay the same failure without ever asking the network again.
            const fail = (reason: string) => {
                if (settled) return;
                settled = true;
                YouTubeController.apiReady = null;
                document.querySelector("script[data-yt-api]")?.remove();
                reject(new Error(reason));
            };
            const done = () => {
                if (settled) return;
                settled = true;
                clearTimeout(timer);
                resolve();
            };

            const timer = setTimeout(
                () => fail("Timed out loading the YouTube player."),
                YouTubeController.API_TIMEOUT_MS,
            );

            const previous = window.onYouTubeIframeAPIReady;
            window.onYouTubeIframeAPIReady = () => {
                previous?.();
                done();
            };

            if (!document.querySelector("script[data-yt-api]")) {
                const tag = document.createElement("script");
                tag.src = YouTubeController.API_SRC;
                tag.async = true;
                tag.setAttribute("data-yt-api", "true");
                tag.onerror = () => {
                    clearTimeout(timer);
                    fail("Could not reach YouTube. Check your connection.");
                };
                document.head.appendChild(tag);
            }
        });
        return YouTubeController.apiReady;
    }

    /// Whether this webview can decode what YouTube serves.
    ///
    /// On Linux the webview is WebKitGTK, which decodes through GStreamer, and
    /// a stock Ubuntu install ships neither H.264 nor AAC. YouTube then hands
    /// back its own "your browser can't play this video" card *inside* the
    /// iframe, where the API reports nothing and the app looks broken. Asking
    /// MediaSource up front turns that into a sentence naming the fix.
    static codecSupport(): { ok: boolean; video: boolean; audio: boolean } {
        const MS = typeof window === "undefined" ? null : (window as any).MediaSource;
        if (!MS?.isTypeSupported) return { ok: true, video: true, audio: true };
        const can = (type: string) => {
            try {
                return MS.isTypeSupported(type);
            } catch {
                return false;
            }
        };
        const video =
            can('video/mp4; codecs="avc1.42E01E"') || can('video/webm; codecs="vp9"');
        const audio = can('audio/mp4; codecs="mp4a.40.2"') || can('audio/webm; codecs="opus"');
        return { ok: video && audio, video, audio };
    }

    /// YouTube reports refusals as numbers. Say what they mean, and offer the
    /// only thing that actually works for an un-embeddable video: open it out.
    private static describeError(code: number): string {
        switch (code) {
            case 2:
                return "YouTube rejected this video id.";
            case 5:
                return "This video cannot be played in an embedded player.";
            case 100:
                return "This video is private or has been removed.";
            case 101:
            case 150:
                return "The owner does not allow this video to be embedded.";
            default:
                return "YouTube could not play this video.";
        }
    }

    async mount(video: Video, startAt?: number) {
        this.video = video;
        this.error = null;
        this.lastStartAt = startAt;

        try {
            await YouTubeController.loadApi();
        } catch (e) {
            if (this.video !== video) return;
            this.error = e instanceof Error ? e.message : String(e);
            return;
        }
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
            enablejsapi: 1,
        };
        // `origin` tells YouTube which document to post messages back to. The
        // bundled app is served from a custom scheme, and handing YouTube a
        // `tauri://` origin it cannot honour breaks the handshake it is meant
        // to secure — so it is sent only when the page is genuinely on http(s),
        // which is the dev server.
        const origin = typeof window !== "undefined" ? window.location.origin : "";
        if (/^https?:$/.test(window.location.protocol)) playerVars.origin = origin;
        if (video.end_time > 0) playerVars.end = video.end_time;

        this.player = new (window as any).YT.Player(this.containerId, {
            host: YouTubeController.PLAYER_HOST,
            height: "100%",
            width: "100%",
            videoId: video.id,
            playerVars,
            events: {
                onReady: (event: any) => {
                    this.isReady = true;
                    this.error = null;
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
                onError: (event: any) => {
                    this.error = YouTubeController.describeError(Number(event?.data));
                },
            },
        });

        this.ticker = setInterval(() => this.sample(), 500) as unknown as number;
    }

    /// Tears the current attempt down and mounts again from scratch. The
    /// watch position is not written here: a failed mount never had one.
    async retry() {
        const video = this.video;
        if (!video) return;
        if (this.ticker !== undefined) {
            clearInterval(this.ticker);
            this.ticker = undefined;
        }
        try {
            this.player?.destroy?.();
        } catch {
            // A player that failed to build has nothing to tear down.
        }
        this.player = null;
        this.isReady = false;
        this.error = null;
        await this.mount(video, this.lastStartAt);
    }

    get watchUrl(): string {
        if (!this.video) return "https://www.youtube.com";
        const at = Math.floor(this.currentTime);
        return `https://www.youtube.com/watch?v=${this.video.id}${at > 0 ? `&t=${at}s` : ""}`;
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
        if (typeof p.getVideoLoadedFraction === "function") {
            const f = p.getVideoLoadedFraction();
            if (typeof f === "number" && !Number.isNaN(f)) this.loaded = f;
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
        this.error = null;

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
