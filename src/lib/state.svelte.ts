import { getVideos, getFolders, type Video, type Folder, type Clip, getClips } from './db';

class AppState {
    videos = $state<Video[]>([]);
    folders = $state<Folder[]>([]);
    activeVideo = $state<Video | null>(null);
    activeClips = $state<Clip[]>([]);

    // UI State
    isSidebarOpen = $state(false);
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
        githubToken: "",
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

    constructor() {
        console.log("[AppState] Initializing...");
        this.refreshVideos().catch(e => console.error("[AppState] Failed to refresh videos:", e));
        this.refreshFolders().catch(e => console.error("[AppState] Failed to refresh folders:", e));
        this.loadSettings();

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

            // Sync Path
            this.selectionPath = path || [];

            // Sync Video
            if (view === 'video' && videoId) {
                // Find video object (might need to wait for videos to load if empty?)
                const vid = this.videos.find(v => v.id === videoId);
                if (vid) {
                    this.setActiveVideo(vid);
                }
            } else {
                this.setActiveVideo(null);
            }
        } else {
            // Default/Fallback
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

    async triggerSync() {
        if (!this.settings.githubToken || !this.settings.githubRepo) {
            // throw new Error("GitHub Token and Repo URL are required");
            // Silent return if not configured
            return;
        }

        this.syncStatus = "syncing";
        this.syncError = null;

        try {
            await import("@tauri-apps/api/core").then(async ({ invoke }) => {
                await invoke("start_github_sync", {
                    token: this.settings.githubToken,
                    repoUrl: this.settings.githubRepo,
                });
                await this.refreshAll();
                this.updateSettings({ lastSyncAt: Date.now() });
                this.syncStatus = "success";
                setTimeout(() => {
                    if (this.syncStatus === "success") this.syncStatus = "idle";
                }, 3000);
            });
        } catch (e: any) {
            console.error(e);
            this.syncStatus = "error";
            this.syncError = e.toString();
        }
    }
}

export const appState = new AppState();
