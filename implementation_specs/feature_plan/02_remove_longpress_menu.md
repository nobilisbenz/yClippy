# Feature 2: Remove Long-Press Context Menu on Dashboard Items

## Problem
When the user taps and holds (long-press) on a video or folder item in the Dashboard, a context menu appears. The user does not want this behavior — they want the long-press/context menu removed from Dashboard items.

## Current Implementation

The context menu is triggered by the `oncontextmenu` event handler on both **folder items** and **video items** in `Dashboard.svelte`.

### Folder Context Menu
**File**: `src/lib/Dashboard.svelte`  
**Lines**: 438–474

```svelte
oncontextmenu={(e) => {
    e.preventDefault();
    appState.contextMenu = {
        x: e.clientX,
        y: e.clientY,
        show: true,
        items: [
            {
                label: "Rename",
                action: async () => { ... },
            },
            {
                label: "Delete",
                danger: true,
                action: () => handleDeleteFolder(folder.id!, { ... } as MouseEvent),
            },
        ],
    };
}}
```

### Video Context Menu
**File**: `src/lib/Dashboard.svelte`  
**Lines**: 564–607

```svelte
oncontextmenu={(e) => {
    e.preventDefault();
    appState.contextMenu = {
        x: e.clientX,
        y: e.clientY,
        show: true,
        items: [
            { label: "Edit", action: () => { ... } },
            { label: "Rename", action: async () => { ... } },
            { label: "Delete", danger: true, action: () => handleDeleteVideo(...) },
        ],
    };
}}
```

## Changes Required

### Step 1: Remove `oncontextmenu` handlers from folder items

**File**: `src/lib/Dashboard.svelte`

Find the folder `<div>` element (around line 429) and **remove the entire `oncontextmenu` attribute** (lines 438–474). The div starts with:
```svelte
<div
    role="button"
    tabindex="0"
    draggable="true"
    ondragstart={(e) => onDragStartFolder(e, folder.id!)}
    onclick={(e) => { ... }}
    oncontextmenu={(e) => {   <!-- REMOVE THIS ENTIRE HANDLER -->
        ...
    }}
```

Simply delete the `oncontextmenu={...}` attribute entirely from this element.

### Step 2: Remove `oncontextmenu` handlers from video items

**File**: `src/lib/Dashboard.svelte`

Find the video `<div>` element (around line 555) and **remove the entire `oncontextmenu` attribute** (lines 564–607). Same approach — delete the handler.

### Step 3: (Optional) Remove unused delete buttons

The Dashboard currently also has inline **delete buttons** (X icons) visible on hover for both folders (line 529–547) and videos (line 644–661). These are **separate** from the context menu and provide delete functionality independently.

**Decision for the AI**: Keep or remove the inline delete buttons? They still work without the context menu. The Rename and Edit actions that were ONLY accessible via context menu will no longer be available unless:
- The user accesses them from somewhere else
- You add them as inline buttons

**Recommendation**: Keep the inline delete buttons. For video Edit access, the user can still right-click on desktop (this removal only affects mobile long-press). If you want to be thorough, only conditionally remove the context menu on Android:

```svelte
oncontextmenu={(e) => {
    if (platform() === "android") return;  // No context menu on mobile
    e.preventDefault();
    appState.contextMenu = { ... };
}}
```

But if the user wants it fully removed on all platforms, just delete the handlers entirely.

## Testing

1. Run the app: `bun run tauri dev`
2. Long-press on a folder item → No context menu should appear
3. Long-press on a video item → No context menu should appear
4. Right-click on desktop → No context menu should appear (if fully removed) OR still works (if conditionally removed for Android only)
5. Existing click behavior (opening folders/videos) must still work normally
6. Drag & drop must still work normally
