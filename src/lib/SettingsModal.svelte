<script lang="ts">
    import { appState } from "./state.svelte";

    let template = $state(appState.settings.clipboardTemplate);

    function handleSave() {
        appState.updateSettings({ clipboardTemplate: template });
        appState.isSettingsModalOpen = false;
    }

    function handleReset() {
        template = `<iframe src="https://www.youtube.com/embed/{id}?start={start}&end={end}" height="360" width="100%" seamless="seamless" frameborder="0" allowfullscreen></iframe>`;
    }

    import { exportDb, importDb, getDbPath, setDbPath } from "./db";
    import { save, open } from "@tauri-apps/plugin-dialog";
    import { writeTextFile, readTextFile } from "@tauri-apps/plugin-fs";
    import { invoke } from "@tauri-apps/api/core";

    let dbPath = $state("Loading...");

    $effect(() => {
        getDbPath().then((p) => (dbPath = p));
    });

    async function handleChangeDb() {
        try {
            const path = await save({
                title: "Select or Create Database File",
                defaultPath: "yclippy.db",
                filters: [
                    {
                        name: "SQLite Database",
                        extensions: ["db", "sqlite", "sqlite3"],
                    },
                ],
            });

            if (path) {
                await setDbPath(path);
                dbPath = path;
                await appState.refreshAll();
                alert("Database path updated successfully!");
            }
        } catch (e) {
            console.error(e);
            alert("Failed to change database: " + e);
        }
    }

    async function handleExport() {
        try {
            const path = await save({
                filters: [
                    {
                        name: "JSON",
                        extensions: ["json"],
                    },
                ],
                defaultPath: "yclippy_backup.json",
            });

            if (path) {
                const data = await exportDb();
                await writeTextFile(path, JSON.stringify(data, null, 2));
                alert("Data exported successfully!");
            }
        } catch (e) {
            console.error(e);
            alert("Export failed: " + e);
        }
    }

    async function handleImport() {
        if (
            !confirm(
                "Importing data will overwrite/merge with existing data. It is recommended to backup first. Continue?",
            )
        )
            return;

        try {
            const path = await open({
                filters: [
                    {
                        name: "JSON",
                        extensions: ["json"],
                    },
                ],
            });

            if (path) {
                const content = await readTextFile(path);
                const data = JSON.parse(content);
                await importDb(data);
                await appState.refreshFolders();
                await appState.refreshVideos();
                alert("Data imported successfully!");
            }
        } catch (e) {
            console.error(e);
            alert("Import failed: " + e);
        }
    }
async function handleImportFromYtRenamer() {
        if (
            !confirm(
                "Importing from ytRenamer will add videos and clips to your library. Continue?",
            )
        )
            return;

        try {
            const path = await open({
                filters: [
                    {
                        name: "JSON",
                        extensions: ["json"],
                    },
                ],
                title: "Select ytRenamer Export File",
            });

            if (path) {
                const youtubeUrl = prompt("Enter YouTube URL for this clip list:");
                if (!youtubeUrl) return;

                const content = await readTextFile(path);
                const count = await invoke("import_from_yt_renamer", { fileContent: content, youtubeUrl });
                await appState.refreshVideos();
                await appState.refreshActiveClips();
                alert(`Successfully imported ${count} clips from ytRenamer!`);
            }
        } catch (e) {
            console.error(e);
            alert("Import failed: " + e);
        }
    }
</script>

<div
    class="absolute inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm"
