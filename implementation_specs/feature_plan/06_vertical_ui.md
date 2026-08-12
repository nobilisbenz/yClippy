# Feature 6: Fix Vertical Phone UI + Clips Scroll Direction

## Problem

Two layout issues on mobile (phone in portrait mode):

1. **Dashboard vertical UI** — The layout doesn't work well in portrait/vertical orientation on phones. Miller columns need better mobile adaptation.
2. **Clips menu scrolls horizontally** — When the ClipList sidebar is open, clips are laid out in a horizontal row and scroll left-to-right. The user wants them to scroll **vertically** (top-to-bottom) on phone.

## Current Implementation

### ClipList Layout
**File**: `src/lib/ClipList.svelte`  
**Line**: 248

```svelte
<div class="flex flex-row md:flex-col gap-2 p-2 h-full">
```

- `flex flex-row` — On mobile: horizontal layout (left to right)
- `md:flex-col` — On desktop (md breakpoint): vertical layout (top to bottom)

Each clip item (line 305):
```svelte
class="group min-w-[200px] md:w-full p-3 rounded-lg ..."
```
- `min-w-[200px]` — On mobile: fixed minimum width per clip card (causes horizontal scroll)
- `md:w-full` — On desktop: full width

The "Clip All" button (line 257):
```svelte
class="min-w-[120px] md:w-full py-2 ..."
```
Same pattern — `min-w-[120px]` on mobile, `md:w-full` on desktop.

### ClipList Container (in VideoPlayer)
**File**: `src/lib/VideoPlayer.svelte`  
**Lines**: 288–323

```svelte
<div
    class="{appState.isClipsSidebarOpen
        ? 'h-[50vh] opacity-100 md:h-full md:w-80 border-t md:border-t-0 md:border-l'
        : 'h-0 opacity-0 md:h-full md:opacity-100 md:w-0 border-t-0 md:border-l-0'} border-zinc-900 bg-zinc-950 flex flex-col z-10 transition-all duration-300 overflow-hidden shrink-0"
>
    ...
    <div class="flex-1 w-full md:w-80 overflow-x-auto md:overflow-x-hidden md:overflow-y-auto">
        <ClipList ... />
    </div>
</div>
```

The inner scroll container uses:
- `overflow-x-auto` — On mobile: horizontal scrolling enabled
- `md:overflow-x-hidden md:overflow-y-auto` — On desktop: vertical scrolling

### Dashboard Columns
**File**: `src/lib/Dashboard.svelte`  
**Lines**: 409–414

```svelte
class="
    flex flex-col border-r border-zinc-800 bg-zinc-950/30 overflow-y-auto outline-none
    {depth === columns.length - 1
    ? 'flex-1 w-full min-w-[280px]'
    : 'hidden md:flex w-[280px] min-w-[280px] max-w-[280px]'}
"
```

On mobile, only the **last column** is shown (the deepest selected folder). Previous columns are hidden with `hidden md:flex`. This is actually reasonable Miller column behavior for mobile.

## Changes Required

### Step 1: Change ClipList from horizontal to vertical on mobile

**File**: `src/lib/ClipList.svelte`

**Line 248** — Change the flex direction:
```svelte
<!-- FROM: -->
<div class="flex flex-row md:flex-col gap-2 p-2 h-full">

<!-- TO: -->
<div class="flex flex-col gap-2 p-2 h-full">
```

This makes clips stack vertically on ALL screen sizes.

**Line 305** — Remove the `min-w-[200px]` constraint that forced horizontal card sizing:
```svelte
<!-- FROM: -->
class="group min-w-[200px] md:w-full p-3 rounded-lg bg-zinc-900 border ..."

<!-- TO: -->
class="group w-full p-3 rounded-lg bg-zinc-900 border ..."
```

**Line 257** — Update the "Clip All" button similarly:
```svelte
<!-- FROM: -->
class="min-w-[120px] md:w-full py-2 bg-zinc-800 ..."

<!-- TO: -->
class="w-full py-2 bg-zinc-800 ..."
```

### Step 2: Fix the ClipList scroll container in VideoPlayer

**File**: `src/lib/VideoPlayer.svelte`

**Line 317** — Change the scroll container to always use vertical scrolling:
```svelte
<!-- FROM: -->
<div class="flex-1 w-full md:w-80 overflow-x-auto md:overflow-x-hidden md:overflow-y-auto">

<!-- TO: -->
<div class="flex-1 w-full md:w-80 overflow-y-auto overflow-x-hidden">
```

