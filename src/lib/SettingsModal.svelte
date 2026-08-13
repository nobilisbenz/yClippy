<script lang="ts">
    import { appState } from "./state.svelte";
    import {
        exportDb,
        importDb,
        getDbPath,
        setDbPath,
        setGithubConfig,
        clearGithubToken,
    } from "./db";

    let template = $state(appState.settings.clipboardTemplate);
    let draftRepo = $state(appState.settings.githubRepo);
    let draftVault = $state(appState.settings.vaultPath);
    let draftToken = $state("");
    let tokenConfigured = $state(appState.settings.githubTokenPresent);
    let isSyncing = $state(false);
    let isSavingCredentials = $state(false);
    let localSyncMessage = $state<{ ok: boolean; text: string } | null>(null);
    let credentialMessage = $state<{ ok: boolean; text: string } | null>(null);

    let confirmState = $state<{
        open: boolean;
        title: string;
        message: string;
        confirmLabel: string;
        danger: boolean;
        withInput: boolean;
        promptPlaceholder: string;
        onConfirm: (value?: string) => void;
    }>({
        open: false,
        title: "",
        message: "",
        confirmLabel: "Confirm",
        danger: false,
        withInput: false,
        promptPlaceholder: "",
        onConfirm: () => {},
    });

    function showConfirm(opts: {
        title: string;
        message: string;
        confirmLabel?: string;
        danger?: boolean;
        withInput?: boolean;
        promptPlaceholder?: string;
    }): Promise<string | undefined> {
        return new Promise((resolve) => {
            confirmState = {
                open: true,
                title: opts.title,
                message: opts.message,
                confirmLabel: opts.confirmLabel ?? "Confirm",
                danger: opts.danger ?? false,
                withInput: opts.withInput ?? false,
                promptPlaceholder: opts.promptPlaceholder ?? "",
                onConfirm: (value) => {
                    confirmState.open = false;
                    resolve(value);
                },
            };
        });
    }

    function cancelConfirm() {
        confirmState.open = false;
        confirmState.onConfirm(undefined);
    }

    $effect(() => {
        draftRepo = appState.settings.githubRepo;
        draftVault = appState.settings.vaultPath;
        tokenConfigured = appState.settings.githubTokenPresent;
    });

    async function handleSaveCredentials() {
        isSavingCredentials = true;
        credentialMessage = null;
        try {
            await setGithubConfig(draftRepo, draftToken || null, draftVault);
            if (draftToken) {
                draftToken = "";
            }
            await appState.refreshGithubConfig();
            credentialMessage = {
                ok: true,
                text: tokenConfigured ? "Credentials updated" : "Credentials saved",
            };
        } catch (e) {
            credentialMessage = { ok: false, text: `Failed: ${String(e)}` };
        } finally {
            isSavingCredentials = false;
        }
    }

    async function handleClearToken() {
        const confirmed = await showConfirm({
            title: "Clear GitHub token?",
            message:
                "Clear the stored GitHub token? Sync will stop working until you add a new one.",
            confirmLabel: "Clear token",
            danger: true,
        });
        if (!confirmed && confirmed !== "") return;
        try {
            await clearGithubToken();
            await appState.refreshGithubConfig();
            credentialMessage = { ok: true, text: "Token cleared" };
        } catch (e) {
            credentialMessage = { ok: false, text: `Failed: ${String(e)}` };
        }
    }

    function handleSave() {
        appState.updateSettings({ clipboardTemplate: template });
        appState.isSettingsModalOpen = false;
    }

    // Three shapes a clip gets copied as. `@video` and `clip:` are what the
    // vault understands: the first makes the moment a section-level fact, the
    // second attaches it to a quiz card so it plays as an answer in yReviewy.
    const PRESETS = [
        {
            name: "Embed",
            hint: "An iframe for a web page",
            value: `<iframe src="https://www.youtube.com/embed/{id}?start={start}&end={end}" height="360" width="100%" seamless="seamless" frameborder="0" allowfullscreen></iframe>`,
        },
        {
            name: "@video line",
            hint: "A moment, indexed and replayable from the vault",
            value: `@video {url_clean} {start_hms}  {title}`,
        },
        {
            name: "clip: line",
            hint: "Attach to a quiz block — plays as the answer",
            value: `clip: {url_clean} {start_hms}-{end_hms}  {title}`,
        },
    ];

    function handleReset() {
        template = PRESETS[0].value;
    }

    import { save, open } from "@tauri-apps/plugin-dialog";
    import { writeTextFile, readTextFile } from "@tauri-apps/plugin-fs";
    import { invoke } from "@tauri-apps/api/core";
    import ConfirmDialog from "./ConfirmDialog.svelte";

    let dbPath = $state("Loading...");

    $effect(() => {
        getDbPath().then((p) => (dbPath = p));
    });

    async function handlePickVault() {
        try {
            const picked = await open({ directory: true, title: "Choose your vault folder" });
            if (typeof picked === "string") draftVault = picked;
        } catch (e) {
            appState.showToast(`Could not open the picker: ${String(e)}`, "error");
        }
    }

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
                appState.showToast("Database path updated", "success");
            }
        } catch (e) {
            console.error(e);
            appState.showToast(`Failed to change database: ${String(e)}`, "error");
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
                appState.showToast("Data exported successfully", "success");
            }
        } catch (e) {
            console.error(e);
            appState.showToast(`Export failed: ${String(e)}`, "error");
        }
    }

    async function handleImport() {
        const confirmed = await showConfirm({
            title: "Import data?",
            message:
                "Importing data will overwrite/merge with existing data. It is recommended to backup first.",
            confirmLabel: "Continue",
        });
        if (confirmed === undefined) return;

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
                appState.showToast("Data imported successfully", "success");
            }
        } catch (e) {
            console.error(e);
            appState.showToast(`Import failed: ${String(e)}`, "error");
        }
    }
    async function handleImportFromYtRenamer() {
        const confirmed = await showConfirm({
            title: "Import from ytRenamer?",
            message:
                "Importing from ytRenamer will add videos and clips to your library.",
            confirmLabel: "Continue",
        });
        if (confirmed === undefined) return;

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
                const youtubeUrl = await showConfirm({
                    title: "YouTube URL",
                    message:
                        "Enter the YouTube URL that this ytRenamer clip list belongs to.",
                    confirmLabel: "Import",
                    withInput: true,
                    promptPlaceholder: "https://youtube.com/watch?v=...",
                });
                if (!youtubeUrl) return;

                const content = await readTextFile(path);
                const count = await invoke("import_from_yt_renamer", { fileContent: content, youtubeUrl });
                await appState.refreshVideos();
                await appState.refreshActiveClips();
                appState.showToast(`Imported ${count} clips from ytRenamer`, "success");
            }
        } catch (e) {
            console.error(e);
            appState.showToast(`Import failed: ${String(e)}`, "error");
        }
    }

    async function handleSyncNow() {
        localSyncMessage = null;
        isSyncing = true;
        const result = await appState.triggerSync();
        isSyncing = false;
        if (result.success) {
            localSyncMessage = {
                ok: true,
                text: result.detail ? `Synced — ${result.detail}` : "Synced",
            };
        } else {
            localSyncMessage = { ok: false, text: `Sync failed: ${result.error || "unknown error"}` };
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
                <code class="bg-zinc-800 px-1 rounded">{"{start_hms}"}</code>
                <code class="bg-zinc-800 px-1 rounded">{"{end_hms}"}</code>
                <code class="bg-zinc-800 px-1 rounded">{"{url_clean}"}</code>
            </p>
            <div class="flex flex-wrap gap-2 mb-2">
                {#each PRESETS as preset (preset.name)}
                    <button
                        onclick={() => (template = preset.value)}
                        title={preset.hint}
                        class="px-2.5 py-1 text-xs rounded border transition {template ===
                        preset.value
                            ? 'border-[color:var(--accent)] text-[color:var(--text)]'
                            : 'border-zinc-800 text-zinc-400 hover:border-zinc-700'}"
                    >
                        {preset.name}
                    </button>
                {/each}
            </div>
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
                        bind:value={draftRepo}
                        placeholder="https://github.com/username/repo.git"
                        class="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-white focus:outline-none focus:border-blue-600 transition text-sm"
                    />
                </div>
                <div>
                    <label
                        for="vault-path"
                        class="block text-xs font-medium text-zinc-400 mb-1"
                    >
                        Vault folder <span class="text-zinc-600">(optional)</span>
                    </label>
                    <div class="flex gap-2">
                        <input
                            id="vault-path"
                            type="text"
                            bind:value={draftVault}
                            placeholder="~/Notes"
                            class="flex-1 min-w-0 bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-white focus:outline-none focus:border-blue-600 transition text-sm"
                        />
                        <button
                            onclick={handlePickVault}
                            class="px-3 py-2 text-sm rounded-lg border border-zinc-800 text-zinc-300 hover:border-zinc-700 transition shrink-0"
                        >
                            Browse
                        </button>
                    </div>
                    <p class="text-[11px] text-zinc-500 mt-1">
                        Set this and desktop sync also writes the library to
                        <code class="bg-zinc-800 px-1 rounded">&lt;vault&gt;/.notes/yclippy/</code>,
                        so <code class="bg-zinc-800 px-1 rounded">yalive sync</code> carries it through git.
                    </p>
                </div>
                <div>
                    <label
                        for="gh-token"
                        class="block text-xs font-medium text-zinc-400 mb-1"
                    >
                        Classic Access Token {tokenConfigured ? "(configured — paste to replace)" : ""}
                    </label>
                    <input
                        id="gh-token"
                        type="password"
                        bind:value={draftToken}
                        placeholder={tokenConfigured ? "•••••••• (already saved)" : "ghp_..."}
                        class="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-white focus:outline-none focus:border-blue-600 transition text-sm"
                    />
                    <p class="text-xs text-zinc-500 mt-1">
                        Stored in app-private config. Never exposed to the webview.
                    </p>
                </div>
                <div class="flex gap-2">
                    <button
                        onclick={handleSaveCredentials}
                        disabled={isSavingCredentials || !draftRepo}
                        class="flex-1 py-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg text-white text-sm font-medium transition"
                    >
                        {isSavingCredentials ? "Saving…" : "Save Credentials"}
                    </button>
                    {#if tokenConfigured}
                        <button
                            onclick={handleClearToken}
                            class="px-4 py-2 bg-zinc-800 hover:bg-red-900 rounded-lg text-zinc-300 text-sm font-medium transition border border-zinc-700"
                        >
                            Clear Token
                        </button>
                    {/if}
                </div>
                {#if credentialMessage}
                    <p class="text-xs text-center {credentialMessage.ok ? 'text-green-400' : 'text-red-400'}">
                        {credentialMessage.text}
                    </p>
                {/if}
                <button
                    onclick={handleSyncNow}
                    disabled={isSyncing || !tokenConfigured || !draftRepo}
                    class="w-full py-2 bg-zinc-800 hover:bg-zinc-700 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg text-zinc-300 font-medium transition border border-zinc-700 hover:border-zinc-500 flex items-center justify-center gap-2"
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
                        class="lucide lucide-refresh-cw {isSyncing ? 'animate-spin' : ''}"
                        ><path
                            d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"
                        /><path d="M21 3v5h-5" /><path
                            d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"
                        /><path d="M3 21v-5h5" /></svg
                    >
                    {isSyncing ? "Syncing..." : "Sync Now"}
                </button>
                {#if localSyncMessage}
                    <p class="text-xs text-center {localSyncMessage.ok ? 'text-green-400' : 'text-red-400'}">
                        {localSyncMessage.text}
                    </p>
                {/if}
                {#if appState.settings.lastSyncAt && localSyncMessage?.ok}
                    <p class="text-xs text-zinc-500 text-center mt-2">
                        Last synced: {new Date(appState.settings.lastSyncAt).toLocaleString()}
                    </p>
                {/if}
            </div>
        </div>

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

<ConfirmDialog
    open={confirmState.open}
    title={confirmState.title}
    message={confirmState.message}
    confirmLabel={confirmState.confirmLabel}
    danger={confirmState.danger}
    withInput={confirmState.withInput}
    promptPlaceholder={confirmState.promptPlaceholder}
    onConfirm={confirmState.onConfirm}
    onCancel={cancelConfirm}
/>
