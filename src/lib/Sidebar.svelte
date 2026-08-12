<script lang="ts">
    import { appState } from "./state.svelte";

    // Simple navigation items
    const items = [
        {
            label: "Library",
            icon: "M4 6h16M4 12h16M4 18h16",
            action: () => {
                appState.activeVideo = null;
            },
        },
        // {
        //     label: "Playlists",
        //     icon: "M4 6h16M4 10h16M4 14h16M4 18h16",
        //     action: () => {
        //         /* TODO */
        //     },
        // },
        {
            label: "Settings",
            icon: "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z M15 12a3 3 0 11-6 0 3 3 0 016 0z",
            action: () => {
                appState.isSettingsModalOpen = true;
            },
        },
    ];
</script>

<aside
    class="{appState.isSidebarOpen
        ? 'w-64'
        : 'w-16'} h-full bg-zinc-950 border-r border-zinc-900 flex flex-col transition-all duration-300"
>
    <div
        class="h-16 flex items-center justify-between px-4 border-b border-zinc-900"
    >
        {#if appState.isSidebarOpen}
            <span class="text-xl font-bold tracking-tighter text-white"
                >Clipper</span
            >
        {/if}
        <button
            onclick={() => (appState.isSidebarOpen = !appState.isSidebarOpen)}
            class="p-1 hover:bg-zinc-800 rounded"
            aria-label="Toggle Sidebar"
        >
            <svg
                class="size-6 text-zinc-400"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
            >
                <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M4 6h16M4 12h16M4 18h16"
                />
            </svg>
        </button>
    </div>

    <nav class="flex-1 py-4 flex flex-col gap-2">
        {#each items as item}
            <button
                onclick={item.action}
                class="flex items-center gap-3 px-3 py-3 mx-2 rounded-lg text-zinc-400 hover:text-white hover:bg-zinc-900 transition-colors group {appState.isSidebarOpen
                    ? ''
                    : 'justify-center'}"
                title={item.label}
            >
                <svg
                    class="size-6 shrink-0"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                >
                    <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d={item.icon}
                    />
                </svg>
                {#if appState.isSidebarOpen}
                    <span class="font-medium">{item.label}</span>
                {/if}
            </button>
        {/each}
    </nav>

    <div class="p-4 border-t border-zinc-900">
        <button
            onclick={() => {
                appState.addVideoFolderId = appState.selectionPath.length > 0 
                    ? appState.selectionPath[appState.selectionPath.length - 1] 
                    : null;
                appState.isAddVideoModalOpen = true;
            }}
            class="w-full flex items-center justify-center {appState.isSidebarOpen
                ? 'md:justify-start'
                : 'justify-center'} gap-3 px-4 py-2 rounded-lg bg-zinc-900 hover:bg-zinc-800 text-white transition-colors"
        >
            <span class="text-xl font-bold">+</span>
            {#if appState.isSidebarOpen}
                <span class="text-sm font-medium">Add Video</span>
            {/if}
        </button>
    </div>
</aside>
