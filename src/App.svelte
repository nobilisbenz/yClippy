<script lang="ts">
  import Dashboard from "./lib/Dashboard.svelte";
  import VideoPlayer from "./lib/VideoPlayer.svelte";
  import NativePlayer from "./lib/NativePlayer.svelte";
  import { appState } from "./lib/state.svelte";
  import AddVideoModal from "./lib/AddVideoModal.svelte";
  import SettingsModal from "./lib/SettingsModal.svelte";
  import EditVideoModal from "./lib/EditVideoModal.svelte";
  import ContextMenu from "./lib/ContextMenu.svelte";
  import SharedVideoDialog from "./lib/SharedVideoDialog.svelte";
  import TitleBar from "./lib/TitleBar.svelte";
  import { platform } from "@tauri-apps/plugin-os";
  import { onMount } from "svelte";

  const isAndroid = platform() === "android";

  let pendingSharedVideoId = $state<string | null>(null);

  onMount(() => {
    appState.initHistory();
    window.onpopstate = (e) => appState.handlePopState(e);

    (window as any).__yclippyOnSharedVideo = (videoId: string) => {
      pendingSharedVideoId = videoId;
    };

    const native = window as any;
    if (isAndroid && native.yClippyNative?.onAppReady) {
      native.yClippyNative.onAppReady();
    }
  });
</script>

<TitleBar />

<div
  class="flex w-full bg-black text-white font-sans overflow-hidden select-none"
  style="height: 100vh; padding-top: var(--safe-top); padding-bottom: var(--safe-bottom);"
>
  <main class="flex-1 overflow-hidden relative flex flex-col">
    {#if appState.activeVideo}
      {#if isAndroid}
        <NativePlayer video={appState.activeVideo} />
      {:else}
        <VideoPlayer video={appState.activeVideo} />
      {/if}
    {:else}
      <Dashboard />
    {/if}
  </main>

  {#if appState.isAddVideoModalOpen}
    <AddVideoModal folderId={appState.addVideoFolderId} />
  {/if}

  {#if appState.isSettingsModalOpen}
    <SettingsModal />
  {/if}

  {#if appState.isEditVideoModalOpen}
    <EditVideoModal />
  {/if}

  {#if appState.contextMenu.show}
    <ContextMenu
      x={appState.contextMenu.x}
      y={appState.contextMenu.y}
      items={appState.contextMenu.items}
      onClose={() => (appState.contextMenu.show = false)}
    />
  {/if}

  {#if pendingSharedVideoId}
    <SharedVideoDialog
      videoId={pendingSharedVideoId}
      onClose={() => (pendingSharedVideoId = null)}
    />
  {/if}
</div>
