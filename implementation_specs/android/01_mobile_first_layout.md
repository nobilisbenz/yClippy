# Feature 01 — Mobile-First Layout (Top & Bottom Fixes)

## Overview

The current app has a desktop-first layout. On Android, the `TitleBar` is always visible at the top (pushing content down by `pt-10`), but there is no proper bottom system-bar inset handling and no safe-area awareness. The `NativePlayer`'s bottom control bar overflows or gets clipped by the Android navigation bar. This spec covers making both the header and footer layout respect Android's edge-to-edge insets and look great on mobile.

---

## Current State

### `src/App.svelte`
- Detects `isAndroid` via `platform()` from `@tauri-apps/plugin-os`.
- Applies `pt-10` (padding-top 40px) on Android to offset the fixed `TitleBar`.
- The outer `div` uses `h-screen` — this does **not** account for dynamic bottom system bars.

### `src/lib/TitleBar.svelte`
- On Android: always fully visible (`translate-y-0`), centered title, no window controls.
- On desktop: hidden until hover.
- Problem: on Android the title bar is a full-height strip that wastes space. It should either be smaller (compact), or replaced by a lighter status-bar indicator.

### `src/lib/NativePlayer.svelte`
- The bottom control bar uses a hard-coded `h-24` height.
- No `pb-safe` or bottom inset — on phones with gesture navigation, the bar overlaps the system navigation area.

### `src/lib/Dashboard.svelte`
- The footer (library controls) is `min-h-12` with `px-4 py-2` — no bottom-safe inset.
- The Miller Column layout uses `hidden md:flex` to hide all-but-last column on mobile — this is correct behavior.

---

## Goals

1. **Top layout**: Replace the always-on slim `TitleBar` on Android with a proper edge-to-edge approach using `WindowInsets`. The status bar area should be transparent and the content should start below it naturally.
2. **Bottom layout (NativePlayer)**: Bottom control bar must sit above the Android navigation bar using `WindowInsetsCompat` or CSS env() variables.
3. **Bottom layout (Dashboard)**: The library footer controls must also be inset-aware.
4. **No regression on desktop** — all desktop layout logic must remain the same.

---

## Detailed Implementation Steps

### Step 1 — Enable Edge-to-Edge in `MainActivity.kt`

`enableEdgeToEdge()` is already called on line 26. Verify that the theme in `res/values/themes.xml` has:

```xml
<item name="android:windowTranslucentStatus">false</item>
<item name="android:windowTranslucentNavigation">false</item>
<item name="android:statusBarColor">@android:color/transparent</item>
<item name="android:navigationBarColor">@android:color/transparent</item>
```

This allows the app to draw behind both status and navigation bars. The `enableEdgeToEdge()` API (AndroidX `1.7+`) handles most of this automatically.

**File**: `src-tauri/gen/android/app/src/main/res/values/themes.xml`

---

### Step 2 — Inject CSS Safe-Area Variables via `MainActivity.kt`

After the WebView is ready (inside `onWebViewCreate`), inject JavaScript to set CSS custom properties based on `WindowInsetsCompat`. This is the bridge between Android insets and the Svelte CSS.

Add this helper method to `MainActivity.kt`:

```kotlin
private fun applyWindowInsets() {
    val rootView = window.decorView

    ViewCompat.setOnApplyWindowInsetsListener(rootView) { _, insets ->
        val systemBars = insets.getInsets(WindowInsetsCompat.Type.systemBars())
        val statusBarHeight = systemBars.top   // e.g. 74px
        val navBarHeight   = systemBars.bottom // e.g. 126px

        myWebView?.evaluateJavascript("""
            document.documentElement.style.setProperty('--safe-top',    '${statusBarHeight}px');
            document.documentElement.style.setProperty('--safe-bottom',  '${navBarHeight}px');
        """.trimIndent(), null)

        insets
    }
}
```

Call `applyWindowInsets()` at the end of `onWebViewCreate`. Also add the needed import:

```kotlin
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
```

---

### Step 3 — Update `app.css` to Consume Safe-Area Variables

Add fallback defaults for desktop (where the variables are not set):

