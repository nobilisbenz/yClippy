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
    import { save, open } from "@tauri-apps/plugin-dialog";
    import { writeTextFile, readTextFile } from "@tauri-apps/plugin-fs";
    import { invoke } from "@tauri-apps/api/core";
    import ConfirmDialog from "./ConfirmDialog.svelte";
    import Icon from "./Icon.svelte";
    import Modal from "./Modal.svelte";

    /// Settings used to be one column six screens tall with no scroll
    /// container, so on a phone the buttons at the bottom were unreachable.
    /// Three tabs, one scroll region, one footer.
    type Tab = "copying" | "sync" | "data";
    let tab = $state<Tab>("copying");

    let template = $state(appState.settings.clipboardTemplate);
    let draftRepo = $state(appState.settings.githubRepo);
    let draftVault = $state(appState.settings.vaultPath);
    let draftToken = $state("");
    let tokenConfigured = $state(appState.settings.githubTokenPresent);
    let isSyncing = $state(false);
    let isSavingCredentials = $state(false);
    let localSyncMessage = $state<{ ok: boolean; text: string } | null>(null);
    let credentialMessage = $state<{ ok: boolean; text: string } | null>(null);
    let dbPath = $state("Loading…");

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

    $effect(() => {
        getDbPath().then((p) => (dbPath = p));
    });

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

    const PLACEHOLDERS = [
        "{id}",
        "{start}",
        "{end}",
        "{title}",
        "{url}",
        "{url_clean}",
        "{start_hms}",
        "{end_hms}",
    ];

    function close() {
        appState.isSettingsModalOpen = false;
    }

    function handleSave() {
        appState.updateSettings({ clipboardTemplate: template });
        appState.showToast("Settings saved", "success");
        close();
    }

    async function handleSaveCredentials() {
        isSavingCredentials = true;
        credentialMessage = null;
        try {
            await setGithubConfig(draftRepo, draftToken || null, draftVault);
            if (draftToken) draftToken = "";
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
            message: "Sync will stop working until you add a new one.",
            confirmLabel: "Clear token",
            danger: true,
        });
        if (confirmed === undefined) return;
        try {
            await clearGithubToken();
            await appState.refreshGithubConfig();
            credentialMessage = { ok: true, text: "Token cleared" };
        } catch (e) {
            credentialMessage = { ok: false, text: `Failed: ${String(e)}` };
        }
    }

    async function handlePickVault() {
        try {
            const picked = await open({ directory: true, title: "Choose your vault folder" });
            if (typeof picked === "string") draftVault = picked;
        } catch (e) {
            appState.showToast(`Could not open the picker: ${String(e)}`, "error");
        }
    }

    async function handleSyncNow() {
        localSyncMessage = null;
        isSyncing = true;
        const result = await appState.triggerSync();
        isSyncing = false;
        localSyncMessage = result.success
            ? { ok: true, text: result.detail ? `Synced — ${result.detail}` : "Synced" }
            : { ok: false, text: `Sync failed: ${result.error || "unknown error"}` };
    }

    async function handleChangeDb() {
        try {
            const path = await save({
                title: "Select or create a database file",
                defaultPath: "yclippy.db",
                filters: [{ name: "SQLite database", extensions: ["db", "sqlite", "sqlite3"] }],
            });
            if (!path) return;
            await setDbPath(path);
            dbPath = path;
            await appState.refreshAll();
            appState.showToast("Database path updated", "success");
        } catch (e) {
            console.error(e);
            appState.showToast(`Failed to change database: ${String(e)}`, "error");
        }
    }

    async function handleExport() {
        try {
            const path = await save({
                filters: [{ name: "JSON", extensions: ["json"] }],
                defaultPath: "yclippy_backup.json",
            });
            if (!path) return;
            const data = await exportDb();
            await writeTextFile(path, JSON.stringify(data, null, 2));
            appState.showToast("Data exported", "success");
        } catch (e) {
            console.error(e);
            appState.showToast(`Export failed: ${String(e)}`, "error");
        }
    }

    async function handleImport() {
        const confirmed = await showConfirm({
            title: "Import data?",
            message:
                "Importing merges the file into your library and can overwrite rows. Export a backup first.",
            confirmLabel: "Continue",
        });
        if (confirmed === undefined) return;
        try {
            const path = await open({ filters: [{ name: "JSON", extensions: ["json"] }] });
            if (!path) return;
            const content = await readTextFile(path as string);
            await importDb(JSON.parse(content));
            await appState.refreshFolders();
            await appState.refreshVideos();
            appState.showToast("Data imported", "success");
        } catch (e) {
            console.error(e);
            appState.showToast(`Import failed: ${String(e)}`, "error");
        }
    }

    async function handleImportFromYtRenamer() {
        const confirmed = await showConfirm({
            title: "Import from ytRenamer?",
            message: "This adds videos and clips to your library.",
            confirmLabel: "Continue",
        });
        if (confirmed === undefined) return;
        try {
            const path = await open({
                filters: [{ name: "JSON", extensions: ["json"] }],
                title: "Select a ytRenamer export",
            });
            if (!path) return;
            const youtubeUrl = await showConfirm({
                title: "Which video?",
                message: "Enter the YouTube URL this clip list belongs to.",
                confirmLabel: "Import",
                withInput: true,
                promptPlaceholder: "https://www.youtube.com/watch?v=…",
            });
            if (!youtubeUrl) return;
            const content = await readTextFile(path as string);
            const count = await invoke("import_from_yt_renamer", {
                fileContent: content,
                youtubeUrl,
            });
            await appState.refreshVideos();
            await appState.refreshActiveClips();
            appState.showToast(`Imported ${count} clips`, "success");
        } catch (e) {
            console.error(e);
            appState.showToast(`Import failed: ${String(e)}`, "error");
        }
    }
