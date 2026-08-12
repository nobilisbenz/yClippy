export { };

declare global {
    interface Window {
        onYouTubeIframeAPIReady: () => void;
        YT: any;
        yClippyNative?: {
            startAudioService: () => void;
            stopAudioService: () => void;
            keepScreenOn: () => void;
            releaseScreenOn: () => void;
            showYouTubePlayer: (videoId: string, startPosition: number) => void;
            showYouTubePlayerByUrl: (videoUrl: string, startPosition: number) => void;
            hideYouTubePlayer: () => void;
            playYouTubeVideo: () => void;
            pauseYouTubeVideo: () => void;
            seekYouTubeVideo: (seconds: number) => void;
            getYouTubePosition: () => number;
        };
    }
}
