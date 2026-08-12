# Feature 1: Background Audio Playback (Screen-Off / Standby)

## Problem
When the phone screen is turned off or the app goes to standby, YouTube video audio stops playing. The user wants to continue listening to the video audio even when the screen is off.

## Root Cause
The YouTube IFrame API embeds a web-based `<iframe>` player. On Android, when the app goes to the background or the screen is locked, the WebView suspends execution including media playback. This is standard Android behavior — WebViews pause when not visible.

## Solution Strategy

There are **two approaches**, ranked by feasibility:

### Approach A: Android Media Session + WebView Keep-Alive (Recommended)

This approach keeps the WebView alive during screen-off by using Android's foreground service and wake lock mechanisms through Tauri's plugin system.

#### Step 1: Add the `tauri-plugin-notification` dependency

**File**: `src-tauri/Cargo.toml`

Add to `[dependencies]`:
```toml
tauri-plugin-notification = "2"
```

This is needed to create a persistent notification that keeps the foreground service alive.

#### Step 2: Configure Android permissions

**File**: `src-tauri/gen/android/app/src/main/AndroidManifest.xml`

Add these permissions inside `<manifest>` (before `<application>`):
```xml
<uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
<uses-permission android:name="android.permission.FOREGROUND_SERVICE_MEDIA_PLAYBACK" />
<uses-permission android:name="android.permission.WAKE_LOCK" />
```

Add foreground service type to the main activity or create a service:
```xml
<service
    android:name=".AudioPlaybackService"
    android:foregroundServiceType="mediaPlayback"
    android:exported="false" />
```

#### Step 3: Create a Rust Tauri command for wake lock management

**File**: `src-tauri/src/lib.rs`

Add a new module and Tauri commands:
```rust
mod audio_service;
```

Register new commands in the invoke handler:
```rust
audio_service::acquire_wake_lock,
audio_service::release_wake_lock,
```

**File**: `src-tauri/src/audio_service.rs` (NEW)

Create Tauri commands that interact with the Android platform:
```rust
use tauri::AppHandle;

#[tauri::command]
pub async fn acquire_wake_lock(app: AppHandle) -> Result<(), String> {
    // Use tauri's mobile plugin system to call Android Java code
    // This sends an intent to start the foreground service
    #[cfg(target_os = "android")]
    {
        app.run_on_android_context(|env, activity, _webview| {
            // Call Java method to start foreground service with wake lock
        });
    }
    Ok(())
}

#[tauri::command]
pub async fn release_wake_lock(app: AppHandle) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        app.run_on_android_context(|env, activity, _webview| {
            // Call Java method to stop foreground service
        });
    }
    Ok(())
}
```

#### Step 4: Create Android Kotlin/Java Service

**File**: `src-tauri/gen/android/app/src/main/java/com/yclippy/app/AudioPlaybackService.kt` (NEW)

```kotlin
package com.yclippy.app

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Intent
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import androidx.core.app.NotificationCompat

class AudioPlaybackService : Service() {
    private var wakeLock: PowerManager.WakeLock? = null
    
    companion object {
        const val CHANNEL_ID = "yclippy_audio_channel"
        const val NOTIFICATION_ID = 1
    }

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
        val notification = createNotification()
        startForeground(NOTIFICATION_ID, notification)
        acquireWakeLock()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onDestroy() {
        releaseWakeLock()
        super.onDestroy()
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "Audio Playback",
                NotificationManager.IMPORTANCE_LOW
            )
            val manager = getSystemService(NotificationManager::class.java)
            manager.createNotificationChannel(channel)
        }
    }

    private fun createNotification(): Notification {
        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("yClippy")
            .setContentText("Playing audio in background")
            .setSmallIcon(android.R.drawable.ic_media_play)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .build()
    }

    private fun acquireWakeLock() {
        val pm = getSystemService(POWER_SERVICE) as PowerManager
        wakeLock = pm.newWakeLock(
            PowerManager.PARTIAL_WAKE_LOCK,
            "yclippy:audio_playback"
        )
        wakeLock?.acquire()
    }

    private fun releaseWakeLock() {
        wakeLock?.release()
        wakeLock = null
    }
}
```

#### Step 5: Add WebView settings for background media

**File**: `src-tauri/gen/android/app/src/main/java/com/yclippy/app/MainActivity.kt`

Override the WebView configuration to allow media playback in background:
```kotlin
// In the WebView configuration, add:
webView.settings.mediaPlaybackRequiresUserGesture = false
// Keep WebView rendering even when not visible:
webView.settings.setOffscreenPreRaster(true)
```

#### Step 6: Frontend integration — call wake lock on play

**File**: `src/lib/VideoPlayer.svelte`

Add these imports and functions:
```typescript
import { invoke } from "@tauri-apps/api/core";
import { platform } from "@tauri-apps/plugin-os";

const isAndroid = platform() === "android";
```

In `onPlayerStateChange`, call wake lock based on player state:
```typescript
function onPlayerStateChange(event: any) {
    if (player && player.getCurrentTime) {
        currentTime = player.getCurrentTime();
    }

    if (event.data === 2) {
        isPaused = true;
        if (isAndroid) invoke("release_wake_lock").catch(console.error);
    } else if (event.data === 1) {
        isPaused = false;
        if (isAndroid) invoke("acquire_wake_lock").catch(console.error);
    } else {
        isPaused = false;
    }
}
```

In `onDestroy`, release the wake lock:
```typescript
onDestroy(async () => {
    clearInterval(timer);
    if (isAndroid) invoke("release_wake_lock").catch(console.error);
    if (video) {
        video.last_position = Math.floor(currentTime);
        saveVideo(video).then(() => appState.refreshVideos());
    }
});
```

### Approach B: Alternative — Use `cordova-plugin-background-mode` via Capacitor bridge

If Approach A is too complex, consider using `@niceplugins/capacitor-background-mode` or writing a simpler Tauri plugin that just toggles Android's `FLAG_KEEP_SCREEN_ON` combined with dimming to black, which is less ideal but simpler.

## Testing

1. Build the Android APK: `bun run tauri android build`
2. Install on a physical Android device
3. Open a video and start playing
4. Lock the screen → audio should continue playing
5. A notification should appear: "yClippy — Playing audio in background"
6. When video is paused or user navigates back, the notification should disappear

## Important Notes

- This feature is **Android-only**. On desktop (Linux), screen-off behavior is handled by the OS and the app stays running.
- The `gen/android/` directory is auto-generated by Tauri — regenerate it with `bun run tauri android init` if it doesn't exist, then apply the modifications.
- Wake locks MUST be properly released to avoid battery drain. The `onDestroy` cleanup is critical.
