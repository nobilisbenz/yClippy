import { getVideos, getFolders, type Video, type Folder, type Clip, getClips } from './db';

class AppState {
    videos = $state<Video[]>([]);
    folders = $state<Folder[]>([]);
    activeVideo = $state<Video | null>(null);
    activeClips = $state<Clip[]>([]);
    videosLoadingPromise: Promise<void> | null = null;
    seekToTime = $state<number | undefined>(undefined);

    // UI State
    isClipsSidebarOpen = $state(false);
    isClipModalOpen = $state(false);
    isAddVideoModalOpen = $state(false);
    addVideoFolderId = $state<number | null>(null);
    isSettingsModalOpen = $state(false);
    isEditVideoModalOpen = $state(false);
    videoToEdit = $state<Video | null>(null);

    selectionPath = $state<number[]>([]); // Persist navigation path

    settings = $state({
        clipboardTemplate: `<iframe src="https://www.youtube.com/embed/{id}?start={start}&end={end}" height="360" width="100%" seamless="seamless" frameborder="0" allowfullscreen></iframe>`,
        githubTokenPresent: false,
        githubRepo: "",
        lastSyncAt: null as number | null,
    });

    contextMenu = $state<{
        x: number;
        y: number;
        items: { label: string; action: () => void; danger?: boolean }[];
        show: boolean;
    }>({
        x: 0,
        y: 0,
        items: [],
        show: false,
    });

    syncStatus = $state<"idle" | "syncing" | "success" | "error">("idle");
    syncError = $state<string | null>(null);

    toast = $state<{ id: number; message: string; kind: "info" | "success" | "error" } | null>(null);

    showToast(message: string, kind: "info" | "success" | "error" = "info") {
        this.toast = { id: Date.now(), message, kind };
    }

    undoable = $state<{
        id: number;
        message: string;
        restore: () => Promise<void>;
        expiresAt: number;
    } | null>(null);

    showUndo(message: string, restore: () => Promise<void>, durationMs = 5000) {
        this.undoable = {
            id: Date.now(),
            message,
            restore,
            expiresAt: Date.now() + durationMs,
        };
    }

    async performUndo() {
        const u = this.undoable;
        if (!u) return;
        this.undoable = null;
        try {
            await u.restore();
            this.showToast("Restored", "success");
        } catch (e) {
            console.error("Undo failed:", e);
            this.showToast(`Undo failed: ${String(e)}`, "error");
        }
    }

    dismissUndo() {
        this.undoable = null;
    }

    constructor() {
        console.log("[AppState] Initializing...");
        this.refreshVideos().catch(e => console.error("[AppState] Failed to refresh videos:", e));
        this.refreshFolders().catch(e => console.error("[AppState] Failed to refresh folders:", e));
        this.loadSettings();
        this.refreshGithubConfig().catch((e) =>
            console.error("[AppState] Failed to refresh GitHub config:", e),
        );

        console.log("[AppState] Initialization complete");
    }

    loadSettings() {
        const saved = localStorage.getItem("app_settings");
        if (saved) {
            try {
                this.settings = { ...this.settings, ...JSON.parse(saved) };
            } catch (e) {
                console.error("Failed to load settings", e);
            }
        }
    }

    updateSettings(newSettings: Partial<typeof this.settings>) {
        this.settings = { ...this.settings, ...newSettings };
        localStorage.setItem("app_settings", JSON.stringify(this.settings));
    }

    async refreshVideos() {
        this.videos = await getVideos();
    }

    async refreshFolders() {
        this.folders = await getFolders();
    }

    async setActiveVideo(video: Video | null) {
        this.activeVideo = video;
        if (video && video.id !== undefined) {
            this.activeClips = await getClips(video.id);
        } else {
            this.activeClips = [];
        }
    }

    async refreshActiveClips() {
        if (this.activeVideo && this.activeVideo.id !== undefined) {
            this.activeClips = await getClips(this.activeVideo.id);
        }
    }
    async refreshAll() {
        await Promise.all([this.refreshFolders(), this.refreshVideos(), this.refreshActiveClips()]);
    }

    // Navigation & History
    initHistory() {
        // Initial state
        history.replaceState({ view: 'root', path: [] }, '');
    }

    handlePopState(event: PopStateEvent) {
        if (event.state) {
            const { view, path, videoId } = event.state;

            this.selectionPath = path || [];

            if (view === 'video' && videoId) {
                const vid = this.videos.find(v => v.id === videoId);
                if (vid) {
                    this.setActiveVideo(vid);
                } else if (this.videos.length === 0) {
                    this.videosLoadingPromise = this.refreshVideos().then(() => {
                        const found = this.videos.find(v => v.id === videoId);
                        if (found) {
                            this.setActiveVideo(found);
                        } else {
                            this.setActiveVideo(null);
                            history.replaceState({ view: 'folder', path: path || [] }, '');
                        }
                    }).catch((e) => {
                        console.error("PopState failed to load videos:", e);
                        this.setActiveVideo(null);
                    });
                } else {
                    this.setActiveVideo(null);
                }
            } else {
                this.setActiveVideo(null);
            }
        } else {
            this.selectionPath = [];
            this.setActiveVideo(null);
        }
    }

    openFolder(path: number[]) {
        this.selectionPath = path;
        history.pushState({ view: 'folder', path }, '');
    }

    openVideo(video: Video) {
        // Ensure we preserve the current path in the history state
        history.pushState({
            view: 'video',
            path: $state.snapshot(this.selectionPath),
            videoId: video.id
        }, '');
        this.setActiveVideo(video);
    }

    goBack() {
        history.back();
    }

    async triggerSync(): Promise<{ success: boolean; error?: string }> {
        if (!this.settings.githubTokenPresent || !this.settings.githubRepo) {
            const message = "GitHub Token and Repo URL are required";
            this.syncStatus = "error";
            this.syncError = message;
            return { success: false, error: message };
        }

        this.syncStatus = "syncing";
        this.syncError = null;

        try {
            const { invoke } = await import("@tauri-apps/api/core");
            await invoke("start_github_sync");
            await this.refreshAll();
            this.updateSettings({ lastSyncAt: Date.now() });
            this.syncStatus = "success";
            setTimeout(() => {
                if (this.syncStatus === "success") this.syncStatus = "idle";
            }, 3000);
            return { success: true };
        } catch (e: any) {
            console.error(e);
            const errorStr = e.toString ? e.toString() : String(e);
            this.syncStatus = "error";
            this.syncError = errorStr;
            return { success: false, error: errorStr };
        }
    }

    async refreshGithubConfig(): Promise<void> {
        try {
            const { invoke } = await import("@tauri-apps/api/core");
            const cfg = await invoke<{ github_repo: string; token_present: boolean }>(
                "get_github_config",
            );
            this.updateSettings({
                githubRepo: cfg.github_repo || "",
                githubTokenPresent: cfg.token_present,
            });
        } catch (e) {
            console.error("Failed to load GitHub config:", e);
        }
    }

    async playVideoById(videoId: string, atSeconds?: number) {
        const found = this.videos.find((v) => v.id === videoId);
        if (found) {
            if (atSeconds !== undefined) {
                this.seekToTime = atSeconds;
            }
            this.openVideo(found);
            return true;
        }
        return false;
    }
}

export const appState = new AppState();
