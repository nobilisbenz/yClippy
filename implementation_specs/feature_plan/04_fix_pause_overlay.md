# Feature 4: Fix Blurred Pause Overlay (Can't Read Video Content)

## Problem
When the user pauses a video, a full-screen overlay appears with `backdrop-blur-sm`, making the video content behind it completely unreadable. The user wants to still be able to see/read the content of the video they're watching when paused.

## Current Implementation

**File**: `src/lib/VideoPlayer.svelte`  
**Lines**: 169–213

```svelte
{#if isPaused}
    <div
        class="absolute inset-0 z-20 bg-black/50 flex flex-col items-center justify-center cursor-pointer backdrop-blur-sm transition-opacity duration-200"
        onclick={togglePlay}
        role="button"
        tabindex="0"
        onkeydown={(e) => e.key === "Enter" && togglePlay()}
    >
        <!-- Back Button in Overlay -->
        <button ... >
            <!-- back arrow SVG -->
        </button>

        <div class="bg-zinc-900/90 rounded-full p-4 ...">
            <!-- play icon SVG -->
        </div>
    </div>
{/if}
```

The problematic classes are:
- `bg-black/50` — 50% opacity black background (dims the video significantly)
- `backdrop-blur-sm` — applies a CSS blur filter to everything behind the overlay

Together these make the video content completely unreadable.

## Changes Required

### Step 1: Remove blur and reduce background opacity

**File**: `src/lib/VideoPlayer.svelte`  
**Line**: 171

Change the overlay `<div>` class from:
```
class="absolute inset-0 z-20 bg-black/50 flex flex-col items-center justify-center cursor-pointer backdrop-blur-sm transition-opacity duration-200"
```

To:
```
class="absolute inset-0 z-20 bg-black/20 flex flex-col items-center justify-center cursor-pointer transition-opacity duration-200"
```

**What changed**:
1. **Removed** `backdrop-blur-sm` — no more blur effect
2. **Changed** `bg-black/50` to `bg-black/20` — reduced from 50% to 20% opacity so the video is clearly visible

### Step 2: (Optional) Make the play button more visible

Since the overlay is now more transparent, the play button and back button should have stronger backgrounds to remain visible:

**Play button container** (line 201–202):
```svelte
<!-- Change from: -->
<div class="bg-zinc-900/90 rounded-full p-4 hover:scale-110 transition-transform shadow-2xl border border-zinc-700/50">

<!-- To: -->
<div class="bg-black/70 rounded-full p-4 hover:scale-110 transition-transform shadow-2xl border border-zinc-600/50">
```

**Back button** (line 183):
```svelte
<!-- Change from: -->
class="absolute top-4 left-4 p-3 bg-zinc-900/80 rounded-full hover:bg-zinc-800 transition-colors border border-zinc-700/50 shadow-lg group"

<!-- To (keep as-is or slightly darken): -->
class="absolute top-4 left-4 p-3 bg-black/70 rounded-full hover:bg-black/90 transition-colors border border-zinc-600/50 shadow-lg group"
```

## Alternative Approach

If the user wants NO overlay at all and only a floating play button:

```svelte
{#if isPaused}
    <!-- Back button only -->
    <button
        onclick={(e) => { e.stopPropagation(); handleBack(); }}
        class="absolute top-4 left-4 z-20 p-3 bg-black/70 rounded-full hover:bg-black/90 transition-colors border border-zinc-600/50 shadow-lg"
        aria-label="Back to Library"
    >
        <!-- back arrow SVG -->
    </button>

    <!-- Centered play button only -->
    <button
        onclick={togglePlay}
        class="absolute inset-0 z-20 flex items-center justify-center cursor-pointer"
        aria-label="Play"
    >
        <div class="bg-black/70 rounded-full p-4 hover:scale-110 transition-transform shadow-2xl border border-zinc-600/50">
            <!-- play icon SVG -->
        </div>
    </button>
{/if}
```

This removes the dark tint entirely — only a floating play button and back button appear over the fully-visible paused video.

## Testing

1. Run `bun run tauri dev`
2. Open a video and start playing
3. Pause the video → The video content should be clearly visible/readable through the overlay
4. The play button and back button should still be clearly visible and clickable
5. Clicking the overlay or play button should resume playback
6. The back button should still navigate to the Dashboard
