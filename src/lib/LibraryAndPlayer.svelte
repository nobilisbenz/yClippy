<script lang="ts">
    import Dashboard from "./Dashboard.svelte";
    import MobileDashboard from "./MobileDashboard.svelte";
    import VideoPlayer from "./VideoPlayer.svelte";
    import NativePlayer from "./NativePlayer.svelte";
    import ClipList from "./ClipList.svelte";
    import ContinueWatching from "./ContinueWatching.svelte";
    import { appState } from "./state.svelte";
    import { platform } from "@tauri-apps/plugin-os";

    const isAndroid = platform() === "android";
    let seekTo = $state<(t: number) => void>(() => {});
</script>

<div class="flex w-full h-full min-h-0">
    {#if !isAndroid}
        <aside
            class="hidden md:flex flex-col shrink-0 border-r border-[color:var(--border)] bg-[color:var(--surface)]"
            style="width: 320px; min-width: 280px;"
        >
            <div class="flex-1 min-h-0 overflow-hidden">
                <Dashboard compact />
            </div>
        </aside>
    {/if}

    <main class="flex-1 min-w-0 flex flex-col min-h-0 bg-black">
        {#if isAndroid}
            {#if appState.activeVideo}
                {#key appState.activeVideo.id}
                    <NativePlayer
                        video={appState.activeVideo}
                        seekToTime={appState.seekToTime}
                        onSeekConsumed={() => appState.consumeSeek()}
                        bind:seekTo
                    />
                {/key}
            {:else}
                <MobileDashboard />
            {/if}
        {:else if appState.activeVideo}
            {#key appState.activeVideo.id}
                <VideoPlayer
                    video={appState.activeVideo}
                    seekToTime={appState.seekToTime}
                    onSeekConsumed={() => appState.consumeSeek()}
                    bind:seekTo
                />
            {/key}
        {:else}
            <!-- The rail is the only library on desktop; a second Dashboard here
                 gave two trees with independent selections. -->
            <div class="md:hidden flex-1 min-h-0">
                <Dashboard />
            </div>
            <div class="hidden md:flex flex-1 min-h-0">
                <ContinueWatching />
            </div>
        {/if}
    </main>

    {#if appState.activeVideo && !isAndroid}
        <aside
            class="hidden lg:flex flex-col shrink-0 border-l border-[color:var(--border)] bg-[color:var(--surface)]"
            style="width: 320px; min-width: 280px;"
        >
            <div class="p-4 border-b border-[color:var(--border)] flex items-center justify-between">
                <span class="font-bold">Clips</span>
                <span class="text-xs text-[color:var(--text-faint)]">
                    {appState.activeClips.length}
                </span>
            </div>
            <div class="flex-1 min-h-0 overflow-y-auto">
                <ClipList
                    videoId={appState.activeVideo.id}
                    seekTo={(t) => seekTo(t)}
                />
            </div>
        </aside>
    {/if}
</div>