This ensures clips scroll top-to-bottom on both mobile and desktop.

### Step 3: Fix Dashboard vertical layout for phone

**File**: `src/lib/Dashboard.svelte`

The current mobile approach (showing only the deepest column) is actually good. But there are improvements for better vertical phone UX:

**A) Make the footer bar wrap-friendly**:

**Lines 678–679** — The footer currently uses a fixed `h-12`:
```svelte
<!-- FROM: -->
<div class="h-12 flex items-center justify-between px-4 border-t border-zinc-800 bg-zinc-900 select-none grid-area-footer">

<!-- TO: -->
<div class="min-h-12 flex items-center justify-between px-4 py-2 border-t border-zinc-800 bg-zinc-900 select-none grid-area-footer flex-wrap gap-2">
```

**B) Make the breadcrumb path scrollable on small screens**:

**Lines 700–714** — The breadcrumb path already has `overflow-hidden text-ellipsis whitespace-nowrap` but could benefit from horizontal scroll on mobile:
```svelte
<!-- FROM: -->
<div class="flex gap-1 text-xs text-zinc-400 overflow-hidden text-ellipsis whitespace-nowrap">

<!-- TO: -->
<div class="flex gap-1 text-xs text-zinc-400 overflow-x-auto whitespace-nowrap scrollbar-none">
```

Add to `src/app.css`:
```css
.scrollbar-none::-webkit-scrollbar {
    display: none;
}
.scrollbar-none {
    -ms-overflow-style: none;
    scrollbar-width: none;
}
```

**C) Ensure video items have touch-friendly sizing**:

The current item height is `h-10` (40px). On mobile, this might be too small for touch targets. Consider increasing to `h-12` (48px) which is the minimum recommended touch target:

**Lines 483, 615** — For both folder and video items:
```svelte
<!-- FROM (folder items): -->
class="h-10 px-3 flex items-center ..."

<!-- TO: -->
class="h-12 px-3 flex items-center ..."
```

```svelte
<!-- FROM (video items): -->
class="h-10 px-3 flex items-center ..."

<!-- TO: -->
class="h-12 px-3 flex items-center ..."
```

**This one is optional** — only apply if the user agrees. The `h-10` might be intentionally compact.

**D) Make the Miller columns container take full width on mobile**:

The container already has `flex-1 w-full min-w-[280px]` for the active column. But the Miller columns outer container (line 407) might need adjustment:

```svelte
<!-- FROM: -->
<div class="flex-1 flex overflow-x-auto bg-black border-b border-zinc-900">

<!-- TO: -->
<div class="flex-1 flex overflow-x-auto overflow-y-hidden bg-black border-b border-zinc-900">
```

This prevents double-axis scrolling on mobile which can feel janky.

## Summary of All Changes

| File | Line(s) | Change | Purpose |
|------|---------|--------|---------|
| `ClipList.svelte` | 248 | `flex-row` → `flex-col` | Vertical clip layout on mobile |
| `ClipList.svelte` | 305 | Remove `min-w-[200px]` | Full-width clip cards |
| `ClipList.svelte` | 257 | Remove `min-w-[120px]` | Full-width "Clip All" button |
| `VideoPlayer.svelte` | 317 | `overflow-x-auto` → `overflow-y-auto overflow-x-hidden` | Vertical scrolling for clips |
| `Dashboard.svelte` | 678 | `h-12` → `min-h-12`, add `flex-wrap gap-2` | Footer wrapping on small screens |
| `Dashboard.svelte` | 700 | `overflow-hidden` → `overflow-x-auto scrollbar-none` | Scrollable breadcrumbs |
| `Dashboard.svelte` | 407 | Add `overflow-y-hidden` | Prevent double-axis scroll |
| `app.css` | (new) | `.scrollbar-none` utility | Hide scrollbar on breadcrumbs |
| `Dashboard.svelte` | 483, 615 | `h-10` → `h-12` (optional) | Better touch targets |

## Testing

1. Run `bun run tauri dev` or test on Android device
2. **Clips sidebar**: Open a video → Open clips sidebar → Clips should stack vertically and scroll up/down
3. **Dashboard on narrow screen**: Resize window to phone width (~360px) → Layout should work without horizontal overflow
4. **Breadcrumbs**: Navigate deep into folders → Breadcrumb path should be horizontally scrollable
5. **Touch targets**: Tap folder/video items on mobile → They should be easy to hit
6. **Footer**: All footer buttons should be visible and accessible on narrow screens
