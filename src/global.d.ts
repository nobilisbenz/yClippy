export { };

declare global {
    interface Window {
        onYouTubeIframeAPIReady: () => void;
        YT: any;
        yClippyNative?: {
            openInRevanced: (videoId: string, startSeconds?: number) => void;
            onAppReady: () => void;
        };
    }
}
