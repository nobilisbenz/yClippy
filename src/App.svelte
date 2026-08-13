<script lang="ts">
  import LibraryAndPlayer from "./lib/LibraryAndPlayer.svelte";
  import { appState } from "./lib/state.svelte";
  import AddVideoModal from "./lib/AddVideoModal.svelte";
  import SettingsModal from "./lib/SettingsModal.svelte";
  import EditVideoModal from "./lib/EditVideoModal.svelte";
  import ContextMenu from "./lib/ContextMenu.svelte";
  import SharedVideoDialog from "./lib/SharedVideoDialog.svelte";
  import TitleBar from "./lib/TitleBar.svelte";
  import Toast from "./lib/Toast.svelte";
  import UndoToast from "./lib/UndoToast.svelte";
  import CommandPalette from "./lib/CommandPalette.svelte";
  import { platform } from "@tauri-apps/plugin-os";
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { takePendingPlay } from "./lib/db";

  const isAndroid = platform() === "android";

  let pendingSharedVideoId = $state<string | null>(null);
  let isCommandPaletteOpen = $state(false);

  onMount(() => {
    appState.initHistory();
    window.onpopstate = (e) => appState.handlePopState(e);

    (window as any).__yclippyOnSharedVideo = (videoId: string) => {
      pendingSharedVideoId = videoId;
    };

    // A `yclippy://play?v=…&t=…` deep link, or a YouTube link opened with
    // yClippy. Known videos play at the timestamp; unknown ones offer to import.
    (window as any).__yclippyOnPlay = (p: { videoId: string; startSeconds: number }) => {
      play(p.videoId, p.startSeconds);
    };

    const native = window as any;
    if (isAndroid && native.yClippyNative?.onAppReady) {
      native.yClippyNative.onAppReady();
    }

    window.addEventListener("keydown", (e) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        isCommandPaletteOpen = !isCommandPaletteOpen;
      }
    });

    async function play(videoId: string, atSeconds: number | null | undefined) {
      const at = atSeconds && atSeconds > 0 ? atSeconds : undefined;
      await appState.refreshVideos();
      const ok = await appState.playVideoById(videoId, at);
      if (!ok) {
        pendingSharedVideoId = videoId;
      }
    }

    listen<{ video_id: string; at_seconds: number }>("yclippy://play", async (event) => {
      const payload = event.payload;
      if (!payload?.video_id) return;
      // Claim the slot so the drain below cannot replay the same request.
      await takePendingPlay().catch(() => null);
      await play(payload.video_id, payload.at_seconds);
    }).catch((e) => console.error("Failed to subscribe to play event:", e));

    listen("yclippy://library-changed", () => {
      appState.refreshAll().catch((e) => console.error("Refresh failed:", e));
    }).catch((e) => console.error("Failed to subscribe to library event:", e));

    // A `yclippy play` on a cold start emits before this webview exists, so the
    // request is parked in Rust and collected here instead of being lost.
    takePendingPlay()
      .then((req) => {
        if (req?.video_id) play(req.video_id, req.at_seconds);
      })
      .catch((e) => console.error("Failed to drain pending play:", e));
  });
</script>

<TitleBar />

<div
  class="flex w-full bg-black text-white font-sans overflow-hidden select-none"
  style="height: 100dvh; padding-top: var(--safe-top); padding-bottom: var(--safe-bottom); padding-left: var(--safe-left); padding-right: var(--safe-right);"
>
  <LibraryAndPlayer />

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

{#if appState.toast}
  <Toast
    key={appState.toast.id}
    message={appState.toast.message}
    kind={appState.toast.kind}
    onClose={() => (appState.toast = null)}
  />
{/if}

{#if appState.undoable}
  <UndoToast
    message={appState.undoable.message}
    onUndo={() => appState.performUndo()}
    onDismiss={() => appState.dismissUndo()}
  />
{/if}

{#if isCommandPaletteOpen}
  <CommandPalette onClose={() => (isCommandPaletteOpen = false)} />
{/if}
