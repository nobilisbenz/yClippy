<script lang="ts">
    import ClipList from "./ClipList.svelte";
    import ContinueWatching from "./ContinueWatching.svelte";
    import Dashboard from "./Dashboard.svelte";
    import Icon from "./Icon.svelte";
    import Library from "./Library.svelte";
    import NativePlayer from "./NativePlayer.svelte";
    import VideoPlayer from "./VideoPlayer.svelte";
    import { appState } from "./state.svelte";
    import { platform } from "@tauri-apps/plugin-os";

    /// The shell decides *which* surfaces exist; the surfaces decide how they
    /// look. Three arrangements, one tree:
    ///
    ///   touch / narrow   drill-down list, or a full-bleed player
    ///   ≥ 768px          library rail + player (+ clips in a drawer)
    ///   ≥ 1024px         library rail + player + docked clips panel
    ///
    /// Opening a video no longer replaces the library: the rail keeps your
    /// place in the tree, which was the single biggest structural annoyance.
    const isAndroid = platform() === "android";

    let width = $state(typeof window === "undefined" ? 1280 : window.innerWidth);
    const touchLayout = $derived(isAndroid || width < 768);
    const clipsDocked = $derived(width >= 1024);

    let seekTo = $state<(t: number) => void>(() => {});

    $effect(() => {
        // A docked panel and a drawer are the same panel; closing the drawer
        // state when it docks stops the toggle from being stuck "open".
        if (clipsDocked && appState.isClipsSidebarOpen) {
            appState.isClipsSidebarOpen = false;
        }
    });
</script>

<svelte:window bind:innerWidth={width} />

<div class="flex w-full h-full min-h-0">
    {#if touchLayout}
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
            <div class="flex-1 min-w-0">
                <Library density="touch" />
            </div>
        {/if}
    {:else if appState.activeVideo}
        <aside
            class="flex flex-col shrink-0 border-r border-[color:var(--border)]"
            style="width: 300px"
        >
            <Library density="compact" />
        </aside>

        <main class="flex-1 min-w-0 flex flex-col min-h-0 bg-black">
            {#key appState.activeVideo.id}
                <VideoPlayer
                    video={appState.activeVideo}
                    seekToTime={appState.seekToTime}
                    onSeekConsumed={() => appState.consumeSeek()}
                    bind:seekTo
                />
            {/key}
        </main>

        {#if clipsDocked}
            <aside
                class="flex flex-col shrink-0 border-l border-[color:var(--border)] bg-[color:var(--surface)]"
                style="width: 320px"
            >
                <div class="panel-head">
                    <span class="section-label flex-1">Clips</span>
                    <span class="chip">{appState.activeClips.length}</span>
                </div>
                <div class="flex-1 min-h-0">
                    <ClipList videoId={appState.activeVideo.id} seekTo={(t) => seekTo(t)} />
                </div>
            </aside>
        {:else if appState.isClipsSidebarOpen}
            <!-- Under 1024px the panel becomes a drawer rather than
                 disappearing: the clips are the product, not a detail. -->
            <div class="overlay z-[150] justify-end" role="presentation">
                <button
                    class="flex-1 cursor-default"
                    aria-label="Close clips"
                    onclick={() => (appState.isClipsSidebarOpen = false)}
                ></button>
                <aside
                    class="w-[340px] max-w-[85vw] flex flex-col bg-[color:var(--surface)] border-l border-[color:var(--border-hi)] shadow-2xl"
                >
                    <div class="panel-head">
                        <span class="section-label flex-1">Clips</span>
                        <span class="chip">{appState.activeClips.length}</span>
                        <button
                            class="icon-btn"
                            onclick={() => (appState.isClipsSidebarOpen = false)}
                            aria-label="Close clips"><Icon name="close" size={15} /></button
                        >
                    </div>
                    <div class="flex-1 min-h-0">
                        <ClipList videoId={appState.activeVideo.id} seekTo={(t) => seekTo(t)} />
                    </div>
                </aside>
            </div>
        {/if}
    {:else}
        <!-- Nothing playing: the columns get the whole window, which is the
             one place the Miller layout actually pays for itself. -->
        <main class="flex-1 min-w-0 flex flex-col min-h-0 bg-[color:var(--bg)]">
            <ContinueWatching variant="strip" />
            <div class="flex-1 min-h-0">
                <Dashboard />
            </div>
        </main>
    {/if}
</div>