```css
:root {
  --safe-top: 0px;
  --safe-bottom: 0px;
}
```

**File**: `src/app.css`

---

### Step 4 — Update `App.svelte` Root Layout

Replace the hard-coded `pt-10` for Android with the CSS variable:

```svelte
<div
  class="flex w-full bg-black text-white font-sans overflow-hidden select-none"
  style="height: calc(100vh - var(--safe-top)); padding-top: var(--safe-top);"
>
```

Remove the current `h-screen` and the `{isAndroid ? 'pt-10' : 'peer-hover:pt-10'}` logic — the CSS variable is zero on desktop, so it degrades gracefully.

**File**: `src/App.svelte`

---

### Step 5 — Update `TitleBar.svelte` for Android

On Android, the title bar should be **hidden entirely** since edge-to-edge mode with transparent status bar handles the visual separation. We only want the content to start below `--safe-top` which is already handled in Step 4.

```svelte
<!-- Only show full TitleBar on desktop -->
{#if !isAndroid}
  <!-- existing TitleBar markup ... -->
{/if}
```

Or alternatively, keep a very minimal 0-height marker div on Android (for the `peer` CSS trick used on desktop).

**File**: `src/lib/TitleBar.svelte`

---

### Step 6 — Update `NativePlayer.svelte` Bottom Bar

The current bottom control bar:
```svelte
<div class="h-24 border-t border-zinc-900 bg-zinc-950 p-4 flex items-center justify-between z-10 shrink-0">
```

Change to:
```svelte
<div
  class="border-t border-zinc-900 bg-zinc-950 p-4 flex items-center justify-between z-10 shrink-0"
  style="padding-bottom: calc(1rem + var(--safe-bottom));"
>
```

This adds the safe bottom inset on top of the existing `1rem` padding so the buttons are never hidden under gesture navigation handles.

**File**: `src/lib/NativePlayer.svelte`

---

### Step 7 — Update `Dashboard.svelte` Footer Bar

Same treatment for the library controls footer:

```svelte
<div
  class="min-h-12 flex items-center justify-between px-4 border-t border-zinc-800 bg-zinc-900 select-none grid-area-footer flex-wrap gap-2"
  style="padding-bottom: calc(0.5rem + var(--safe-bottom)); padding-top: 0.5rem;"
>
```

**File**: `src/lib/Dashboard.svelte`

---

### Step 8 — Verify `ClipList` Sidebar on Mobile

In `NativePlayer.svelte`, the clips sidebar:
```svelte
<div class="w-full md:w-80 border-l border-zinc-900 bg-zinc-950 shrink-0 overflow-y-auto">
```

On mobile (`w-full`), when the sidebar opens it should overlay the player as a full-width slide-in. Consider adding:
- `absolute inset-0 z-40` when on Android to make it an overlay rather than shifting content.
- A close button at the top of the sidebar for touch users.

---

## Files to Modify

| File | Change |
|------|--------|
| `src-tauri/gen/android/app/src/main/res/values/themes.xml` | Ensure edge-to-edge theme values |
| `src-tauri/gen/android/app/src/main/java/com/yclippy/app/MainActivity.kt` | Add `applyWindowInsets()`, inject CSS vars into WebView |
| `src/app.css` | Add `--safe-top` and `--safe-bottom` CSS variable defaults |
| `src/App.svelte` | Replace `h-screen + pt-10` with CSS var-based height & padding |
| `src/lib/TitleBar.svelte` | Hide TitleBar on Android (edge-to-edge handles status bar) |
| `src/lib/NativePlayer.svelte` | Add `padding-bottom: var(--safe-bottom)` to control bar |
| `src/lib/Dashboard.svelte` | Add `padding-bottom: var(--safe-bottom)` to footer bar |

---

## Verification

1. Build and run on a device with gesture navigation (Android 10+) — verify no button is hidden under nav bar.
2. Build and run on a device with 3-button navigation — verify no extra whitespace at the bottom.
3. Test on desktop — verify layout is pixel-identical to before (`--safe-top` and `--safe-bottom` will be `0px`).
4. Check both portrait and landscape orientations.
