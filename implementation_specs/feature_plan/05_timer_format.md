# Feature 5: Fix Timer Format (hh:mm:ss) + Dashboard Last Position Display

## Problem

Two issues:

1. **VideoPlayer timer** shows `mm:ss` format (e.g., `12:45`). User wants `hh:mm:ss` format (e.g., `0:12:45`).
2. **Dashboard video list** always shows `0:00` for every video's duration instead of showing the last watched position/time.

## Current Implementation

### VideoPlayer Timer
**File**: `src/lib/VideoPlayer.svelte`  
**Lines**: 219–223

```svelte
<div class="text-xl font-mono text-white">
    {Math.floor(currentTime / 60)}:{Math.floor(currentTime % 60)
        .toString()
        .padStart(2, "0")}
</div>
```

This computes `minutes:seconds` — no hours.

### Dashboard Duration Display
**File**: `src/lib/Dashboard.svelte`  
**Lines**: 61–65 (function definition)

```typescript
function formatDuration(seconds: number) {
    const m = Math.floor(seconds / 60);
    const s = seconds % 60;
    return `${m}:${s.toString().padStart(2, "0")}`;
}
```

**Lines**: 638–640 (usage in video item):
```svelte
<span class="text-xs text-zinc-600">{formatDuration(video.duration)}</span>
```

The issue is that `video.duration` is the total video duration (which might be 0 if never fetched from YouTube API). It does NOT show `video.last_position`.

### Video Interface
**File**: `src/lib/db.ts` (lines 11–22):
```typescript
export interface Video {
    id: string;
    title: string;
    thumbnail_url: string;
    duration: number;        // Total video duration (often 0)
    last_position: number;   // Last watched position in seconds
    ...
}
```

### `last_position` is saved correctly
**File**: `src/lib/VideoPlayer.svelte` (lines 117–126):
```typescript
onDestroy(async () => {
    clearInterval(timer);
    if (video) {
        video.last_position = Math.floor(currentTime);
        saveVideo(video).then(() => appState.refreshVideos());
    }
});
```

So `last_position` IS being saved to the database. The Dashboard just isn't displaying it.

## Changes Required

### Step 1: Create a shared `formatTime` utility function

Since both VideoPlayer and Dashboard need the same formatting, create a shared helper.

**Option A** — Add to `src/lib/db.ts` (or a new `src/lib/utils.ts` file):

```typescript
export function formatTime(totalSeconds: number): string {
    const h = Math.floor(totalSeconds / 3600);
    const m = Math.floor((totalSeconds % 3600) / 60);
    const s = Math.floor(totalSeconds % 60);
    return `${h}:${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
}
```

Output examples:
- `0` → `0:00:00`
- `65` → `0:01:05`
- `3725` → `1:02:05`

### Step 2: Update VideoPlayer timer display

**File**: `src/lib/VideoPlayer.svelte`

Import the helper:
```typescript
import { formatTime } from "./db";  // or "./utils"
```

Replace lines 219–223:
```svelte
<!-- FROM: -->
<div class="text-xl font-mono text-white">
    {Math.floor(currentTime / 60)}:{Math.floor(currentTime % 60)
        .toString()
        .padStart(2, "0")}
</div>

<!-- TO: -->
<div class="text-xl font-mono text-white">
    {formatTime(currentTime)}
</div>
```

### Step 3: Update Dashboard to show `last_position` and use `hh:mm:ss`

**File**: `src/lib/Dashboard.svelte`

Import the helper:
```typescript
import { formatTime } from "./db";  // or "./utils"
```

**Replace** the existing `formatDuration` function (lines 61–65):
```typescript
// REMOVE:
function formatDuration(seconds: number) {
    const m = Math.floor(seconds / 60);
    const s = seconds % 60;
    return `${m}:${s.toString().padStart(2, "0")}`;
}
```

**Update** the video item's duration display (lines 638–640):
```svelte
<!-- FROM: -->
<span class="text-xs text-zinc-600">{formatDuration(video.duration)}</span>

<!-- TO: -->
<span class="text-xs text-zinc-600">
    {video.last_position > 0 ? formatTime(video.last_position) : (video.duration > 0 ? formatTime(video.duration) : "")}
</span>
```

**Logic**:
- If `last_position > 0` → Show last watched position (e.g., `0:12:45`) — the user can see where they left off
- Else if `duration > 0` → Show total duration as fallback
- Else → Show nothing (avoids `0:00:00` for videos that have no data)

### Alternative Display: Show Both

If the user prefers to see both position and duration:
```svelte
<span class="text-xs text-zinc-600">
    {#if video.last_position > 0}
        {formatTime(video.last_position)}{#if video.duration > 0} / {formatTime(video.duration)}{/if}
    {:else if video.duration > 0}
        {formatTime(video.duration)}
    {/if}
</span>
```

This shows `0:12:45 / 1:02:05` style.

## Testing

1. Run `bun run tauri dev`
2. **VideoPlayer timer**: Open a video → Timer should show `h:mm:ss` format (e.g., `0:00:00`, `0:01:30`, `1:15:42`)
3. **Dashboard**: Go back to dashboard → The video you just watched should show the last position (e.g., `0:01:30`) instead of `0:00`
4. Videos never watched should show total duration if available, or nothing
5. Open a long video (>1 hour) → Timer should show hours correctly (e.g., `1:23:45`)
