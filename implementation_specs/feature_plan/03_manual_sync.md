# Feature 3: Replace Auto-Sync with Manual Sync Button in Dashboard

## Problem
Currently, sync runs automatically in the background after every data-mutating action (save, delete, rename, reorder, etc.) via a debounced `setChangeListener` callback. This is annoying and resource-intensive. The user wants to replace this "Smart Sync" with a **manual sync button** directly in the Dashboard footer bar.

## Current Implementation

### Auto-sync trigger chain

1. **`src/lib/db.ts`** — Every mutating function calls `notifyChange()`:
   ```typescript
   function notifyChange(): void {
       if (changeListener) {
           changeListener();
       }
   }
   ```

2. **`src/lib/state.svelte.ts`** (lines 50–59) — Constructor sets up the listener:
   ```typescript
   setChangeListener(() => {
       if (this.settings.autoSync && this.settings.githubToken && this.settings.githubRepo) {
           if (this.syncDebounceTimer) {
               clearTimeout(this.syncDebounceTimer);
           }
           this.syncDebounceTimer = setTimeout(() => {
               this.triggerSync().catch(console.error);
           }, 2000);
       }
   });
   ```

3. **`src/lib/Dashboard.svelte`** — Multiple places call `appState.triggerSync()` directly after drag-drop operations (lines 206–208, 353–355, 388–400).

4. **`src/lib/SettingsModal.svelte`** (lines 212–222) — Auto-sync checkbox:
   ```svelte
   <input type="checkbox" id="autosync" bind:checked={appState.settings.autoSync} />
   <label for="autosync">Auto-sync on changes (Smart Sync)</label>
   ```

### Sync status display in Dashboard footer
**`src/lib/Dashboard.svelte`** (lines 719–771) — Shows spinning/success/error icons.

## Changes Required

### Step 1: Remove auto-sync from state constructor

**File**: `src/lib/state.svelte.ts`

Remove or comment out the `setChangeListener` block in the constructor (lines 50–59):
```typescript
// REMOVE THIS ENTIRE BLOCK:
setChangeListener(() => {
    if (this.settings.autoSync && this.settings.githubToken && this.settings.githubRepo) {
        if (this.syncDebounceTimer) {
            clearTimeout(this.syncDebounceTimer);
        }
        this.syncDebounceTimer = setTimeout(() => {
            this.triggerSync().catch(console.error);
        }, 2000);
    }
});
```

Also remove the `syncDebounceTimer` property (line 42):
```typescript
// REMOVE:
private syncDebounceTimer: ReturnType<typeof setTimeout> | null = null;
```

And remove the `autoSync` setting from the settings object (line 24):
```typescript
// REMOVE from settings:
autoSync: false,
```

### Step 2: Remove direct `triggerSync()` calls from Dashboard

**File**: `src/lib/Dashboard.svelte`

Remove these sync trigger blocks — there are 4 occurrences scattered throughout the file:

**Location 1** — after `handleReorder` (lines 206–209):
```typescript
// REMOVE:
if (appState.settings.githubToken && appState.settings.githubRepo) {
    appState.triggerSync();
}
```

**Location 2** — after `handleDropOnItem` "move-inside" (lines 353–355):
```typescript
// REMOVE:
if (appState.settings.githubToken && appState.settings.githubRepo) {
    appState.triggerSync();
}
```

**Location 3** — after `handleDropOnColumn` video drop (lines 388–391):
```typescript
// REMOVE:
if (appState.settings.githubToken && appState.settings.githubRepo) {
    appState.triggerSync();
}
```

**Location 4** — after `handleDropOnColumn` folder drop (lines 397–400):
```typescript
// REMOVE:
if (appState.settings.githubToken && appState.settings.githubRepo) {
    appState.triggerSync();
}
```

### Step 3: Add manual sync button to Dashboard footer

**File**: `src/lib/Dashboard.svelte`

In the footer bar (around line 773, inside `<div class="flex gap-2">`), add a sync button **before** the "New Folder" button:

```svelte
<!-- Manual Sync Button -->
{#if appState.settings.githubToken && appState.settings.githubRepo}
    <button
        onclick={() => appState.triggerSync()}
        disabled={appState.syncStatus === "syncing"}
        class="p-1 hover:bg-zinc-800 rounded text-zinc-400 hover:text-white disabled:opacity-50 disabled:cursor-not-allowed"
        title="Sync Now"
    >
        <svg
            class="size-5 {appState.syncStatus === 'syncing' ? 'animate-spin' : ''}"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
        >
            <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
            />
        </svg>
    </button>
{/if}
```

This button:
- Only shows when GitHub token and repo are configured
- Shows a spinning animation while syncing
- Is disabled during sync to prevent double-triggers
- Reuses the same SVG icon already used in the sync status display

### Step 4: Update the sync status indicator

The existing sync status indicators in the Dashboard footer (lines 720–771) should be kept — they show "Syncing...", "Synced", and "Sync Error". These provide visual feedback *after* pressing the manual sync button.

Consider simplifying them to show only an icon (no text) to save space:

```svelte
{#if appState.syncStatus === "success"}
    <svg class="size-4 text-green-500" ...><!-- checkmark --></svg>
{:else if appState.syncStatus === "error"}
    <svg class="size-4 text-red-500" title={appState.syncError || "Unknown error"} ...><!-- error --></svg>
{/if}
```

### Step 5: Remove auto-sync toggle from Settings

**File**: `src/lib/SettingsModal.svelte`

Remove the auto-sync checkbox (lines 212–222):
```svelte
<!-- REMOVE THIS ENTIRE BLOCK: -->
<div class="flex items-center gap-2">
    <input
        type="checkbox"
        id="autosync"
        bind:checked={appState.settings.autoSync}
        class="..."
    />
    <label for="autosync" class="text-sm text-zinc-300">
        Auto-sync on changes (Smart Sync)
    </label>
</div>
```

Keep the existing "Sync Now" button in Settings — it provides an alternative sync trigger location.

### Step 6: Clean up unused imports

**File**: `src/lib/db.ts`

The `setChangeListener` function (lines 156–158) and related `changeListener` variable (line 3) and `notifyChange` function (lines 5–9) can be removed since nothing uses the listener anymore.

However, `notifyChange()` is still called by every mutating function. You can either:
- **Option A**: Remove all `notifyChange()` calls and the listener system entirely (clean but big diff)
- **Option B**: Leave the calls in place but remove the listener registration (smaller diff, no functional impact)

**Recommendation**: Option B — leave `notifyChange()` calls in place (they become no-ops since no listener is registered). This minimizes the diff and risk.

## Testing

1. Run `bun run tauri dev`
2. Make changes (add video, rename folder, drag items) → No automatic sync should trigger
3. Click the new sync button in the Dashboard footer → Sync should run, spinner shows
4. Open Settings → "Auto-sync on changes" checkbox should be gone
5. "Sync Now" button in Settings should still work
6. After successful sync → green checkmark appears in footer
7. If sync fails → red error icon appears in footer with hover tooltip