</script>

<Modal title="Settings" onClose={close} size="lg">
    <nav class="flex gap-1 mb-4 border-b border-[color:var(--border)] -mt-1">
        {#each [["copying", "Copying"], ["sync", "Sync"], ["data", "Data"]] as [id, label] (id)}
            <button
                class="px-3 py-2 text-[13px] border-b-2 -mb-px transition-colors {tab === id
                    ? 'border-[color:var(--accent)] text-[color:var(--text)]'
                    : 'border-transparent text-[color:var(--text-faint)] hover:text-[color:var(--text-dim)]'}"
                onclick={() => (tab = id as Tab)}
            >
                {label}
            </button>
        {/each}
    </nav>

    {#if tab === "copying"}
        <div class="flex flex-col gap-3">
            <div class="flex flex-wrap gap-2">
                {#each PRESETS as preset (preset.name)}
                    <button
                        onclick={() => (template = preset.value)}
                        title={preset.hint}
                        class="btn {template === preset.value ? 'btn-primary' : ''}"
                    >
                        {preset.name}
                    </button>
                {/each}
            </div>

            <div>
                <label class="label" for="template">Template</label>
                <textarea
                    id="template"
                    bind:value={template}
                    rows="4"
                    class="field font-mono text-xs leading-relaxed"
                ></textarea>
            </div>

            <div class="flex flex-wrap gap-1.5">
                {#each PLACEHOLDERS as placeholder (placeholder)}
                    <button
                        class="chip hover:text-[color:var(--text)] transition-colors"
                        title="Insert {placeholder}"
                        onclick={() => (template += placeholder)}
                    >
                        {placeholder}
                    </button>
                {/each}
            </div>

            <p class="text-xs text-[color:var(--text-faint)]">
                {PRESETS.find((p) => p.value === template)?.hint ??
                    "Click a placeholder to append it."}
            </p>
        </div>
    {:else if tab === "sync"}
        <div class="flex flex-col gap-4">
            <div>
                <label class="label" for="repo-url">Repository URL</label>
                <input
                    id="repo-url"
                    type="text"
                    bind:value={draftRepo}
                    placeholder="https://github.com/username/repo.git"
                    class="field"
                />
            </div>

            <div>
                <label class="label" for="vault-path">
                    Vault folder <span class="text-[color:var(--text-faint)]">(optional)</span>
                </label>
                <div class="flex gap-2">
                    <input
                        id="vault-path"
                        type="text"
                        bind:value={draftVault}
                        placeholder="~/Notes"
                        class="field flex-1 min-w-0"
                    />
                    <button onclick={handlePickVault} class="btn shrink-0">Browse</button>
                </div>
                <p class="text-[11px] text-[color:var(--text-faint)] mt-1.5 leading-relaxed">
                    Set this and desktop sync also writes the library to
                    <code class="chip">&lt;vault&gt;/.notes/yclippy/</code>, so
                    <code class="chip">yalive sync</code> carries it through git.
                </p>
            </div>

            <div>
                <label class="label" for="gh-token">
                    Access token
                    {#if tokenConfigured}
                        <span class="text-[color:var(--text-faint)]">— saved, paste to replace</span>
                    {/if}
                </label>
                <input
                    id="gh-token"
                    type="password"
                    bind:value={draftToken}
                    placeholder={tokenConfigured ? "•••••••• already saved" : "ghp_…"}
                    class="field"
                />
                <p class="text-[11px] text-[color:var(--text-faint)] mt-1.5">
                    Stored in app-private config. Never exposed to the webview.
                </p>
            </div>

            <div class="flex gap-2">
                <button
                    onclick={handleSaveCredentials}
                    disabled={isSavingCredentials || !draftRepo}
                    class="btn btn-primary flex-1"
                >
                    {isSavingCredentials ? "Saving…" : "Save credentials"}
                </button>
                {#if tokenConfigured}
                    <button onclick={handleClearToken} class="btn btn-danger">Clear token</button>
                {/if}
            </div>

            {#if credentialMessage}
                <p
                    class="text-xs text-center"
                    style="color: {credentialMessage.ok ? 'var(--success)' : 'var(--danger)'}"
                >
                    {credentialMessage.text}
                </p>
            {/if}

            <button
                onclick={handleSyncNow}
                disabled={isSyncing || !tokenConfigured || !draftRepo}
                class="btn w-full"
            >
                <span class={isSyncing ? "animate-spin" : ""}><Icon name="sync" size={14} /></span>
                {isSyncing ? "Syncing…" : "Sync now"}
            </button>

            {#if localSyncMessage}
                <p
                    class="text-xs text-center"
                    style="color: {localSyncMessage.ok ? 'var(--success)' : 'var(--danger)'}"
                >
                    {localSyncMessage.text}
                </p>
            {/if}
            {#if appState.settings.lastSyncAt}
                <p class="text-[11px] text-[color:var(--text-faint)] text-center">
                    Last synced {new Date(appState.settings.lastSyncAt).toLocaleString()}
                </p>
            {/if}
        </div>
    {:else}
        <div class="flex flex-col gap-4">
            <div class="flex gap-2">
                <button onclick={handleExport} class="btn flex-1">Export all data</button>
                <button onclick={handleImport} class="btn flex-1">Import data</button>
            </div>
            <button onclick={handleImportFromYtRenamer} class="btn w-full">
                Import from ytRenamer
            </button>

            <div class="pt-4 border-t border-[color:var(--border)]">
                <span class="label">Database file</span>
                <p
                    class="text-xs text-[color:var(--text-faint)] break-all mb-2 font-mono"
                    title={dbPath}
                >
                    {dbPath}
                </p>
                <button onclick={handleChangeDb} class="btn w-full">Change location…</button>
            </div>
        </div>
    {/if}

    {#snippet footer()}
        <button class="btn btn-ghost" onclick={close}>Cancel</button>
        <button class="btn btn-primary" onclick={handleSave}>Save changes</button>
    {/snippet}
</Modal>

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
