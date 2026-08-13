<script lang="ts">
    import { getCurrentWindow } from "@tauri-apps/api/window";
    import { platform } from "@tauri-apps/plugin-os";
    import { appState } from "./state.svelte";

    const appWindow = getCurrentWindow();
    const isAndroid = platform() === "android";

    function minimize() {
        appWindow.minimize().catch((e) => console.error(e));
    }

    async function toggleMaximize() {
        appWindow.toggleMaximize().catch((e) => console.error(e));
    }

    function close() {
        appWindow.close().catch((e) => console.error(e));
    }

    function startDrag() {
        appWindow.startDragging().catch((e) => console.error(e));
    }

    const trail = $derived(
        appState.selectionPath
            .map((id) => appState.folders.find((f) => f.id === id))
            .filter((f): f is NonNullable<typeof f> => f !== undefined)
            .map((f) => ({ id: f.id!, name: f.name })),
    );
</script>

<div
    class="peer fixed top-0 left-0 right-0 h-3 z-[60] hover:bg-transparent"
></div>

{#if !isAndroid}
<div
    class="peer fixed top-0 left-0 right-0 h-10 bg-black border-b border-zinc-900 border-opacity-50 flex items-center px-4 z-[100] transition-transform duration-300 ease-out shadow-lg group"
>
    <div
        role="button"
        tabindex="-1"
        onmousedown={startDrag}
        class="absolute inset-0 w-full h-full z-0 cursor-default"
        aria-hidden="true"
    ></div>

    <div
        class="relative z-10 flex items-center w-full pointer-events-none justify-between"
    >
        <!-- The breadcrumb is the title bar. You know which app you are in;
             where you are in the library is the useful thing to show, and it
             is the fastest way back up the tree. -->
        <nav
            class="flex items-center gap-1 min-w-0 pointer-events-auto text-xs"
            aria-label="Breadcrumb"
        >
            <button
                class="px-1.5 py-0.5 rounded shrink-0 transition-colors {trail.length === 0
                    ? 'text-white/90'
                    : 'text-zinc-500 hover:text-white'}"
                onclick={() => appState.openFolder([])}
            >
                Library
            </button>
            {#each trail as crumb, i (crumb.id)}
                <span class="text-zinc-700 shrink-0" aria-hidden="true">›</span>
                <button
                    class="px-1.5 py-0.5 rounded truncate max-w-[180px] transition-colors {i ===
                    trail.length - 1
                        ? 'text-white/90'
                        : 'text-zinc-500 hover:text-white'}"
                    onclick={() =>
                        appState.openFolder(appState.selectionPath.slice(0, i + 1))}
                >
                    {crumb.name}
                </button>
            {/each}
            {#if appState.activeVideo}
                <span class="text-zinc-700 shrink-0" aria-hidden="true">›</span>
                <span class="truncate max-w-[240px] text-white/90 px-1.5"
                    >{appState.activeVideo.title}</span
                >
            {/if}
        </nav>

        <div class="flex items-center gap-1 pointer-events-auto">
            <button
                onclick={minimize}
                class="p-2 hover:bg-zinc-800/80 rounded active:bg-zinc-700 text-zinc-400 hover:text-white transition-colors cursor-pointer flex items-center justify-center"
                title="Minimize"
                type="button"
            >
                <svg
                    class="size-4 pointer-events-none"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                >
                    <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M18 12H6"
                    />
                </svg>
            </button>
            <button
                onclick={toggleMaximize}
                class="p-2 hover:bg-zinc-800/80 rounded active:bg-zinc-700 text-zinc-400 hover:text-white transition-colors cursor-pointer flex items-center justify-center"
                title="Maximize"
                type="button"
            >
                <svg
                    class="size-4 pointer-events-none"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                >
                    <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M4 8V4m0 0h4M4 4l5 5m11-1V4m0 0h-4m4 0l-5 5M4 16v4m0 0h4m-4 0l5-5m11 5l-5-5m5 5v-4m0 4h-4"
                    />
                </svg>
            </button>
            <button
                onclick={close}
                class="p-2 hover:bg-red-600 rounded active:bg-red-700 text-zinc-400 hover:text-white transition-colors cursor-pointer flex items-center justify-center"
                title="Close"
                type="button"
            >
                <svg
                    class="size-4 pointer-events-none"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                >
                    <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M6 18L18 6M6 6l12 12"
                    />
                </svg>
            </button>
        </div>
    </div>
</div>
{/if}
