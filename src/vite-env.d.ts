/// <reference types="svelte" />
/// <reference types="vite/client" />

declare global {
    interface Window {
        YT: any;
        onYouTubeIframeAPIReady: any;
    }
}
