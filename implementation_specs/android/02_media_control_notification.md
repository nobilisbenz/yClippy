# Feature 02 — Media Control Notification (MediaSession + Rich Notification)

## Overview

When the user plays a video in yClippy on Android, they should see a proper **media control notification** in the notification shade — with the video title, thumbnail, and **Play / Pause / Stop** buttons. This notification should also surface on the **lock screen** and in **quick settings**. The control buttons in the notification must work even when the screen is off.

---

## Current State

### `NativePlayerService.kt`
- Already extends `MediaSessionService` (from `androidx.media3:media3-session`).
- Already creates an `ExoPlayer` and a `MediaSession`.
- Already calls `startForeground()` with a basic notification.
- **Problem**: The current notification uses `NotificationCompat.Builder` manually with raw `addAction()` calls — it does **not** use `MediaStyleNotification`, so Android's system music-player UI and lock-screen transport controls are not shown.
- The `MediaSession.Callback` is nearly empty — it doesn't handle `onPlay`, `onPause`, or `onSkipToNext` from the notification.

### `AudioPlaybackService.kt`
- A separate, simpler foreground service using a `PARTIAL_WAKE_LOCK` and a non-interactive notification.
- This service is used when the app goes to background to keep the WebView alive.
- This is for the YouTube iframe player path, not the ExoPlayer path.

### `build.gradle.kts`
- Already has `media3-exoplayer:1.5.0`, `media3-session:1.5.0`, `media3-ui:1.5.0`.
- No changes needed in dependencies.

---

## Goals

1. Replace the manually-built notification in `NativePlayerService` with a proper **`MediaStyle` notification** using `MediaSession`.
2. Show **album art** (thumbnail) in the notification.
3. Show **Play, Pause, Stop** action buttons that actually control the `ExoPlayer`.
4. Integrate with Android's **system media controls** (lock screen, quick settings tile).
5. Feed back play/pause state changes to the Svelte frontend via the existing `yClippyNative` JS bridge.

---

## Detailed Implementation Steps

### Step 1 — Upgrade `NativePlayerService` to Use `DefaultMediaNotificationProvider`

Media3's `MediaSessionService` has a built-in `DefaultMediaNotificationProvider` that automatically creates a `MediaStyle` notification from the `MediaSession`. Enable it properly:

```kotlin
// In NativePlayerService.onCreate():
override fun onCreate() {
    super.onCreate()
    createNotificationChannel()
    initializePlayer()

    // This replaces all manual notification building:
    setMediaNotificationProvider(
        DefaultMediaNotificationProvider.Builder(this)
            .setNotificationId(NOTIFICATION_ID)
            .setChannelId(CHANNEL_ID)
            .build()
    )
}
```

Add import:
```kotlin
import androidx.media3.session.DefaultMediaNotificationProvider
```

With this, the `MediaSessionService` base class handles showing/updating the notification automatically whenever player state changes. **Remove** the manual `createNotification()` call and `startForeground()` from `playVideo()` — Media3 calls `startForeground` internally via the notification provider.

---

### Step 2 — Set Rich MediaMetadata on the MediaItem

When starting playback, include the video title and thumbnail URI in `MediaMetadata` so the notification and lock screen show this information:

```kotlin
private fun playVideo(url: String, title: String, thumbnailUrl: String?, startPosition: Long) {
    player?.let { exoPlayer ->
        val artworkUri = thumbnailUrl?.let { Uri.parse(it) }

        val mediaItem = MediaItem.Builder()
            .setUri(url)
            .setMediaMetadata(
                MediaMetadata.Builder()
                    .setTitle(title)
                    .setArtist("yClippy")
                    .setArtworkUri(artworkUri)    // <-- thumbnail shown in notification
                    .build()
            )
            .build()

        exoPlayer.setMediaItem(mediaItem)
        exoPlayer.prepare()
        exoPlayer.seekTo(startPosition)
        exoPlayer.play()
        isPlaying = true

        // handler for position tracking (keep existing)
        handler.post(updateRunnable)
    }
}
```

Add import:
```kotlin
import android.net.Uri
```

---

### Step 3 — Add a `Player.Listener` to Forward State Changes Back to the Frontend

When the user taps Play/Pause in the notification (or from the lock screen), we need to notify the Svelte frontend so its UI can update `isPaused` state in `NativePlayer.svelte`.

Add a `Player.Listener` inside `initializePlayer()` in `NativePlayerService`:

```kotlin
player?.addListener(object : Player.Listener {
    override fun onIsPlayingChanged(isPlaying: Boolean) {
        // Broadcast an Intent that MainActivity can pick up
        val broadcastIntent = Intent("com.yclippy.app.PLAYBACK_STATE_CHANGED")
        broadcastIntent.putExtra("isPlaying", isPlaying)
        sendBroadcast(broadcastIntent)
    }
})
```

---

### Step 4 — Register a `BroadcastReceiver` in `MainActivity` to Forward State to JS

In `MainActivity.kt`, register a `BroadcastReceiver` that listens for the `PLAYBACK_STATE_CHANGED` broadcast and fires a JavaScript callback:

```kotlin
private val playbackStateReceiver = object : BroadcastReceiver() {
    override fun onReceive(context: Context?, intent: Intent?) {
        val isPlaying = intent?.getBooleanExtra("isPlaying", false) ?: false
        val jsCallback = if (isPlaying) "onPlayerResumed()" else "onPlayerPaused()"
        myWebView?.evaluateJavascript(
            "if (window.yClippyCallbacks) { window.yClippyCallbacks.$jsCallback; }",
            null
        )
    }
}

override fun onResume() {
    super.onResume()
    registerReceiver(
        playbackStateReceiver,
        IntentFilter("com.yclippy.app.PLAYBACK_STATE_CHANGED"),
        Context.RECEIVER_NOT_EXPORTED
    )
}

override fun onPause() {
    super.onPause()
    unregisterReceiver(playbackStateReceiver)
    // ... rest of existing onPause
}
```

Add imports:
```kotlin
import android.content.BroadcastReceiver
import android.content.Context
import android.content.IntentFilter
```

---

### Step 5 — Expose a `yClippyCallbacks` Object in `NativePlayer.svelte`

In `NativePlayer.svelte`, expose a global callbacks object that `MainActivity` can call from the BroadcastReceiver:

```typescript
onMount(async () => {
    // ... existing mount code ...

    // Expose callback object for native → JS communication
    (window as any).yClippyCallbacks = {
        onPlayerPaused: () => { isPaused = true; },
        onPlayerResumed:  () => { isPaused = false; },
    };
});

onDestroy(async () => {
    delete (window as any).yClippyCallbacks;
    // ... rest of existing cleanup ...
});
```

This makes the `isPaused` Svelte reactive state in sync with the actual `ExoPlayer` state even when the user controls playback from the notification.

---

### Step 6 — Handle `MediaSession.Callback` Actions in `NativePlayerService`

Override the `onConnect` callback to allow the notification controls to work. Media3's `DefaultMediaNotificationProvider` generates Play/Pause/Stop buttons automatically based on the `Player.Commands` available. Ensure `ExoPlayer` has the right commands:

```kotlin
// In initializePlayer():
player = ExoPlayer.Builder(this)
    .build()
    .also { exo ->
        exo.addListener(/* listener from Step 3 */)
    }
```

Media3's `ExoPlayer` already supports `COMMAND_PLAY_PAUSE` and `COMMAND_STOP` by default, so the notification buttons work automatically with `DefaultMediaNotificationProvider`.

---

### Step 7 — Ensure the Notification Channel Has Correct Settings

Update `createNotificationChannel()` to use `IMPORTANCE_DEFAULT` (needed for lock screen visibility):

```kotlin
private fun createNotificationChannel() {
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
        val channel = NotificationChannel(
            CHANNEL_ID,
            "Now Playing",
            NotificationManager.IMPORTANCE_DEFAULT  // was IMPORTANCE_LOW
        ).apply {
            description = "yClippy media playback controls"
            setShowBadge(false)
            lockscreenVisibility = Notification.VISIBILITY_PUBLIC  // show on lock screen
        }
        val manager = getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(channel)
    }
}
```

Add import:
```kotlin
import android.app.Notification
```

---

### Step 8 — Add `POST_NOTIFICATIONS` Permission for Android 13+

In `AndroidManifest.xml`, add:

```xml
<uses-permission android:name="android.permission.POST_NOTIFICATIONS" />
```

In `MainActivity.kt`, request this permission at runtime on Android 13+ (API 33+):

```kotlin
import android.Manifest
import android.content.pm.PackageManager
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat

// In onCreate(), after super.onCreate():
if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
    if (ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS)
        != PackageManager.PERMISSION_GRANTED) {
        ActivityCompat.requestPermissions(
            this,
            arrayOf(Manifest.permission.POST_NOTIFICATIONS),
            REQUEST_NOTIFICATION_PERMISSION
        )
    }
}

companion object {
    private const val REQUEST_NOTIFICATION_PERMISSION = 42
}
```

---

## Files to Modify

| File | Change |
|------|--------|
| `src-tauri/gen/android/app/src/main/AndroidManifest.xml` | Add `POST_NOTIFICATIONS` permission |
| `src-tauri/gen/android/app/src/main/java/com/yclippy/app/NativePlayerService.kt` | Use `DefaultMediaNotificationProvider`, set `MediaMetadata` with artwork, add `Player.Listener` broadcast |
| `src-tauri/gen/android/app/src/main/java/com/yclippy/app/MainActivity.kt` | Add `BroadcastReceiver` for playback state, request `POST_NOTIFICATIONS` permission |
| `src/lib/NativePlayer.svelte` | Expose `window.yClippyCallbacks` object for native → JS state sync |

---

## Verification

1. Start playing a video — swipe down notification shade — verify a rich media notification appears with title, thumbnail, Play/Pause/Stop buttons.
2. Lock the screen while playing — verify media controls appear on the lock screen.
3. Tap **Pause** in the notification — swipe back to the app — verify the UI correctly shows the paused state (play icon visible).
4. Tap **Play** in the notification — verify playback resumes and the app UI shows the playing state.
5. Tap **Stop** in the notification — verify the notification dismisses and the service stops cleanly.
