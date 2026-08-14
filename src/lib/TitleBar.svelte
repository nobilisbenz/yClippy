<script lang="ts">
    import { getCurrentWindow } from "@tauri-apps/api/window";
    import { platform } from "@tauri-apps/plugin-os";
    import { appState } from "./state.svelte";
    import Icon from "./Icon.svelte";

    let { onSearch }: { onSearch?: () => void } = $props();

    const appWindow = getCurrentWindow();
    const isAndroid = platform() === "android";

    function minimize() {
        appWindow.minimize().catch((e) => console.error(e));
    }

    function toggleMaximize() {
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

    const syncTint = $derived(
        appState.syncStatus === "error"
            ? "color: var(--danger)"
            : appState.syncStatus === "success"
              ? "color: var(--success)"
              : "",
    );
</script>

{#if !isAndroid}
    <div
        class="relative shrink-0 flex items-center gap-1 px-2 bg-[color:var(--bg)] border-b border-[color:var(--border)] select-none"
        style="height: var(--titlebar-h)"
    >
        <!-- The whole bar drags, except the controls drawn on top of it. -->
        <div
            role="presentation"
            onmousedown={startDrag}
            ondblclick={toggleMaximize}
            class="absolute inset-0 cursor-default"
        ></div>

        <!-- The breadcrumb is the title bar. You know which app you are in;
             where you are in the library is the useful thing to show, and it
             is the fastest way back up the tree. -->
        <nav
            class="relative flex items-center gap-0.5 min-w-0 flex-1 text-xs"
            aria-label="Breadcrumb"
        >
            <button
                class="px-1.5 py-1 rounded shrink-0 transition-colors hover:bg-[color:var(--surface-hi)] {trail.length ===
                    0 && !appState.activeVideo
                    ? 'text-[color:var(--text)]'
                    : 'text-[color:var(--text-faint)] hover:text-[color:var(--text)]'}"
                onclick={() => appState.openFolder([])}
            >
                Library
            </button>
            {#each trail as crumb, i (crumb.id)}
                <span class="text-[color:var(--text-faint)] shrink-0" aria-hidden="true">
                    <Icon name="chevronRight" size={11} />
                </span>
                <button
                    class="px-1.5 py-1 rounded truncate max-w-[180px] transition-colors hover:bg-[color:var(--surface-hi)] {i ===
                        trail.length - 1 && !appState.activeVideo
                        ? 'text-[color:var(--text)]'
                        : 'text-[color:var(--text-faint)] hover:text-[color:var(--text)]'}"
                    onclick={() => appState.openFolder(appState.selectionPath.slice(0, i + 1))}
                >
                    {crumb.name}
                </button>
            {/each}
            {#if appState.activeVideo}
                <span class="text-[color:var(--text-faint)] shrink-0" aria-hidden="true">
                    <Icon name="chevronRight" size={11} />
                </span>
                <span class="truncate max-w-[280px] text-[color:var(--text)] px-1.5">
                    {appState.activeVideo.title}
                </span>
            {/if}
        </nav>

        <div class="relative flex items-center gap-0.5 shrink-0">
            {#if appState.activeVideo}
                <!-- Under 1024px the clips panel is a drawer, and this is the
                     only way to reach it. -->
                <button
                    class="icon-btn lg:hidden"
                    onclick={() => (appState.isClipsSidebarOpen = !appState.isClipsSidebarOpen)}
                    title="Clips"
                    aria-label="Clips"
                >
                    <Icon name="scissors" size={15} />
                </button>
            {/if}

            <button class="icon-btn" onclick={onSearch} title="Search (Ctrl+K)" aria-label="Search">
                <Icon name="search" size={15} />
            </button>

            {#if appState.settings.githubTokenPresent && appState.settings.githubRepo}
                <button
                    class="icon-btn"
                    onclick={() => appState.triggerSync()}
                    disabled={appState.syncStatus === "syncing"}
                    title={appState.syncStatus === "error"
                        ? appState.syncError || "Sync failed"
                        : appState.syncStatus === "syncing"
                          ? "Syncing…"
                          : "Sync"}
                    aria-label="Sync"
                >
                    <span
                        style={syncTint}
                        class={appState.syncStatus === "syncing" ? "animate-spin" : ""}
                    >
                        <Icon name="sync" size={15} />
                    </span>
                </button>
            {/if}

            <button
                class="icon-btn"
                onclick={() => (appState.isSettingsModalOpen = true)}
                title="Settings"
                aria-label="Settings"
            >
                <Icon name="settings" size={15} />
            </button>

            <div class="w-px h-4 bg-[color:var(--border)] mx-1"></div>

            <button class="icon-btn" onclick={minimize} title="Minimize" aria-label="Minimize">
                <Icon name="minimize" size={15} />
            </button>
            <button class="icon-btn" onclick={toggleMaximize} title="Maximize" aria-label="Maximize">
                <Icon name="maximize" size={13} />
            </button>
            <button
                class="icon-btn hover:!bg-[color:var(--danger)] hover:!text-white"
                onclick={close}
                title="Close"
                aria-label="Close"
            >
                <Icon name="close" size={15} />
            </button>
        </div>
    </div>
{/if}