>
    <div
        class="w-full max-w-lg bg-zinc-900 border border-zinc-800 rounded-xl p-6 shadow-2xl"
    >
        <h3 class="text-lg font-bold text-white mb-6">Settings</h3>

        <div class="mb-6">
            <label
                class="block text-sm font-medium text-zinc-400 mb-2"
                for="template"
            >
                Clipboard Export Template
            </label>
            <p class="text-xs text-zinc-500 mb-2">
                Available placeholders: <code class="bg-zinc-800 px-1 rounded"
                    >{"{id}"}</code
                > <code class="bg-zinc-800 px-1 rounded">{"{start}"}</code>
                <code class="bg-zinc-800 px-1 rounded">{"{end}"}</code>
                <code class="bg-zinc-800 px-1 rounded">{"{title}"}</code>
                <code class="bg-zinc-800 px-1 rounded">{"{url}"}</code>
            </p>
            <textarea
                id="template"
                bind:value={template}
                rows="4"
                class="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-4 py-3 text-white focus:outline-none focus:border-blue-600 transition font-mono text-xs leading-relaxed"
            ></textarea>
            <button
                onclick={handleReset}
                class="text-xs text-blue-500 hover:text-blue-400 mt-2"
            >
                Reset to Default
            </button>
        </div>

        <!-- GitHub Sync -->
        <div class="mb-6 pt-6 border-t border-zinc-800">
            <h4 class="text-sm font-medium text-white mb-4">GitHub Sync</h4>
            <div class="space-y-4">
                <div>
                    <label
                        for="repo-url"
                        class="block text-xs font-medium text-zinc-400 mb-1"
                    >
                        Repository URL
                    </label>
                    <input
                        id="repo-url"
                        type="text"
                        bind:value={appState.settings.githubRepo}
                        placeholder="https://github.com/username/repo.git"
                        class="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-white focus:outline-none focus:border-blue-600 transition text-sm"
                    />
                </div>
                <div>
                    <label
                        for="gh-token"
                        class="block text-xs font-medium text-zinc-400 mb-1"
                    >
                        Classic Access Token
                    </label>
                    <input
                        id="gh-token"
                        type="password"
                        bind:value={appState.settings.githubToken}
                        placeholder="ghp_..."
                        class="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-white focus:outline-none focus:border-blue-600 transition text-sm"
                    />
                </div>
                <button
                    onclick={() =>
                        appState
                            .triggerSync()
                            .then(() => alert("Sync successful!"))
                            .catch((e) => alert("Sync failed: " + e))}
                    class="w-full py-2 bg-zinc-800 hover:bg-zinc-700 rounded-lg text-zinc-300 font-medium transition border border-zinc-700 hover:border-zinc-500 flex items-center justify-center gap-2"
                >
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        width="16"
                        height="16"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        class="lucide lucide-refresh-cw"
                        ><path
                            d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"
                        /><path d="M21 3v5h-5" /><path
                            d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"
                        /><path d="M3 21v-5h5" /></svg
                    >
                    Sync Now
                </button>
                {#if appState.settings.lastSyncAt}
                    <p class="text-xs text-zinc-500 text-center mt-2">
                        Last synced: {new Date(appState.settings.lastSyncAt).toLocaleString()}
                    </p>
                {/if}
            </div>
        </div>

<!-- Data Management -->
        <div class="mb-6 pt-6 border-t border-zinc-800">
            <h4 class="text-sm font-medium text-white mb-4">Data Management</h4>
            <div class="flex gap-4 mb-4">
                <button
                    onclick={handleExport}
                    class="flex-1 py-2 bg-zinc-800 hover:bg-zinc-700 rounded-lg text-zinc-300 font-medium transition border border-zinc-700 hover:border-zinc-500"
                >
                    Export All Data
                </button>
                <button
                    onclick={handleImport}
                    class="flex-1 py-2 bg-zinc-800 hover:bg-zinc-700 rounded-lg text-zinc-300 font-medium transition border border-zinc-700 hover:border-zinc-500"
                >
                    Import Data
                </button>
            </div>
            <button
                onclick={handleImportFromYtRenamer}
                class="w-full py-2 bg-blue-900/50 hover:bg-blue-900 rounded-lg text-blue-300 font-medium transition border border-blue-800 hover:border-blue-600"
            >
                Import from ytRenamer
            </button>
        </div>

        <!-- Database Location -->
        <div class="mb-6 pt-6 border-t border-zinc-800">
            <h4 class="text-sm font-medium text-white mb-2">
                Database Location
            </h4>
            <p class="text-xs text-zinc-500 mb-2 truncate">Current: {dbPath}</p>
            <button
                onclick={handleChangeDb}
                class="w-full py-2 bg-zinc-800 hover:bg-zinc-700 rounded-lg text-zinc-300 font-medium transition border border-zinc-700 hover:border-zinc-500"
            >
                Change Database Location...
            </button>
        </div>
        <div class="flex justify-end gap-3">
            <button
                onclick={() => (appState.isSettingsModalOpen = false)}
                class="px-4 py-2 rounded-lg hover:bg-zinc-800 text-zinc-300 transition"
            >
                Cancel
            </button>
            <button
                onclick={handleSave}
                class="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded-lg text-white font-medium transition"
            >
                Save Changes
            </button>
        </div>
    </div>
</div>
