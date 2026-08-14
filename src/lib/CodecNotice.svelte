<script lang="ts">
    import { writeText } from "@tauri-apps/plugin-clipboard-manager";
    import { appState } from "./state.svelte";
    import { YouTubeController } from "./youtube.svelte";
    import Icon from "./Icon.svelte";

    /// The webview cannot decode YouTube's streams.
    ///
    /// This is not a bug in the app and there is nothing the app can do about
    /// it — but "your browser can't play this video", in whatever language the
    /// player happens to be in, is not a diagnosis. The fix is one apt line,
    /// so the notice carries it.
    const support = YouTubeController.codecSupport();
    const DISMISS_KEY = "codec_notice_dismissed";
    const COMMAND =
        "sudo apt install gstreamer1.0-libav gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly";

    let dismissed = $state(
        typeof localStorage !== "undefined" && localStorage.getItem(DISMISS_KEY) === "1",
    );

    const missing = $derived(
        [!support.video && "video (H.264 / VP9)", !support.audio && "audio (AAC / Opus)"]
            .filter(Boolean)
            .join(" and "),
    );

    async function copyCommand() {
        try {
            await writeText(COMMAND);
            appState.showToast("Command copied", "success");
        } catch (e) {
            appState.showToast(`Clipboard error: ${String(e)}`, "error");
        }
    }

    function dismiss() {
        dismissed = true;
        try {
            localStorage.setItem(DISMISS_KEY, "1");
        } catch {
            // Private storage can be unavailable; dismissing for this session
            // is still worth doing.
        }
    }
</script>

{#if !support.ok && !dismissed}
    <div
        class="dialog rounded-[8px] p-3 flex flex-col gap-2 max-w-md"
        style="border-color: var(--danger)"
    >
        <div class="flex items-start gap-2">
            <span class="shrink-0 mt-0.5" style="color: var(--danger)">
                <Icon name="alert" size={16} />
            </span>
            <div class="min-w-0">
                <p class="text-[13px] text-[color:var(--text)]">
                    This webview has no {missing} decoder, so YouTube refuses to play.
                </p>
                <p class="text-[11px] text-[color:var(--text-faint)] mt-1">
                    Install the GStreamer codecs, then restart yClippy.
                </p>
            </div>
            <button class="icon-btn shrink-0 -mt-0.5" onclick={dismiss} aria-label="Dismiss">
                <Icon name="close" size={14} />
            </button>
        </div>
        <div class="flex items-center gap-2">
            <code
                class="flex-1 min-w-0 truncate text-[11px] font-mono px-2 py-1.5 rounded bg-[color:var(--bg)] border border-[color:var(--border)] text-[color:var(--text-dim)]"
                title={COMMAND}
            >
                {COMMAND}
            </code>
            <button class="btn shrink-0" onclick={copyCommand}>
                <Icon name="copy" size={13} />
                Copy
            </button>
        </div>
    </div>
{/if}
