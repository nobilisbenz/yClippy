# Feature 03 — Background Playback (Screen Off + Auto-Next Video)

## Overview — The Core Problem

The **YouTube iframe player** (`YouTubePlayerView` from `pierfrancescosoffritti/android-youtube-player`) **stops playback when the screen turns off**. Calling `enableBackgroundPlayback(true)` on `YouTubePlayerView` does NOT bypass YouTube's own page-level restriction — it only tries to keep the player alive across Activity lifecycle events. The playback reliably stops on screen-off.

The correct workaround is **Path A**: extract the real MP4/WebM stream URL using `yt-dlp` and feed it to `ExoPlayer`, which is a native Android media player with no such restriction.

---

## Does yt-dlp Work on Android? Yes. ✅

`yt-dlp` runs perfectly on Android. There is a mature Android library, **`youtubedl-android`** (maintained by `junkfood02`, forked from `yausername`), that:

- **Bundles the `yt-dlp` binary and Python 3.8** inside the AAR — no manual binary downloads needed.
- Supports **ARM64-v8a and armeabi-v7a** (covers all modern Android phones).
- Supports **Android API 21+** (Lollipop).
- Ships as a standard Maven artifact — just add a dependency.
- Provides a **Kotlin-friendly API** to get video info and extract a single playable URL.
- Can self-update the bundled `yt-dlp` binary at runtime.

**Real apps using this library**: YTDLnis (500k+ downloads on GitHub, actively maintained as of 2026), dvd (another popular yt-dlp frontend for Android).

> **Note on YouTube ToS**: Using `yt-dlp` to stream YouTube content violates YouTube's Terms of Service. This approach is fine for sideloaded / F-Droid distributed builds, personal use, or use in regions where such restrictions don't apply legally. Do not publish this to the Google Play Store without legal review.

---

## Path A — yt-dlp + ExoPlayer (Full Implementation)

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│  App Foreground                                                  │
│  YouTubePlayerView (renders video) → tracks currentPosition     │
└────────────────────────┬────────────────────────────────────────┘
                         │ User locks screen / presses Home
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│  MainActivity.onPause()                                          │
│   1. Read currentPosition from YouTubePlayerView                │
│   2. Call YoutubeDLExtractor.getStreamUrl(videoId) [coroutine]  │
│   3. Start NativePlayerService (ExoPlayer) with real stream URL  │
│      + playlist JSON for auto-next                              │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│  NativePlayerService (already implemented as MediaSessionService)│
│   - ExoPlayer plays the real MP4 stream                         │
│   - Shows rich media notification (Play/Pause/Stop)             │
│   - Auto-advances queue: each item's URL extracted on-demand    │
│   - Persists position to SharedPreferences every second         │
└────────────────────────┬────────────────────────────────────────┘
                         │ User returns to app
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│  MainActivity.onResume()                                         │
│   1. Read last position from SharedPreferences                  │
│   2. Stop NativePlayerService                                   │
│   3. YouTubePlayerView.seekTo(lastPosition) + play()            │
└─────────────────────────────────────────────────────────────────┘
```

---

## Step-by-Step Implementation

---

### STEP 1 — Add `youtubedl-android` Library

**File**: `src-tauri/gen/android/app/build.gradle.kts`

Add the JitPack repository in the project-level `settings.gradle`:

**File**: `src-tauri/gen/android/settings.gradle`
```groovy
dependencyResolutionManagement {
    repositories {
        google()
        mavenCentral()
        maven { url = uri("https://jitpack.io") }
    }
}
```

Then in the **app-level** `build.gradle.kts`, add the dependency:

```kotlin
dependencies {
    // ... existing dependencies ...

    // youtubedl-android: bundles yt-dlp binary + Python 3.8 for ARM/ARM64
    val youtubedlAndroid = "0.18.1"
    implementation("io.github.junkfood02.youtubedl-android:library:$youtubedlAndroid")
    // Note: do NOT include the :ffmpeg module unless you need ffmpeg post-processing
    // (it adds ~30MB). For stream URL extraction only, :library is enough.

    // Coroutines (needed for async extraction)
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.7.3")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.7.0") // lifecycleScope
}
```

Also add to `android {}` block in `build.gradle.kts`:

```kotlin
android {
    // ... existing config ...
    defaultConfig {
        // ... existing config ...

        // Required by youtubedl-android — specify which ABIs to include
        ndk {
            abiFilters += listOf("arm64-v8a", "armeabi-v7a")
        }
    }
}
```

And add to `AndroidManifest.xml` under `<application>`:

```xml
<application
    android:extractNativeLibs="true"   <!-- Required by youtubedl-android -->
    ...>
```

---

### STEP 2 — Initialize `YoutubeDL` in `MainActivity`

`youtubedl-android` must be initialized once before any extraction calls. Do it in `onCreate()`:

**File**: `src-tauri/gen/android/app/src/main/java/com/yclippy/app/MainActivity.kt`

```kotlin
import com.yausername.youtubedl_android.YoutubeDL
import com.yausername.youtubedl_android.YoutubeDLException

override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    // Initialize yt-dlp (extracts bundled binary to internal storage on first run)
    // This is fast on subsequent launches since the binary is cached
    lifecycleScope.launch(Dispatchers.IO) {
        try {
            YoutubeDL.getInstance().init(applicationContext)
            android.util.Log.d("YoutubeDL", "yt-dlp initialized successfully")
        } catch (e: YoutubeDLException) {
            android.util.Log.e("YoutubeDL", "Failed to initialize yt-dlp: ${e.message}")
        }
    }
}
```

Add import:
```kotlin
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
```

**On first app launch**, this copies the bundled `yt-dlp` binary (~10MB) from the AAR to `context.filesDir`. Subsequent launches reuse the cached binary. To show a loading indicator during first launch, you can add a `StateFlow` and observe it in the UI.

---

### STEP 3 — Create `YtDlpExtractor.kt` (Stream URL Extractor)

Create a new Kotlin object that uses `youtubedl-android` to extract the best single-file stream URL for any YouTube video:

**File**: `src-tauri/gen/android/app/src/main/java/com/yclippy/app/YtDlpExtractor.kt`

```kotlin
package com.yclippy.app

import com.yausername.youtubedl_android.YoutubeDL
import com.yausername.youtubedl_android.YoutubeDLRequest
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

object YtDlpExtractor {

    data class StreamInfo(
        val url: String,
        val title: String,
        val thumbnailUrl: String?,
        val durationSeconds: Long
    )

    /**
     * Extracts the best single-file (audio+video combined) stream URL for a YouTube video.
     * Uses yt-dlp's "-f best" option to get a progressive MP4 stream that ExoPlayer can play.
     *
     * @param videoUrl Full YouTube URL, e.g. "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
     * @return StreamInfo with the direct stream URL, or null if extraction failed
     */
    suspend fun getStreamInfo(videoUrl: String): StreamInfo? = withContext(Dispatchers.IO) {
        try {
            val request = YoutubeDLRequest(videoUrl).apply {
                // Best single-file: audio+video in one stream (progressive MP4)
                // This avoids needing to merge separate audio/video tracks (no ffmpeg needed)
                addOption("-f", "best[ext=mp4]/best")

                // Get metadata + URL without downloading
                addOption("--dump-json")

                // Skip playlists, we handle individual videos
                addOption("--no-playlist")

                // Skip age verification prompts
                addOption("--no-check-certificate")

                // Use Android's temp dir for cookies/cache
                addOption("--cache-dir", "/tmp")
            }

            val info = YoutubeDL.getInstance().getInfo(request)

            if (info?.url != null) {
                android.util.Log.d("YtDlpExtractor", "Extracted stream for: ${info.title}")
                StreamInfo(
                    url          = info.url,
                    title        = info.title ?: "Unknown",
                    thumbnailUrl = info.thumbnail,
                    durationSeconds = info.duration?.toLong() ?: 0L
                )
            } else {
                android.util.Log.w("YtDlpExtractor", "No stream URL returned for: $videoUrl")
                null
            }
        } catch (e: Exception) {
            android.util.Log.e("YtDlpExtractor", "Extraction failed: ${e.message}")
            null
        }
    }

    /**
     * Convenience overload for YouTube video IDs (e.g., "dQw4w9WgXcQ")
     */
    suspend fun getStreamInfoById(videoId: String): StreamInfo? {
        val url = if (videoId.startsWith("http")) videoId
                  else "https://www.youtube.com/watch?v=$videoId"
        return getStreamInfo(url)
    }
}
```

**How it works internally**:
- `YoutubeDL.getInstance().getInfo(request)` runs the bundled `yt-dlp` binary as a subprocess.
- `-f best[ext=mp4]/best` selects the best combined mp4 stream available (no separate audio+video tracks to merge, so ffmpeg is not needed).
- The call takes roughly **1–3 seconds** depending on network and device speed. Always call it from a coroutine / background thread.
- The returned `info.url` is a valid, directly-playable HTTPS URL that ExoPlayer can open.

> **Stream URL Expiry**: YouTube stream URLs expire after ~6 hours. For our use case (background playback session), this is more than enough. If a user leaves a video running for >6 hours, ExoPlayer will naturally error — handle this by calling `YtDlpExtractor.getStreamInfoById()` again.

---

### STEP 4 — Track Current Video in `MainActivity`

Add state to `MainActivity` to track which video is currently active (needed for the handoff):

**File**: `MainActivity.kt`

```kotlin
// Add these to MainActivity class:
private var currentVideoUrl: String? = null
private var currentVideoTitle: String? = null
private var currentVideoThumbnail: String? = null
```

In `YClippyNative`, expose a method that the Svelte frontend calls when a video starts:

```kotlin
// Add to YClippyNative class:
@JavascriptInterface
fun notifyVideoStarted(videoUrl: String, title: String, thumbnail: String) {
    activity.currentVideoUrl = videoUrl
    activity.currentVideoTitle = title
    activity.currentVideoThumbnail = thumbnail.ifEmpty { null }
}
```

In `NativePlayer.svelte`, call this when mounting:

```typescript
onMount(async () => {
    const startPos = Math.max(video.last_position, video.start_time);
    const native = window as any;

    if (native.yClippyNative) {
        // Notify native side so it knows what's playing (needed for background handoff)
        native.yClippyNative.notifyVideoStarted(
            video.id,          // This is the YouTube video ID or full URL
            video.title,
            video.thumbnail_url ?? ""
        );
        native.yClippyNative.showYouTubePlayerByUrl(video.id, startPos);
        native.yClippyNative.keepScreenOn();
    }

    isLoading = false;
    startPositionTracking();
});
```

---

### STEP 5 — Implement the Background Handoff in `MainActivity.onPause()`

This is the core of the feature. When the app goes to background:

1. Get the current playback position from `YouTubePlayerView`.
2. Extract the real stream URL using `YtDlpExtractor` (async).
3. Start `NativePlayerService` (ExoPlayer) with the stream URL.

**File**: `MainActivity.kt`

```kotlin
override fun onPause() {
    super.onPause()
    myWebView?.onResume()    // Keep WebView timers alive
    myWebView?.resumeTimers()

    val videoUrl = currentVideoUrl ?: return // Nothing playing, skip

    // Start background handoff:
    // We launch a coroutine that extracts the stream URL while the service starts.
    // If extraction takes time, ExoPlayer will be idle until the URL is ready.
    lifecycleScope.launch {
        android.util.Log.d("MainActivity", "Starting background handoff for: $videoUrl")

        val positionSeconds = currentPosition  // currentPosition is updated by YouTubePlayerView
        val positionMs = (positionSeconds * 1000).toLong()

        val streamInfo = YtDlpExtractor.getStreamInfoById(videoUrl)

        if (streamInfo != null) {
            runOnUiThread {
                android.util.Log.d("MainActivity", "Got stream URL, starting ExoPlayer service")
                startNativePlayer(
                    url          = streamInfo.url,
                    title        = currentVideoTitle ?: streamInfo.title,
                    thumbnailUrl = currentVideoThumbnail ?: streamInfo.thumbnailUrl,
                    startPosition = positionMs
                )
            }
        } else {
            android.util.Log.w("MainActivity", "Stream extraction failed, using wake lock fallback")
            // Fallback: keep the wake lock service alive so the WebView doesn't die completely
            runOnUiThread { ensureAudioServiceRunning() }
        }
    }
}
```

---

### STEP 6 — Return from Background: `MainActivity.onResume()`

When the user returns to the app, sync position back from ExoPlayer → YouTubePlayerView:

**File**: `MainActivity.kt`

```kotlin
override fun onResume() {
    super.onResume()

    if (nativePlayerIntent != null) {
        // We were playing in background via ExoPlayer.
        // Read the last persisted position and resume YouTubePlayerView from there.
        val prefs = getSharedPreferences("yclippy_player", MODE_PRIVATE)
        val lastPositionMs = prefs.getLong("last_position_ms", 0L)
        val lastPositionSec = lastPositionMs / 1000f

        android.util.Log.d("MainActivity", "Resuming from background at ${lastPositionSec}s")

        stopNativePlayer()
        youTubePlayer?.seekTo(lastPositionSec)
        youTubePlayer?.play()
    }
}
```

---

### STEP 7 — Persist Position in `NativePlayerService` Every Second

The existing `updateRunnable` already runs every second. Add SharedPreferences writing:

**File**: `NativePlayerService.kt`

```kotlin
private val updateRunnable = object : Runnable {
    override fun run() {
        player?.let { exo ->
            currentPosition = exo.currentPosition
            duration = exo.duration

            // Persist so MainActivity can sync position on resume
            getSharedPreferences("yclippy_player", MODE_PRIVATE)
                .edit()
                .putLong("last_position_ms", currentPosition)
                .apply()
        }
        handler.postDelayed(this, 1000)
    }
}
```

---

### STEP 8 — Auto-Next: Serialize Video Queue from Svelte to Native

Before going to background, the Svelte frontend passes the full video playlist as JSON so `NativePlayerService` can auto-advance when one video ends.

**Step 8a — Expose playlist from `NativePlayer.svelte`**:

```typescript
// Add to the yClippyCallbacks object exposed in onMount():
(window as any).yClippyCallbacks = {
    onPlayerPaused:  () => { isPaused = true; },
    onPlayerResumed: () => { isPaused = false; },

    // Returns JSON array of the current folder's video queue
    getQueueJson: () => {
        // Get all videos in the same folder as the current video, sorted by sort_order
        const queue = appState.videos
            .filter(v => v.folder_id === video.folder_id)
            .sort((a, b) => a.sort_order - b.sort_order)
            .map(v => ({
                id:    v.id,           // YouTube video ID or URL
                title: v.title,
                thumb: v.thumbnail_url ?? "",
                start: v.start_time ?? 0,   // user-defined clip start in seconds
            }));
        return JSON.stringify(queue);
    }
};
```

**Step 8b — Read queue in `MainActivity.onPause()` before starting the service**:

```kotlin
override fun onPause() {
    super.onPause()
    myWebView?.onResume()
    myWebView?.resumeTimers()

    val videoUrl = currentVideoUrl ?: return

    // 1. Get the playlist JSON from JS
    myWebView?.evaluateJavascript(
        "window.yClippyCallbacks?.getQueueJson() || '[]'"
    ) { queueJson ->
        // queueJson comes back with JSON string quotes, strip them:
        val cleanJson = queueJson.trim('"').replace("\\\"", "\"").replace("\\\\", "\\")

        lifecycleScope.launch {
            val positionSeconds = currentPosition
            val positionMs = (positionSeconds * 1000).toLong()

            // 2. Find the index of current video in the queue
            val queueArr = try {
                org.json.JSONArray(cleanJson)
            } catch (_: Exception) { org.json.JSONArray() }

            val currentIndex = (0 until queueArr.length()).firstOrNull { i ->
                queueArr.getJSONObject(i).getString("id") == videoUrl
            } ?: 0

            // 3. Extract stream URL for the current video
            val streamInfo = YtDlpExtractor.getStreamInfoById(videoUrl)

            if (streamInfo != null) {
                runOnUiThread {
                    startNativePlayerWithQueue(
                        url          = streamInfo.url,
                        title        = currentVideoTitle ?: streamInfo.title,
                        thumbnailUrl = currentVideoThumbnail ?: streamInfo.thumbnailUrl,
                        startPosition = positionMs,
                        queueJson    = cleanJson,
                        currentIndex = currentIndex
                    )
                }
            } else {
                runOnUiThread { ensureAudioServiceRunning() }
            }
        }
    }
}
```

**Step 8c — Update `startNativePlayer()` to accept queue data**:

```kotlin
private fun startNativePlayerWithQueue(
    url: String,
    title: String,
    thumbnailUrl: String?,
    startPosition: Long,
    queueJson: String,
    currentIndex: Int
) {
    nativePlayerIntent = Intent(this, NativePlayerService::class.java).apply {
        action = NativePlayerService.ACTION_PLAY
        putExtra(NativePlayerService.EXTRA_VIDEO_URL, url)
        putExtra(NativePlayerService.EXTRA_VIDEO_TITLE, title)
        putExtra(NativePlayerService.EXTRA_THUMBNAIL_URL, thumbnailUrl)
        putExtra(NativePlayerService.EXTRA_START_POSITION, startPosition)
        putExtra(NativePlayerService.EXTRA_QUEUE_JSON, queueJson)     // NEW
        putExtra(NativePlayerService.EXTRA_QUEUE_INDEX, currentIndex) // NEW
    }
    startForegroundService(nativePlayerIntent)
}
```

---

### STEP 9 — Auto-Next in `NativePlayerService` via `Player.Listener`

`NativePlayerService` receives the queue. When ExoPlayer finishes the current video, extract the next video's stream URL and enqueue it.

**File**: `NativePlayerService.kt`

Add companion constants:
```kotlin
companion object {
    // ... existing constants ...
    const val EXTRA_QUEUE_JSON  = "queue_json"
    const val EXTRA_QUEUE_INDEX = "queue_index"
}
```

Add state:
```kotlin
private var queueJson: String = "[]"
private var queueIndex: Int = 0
private val serviceScope = CoroutineScope(Dispatchers.IO + SupervisorJob())
```

In `onStartCommand`, parse these:
```kotlin
ACTION_PLAY -> {
    val videoUrl      = intent.getStringExtra(EXTRA_VIDEO_URL) ?: return START_NOT_STICKY
    val videoTitle    = intent.getStringExtra(EXTRA_VIDEO_TITLE) ?: "Video"
    val thumbnailUrl  = intent.getStringExtra(EXTRA_THUMBNAIL_URL)
    val startPosition = intent.getLongExtra(EXTRA_START_POSITION, 0L)
    queueJson         = intent.getStringExtra(EXTRA_QUEUE_JSON) ?: "[]"
    queueIndex        = intent.getIntExtra(EXTRA_QUEUE_INDEX, 0)

    playVideo(videoUrl, videoTitle, thumbnailUrl, startPosition)
}
```

Add a `Player.Listener` inside `initializePlayer()`:

```kotlin
player?.addListener(object : Player.Listener {
    override fun onPlaybackStateChanged(playbackState: Int) {
        if (playbackState == Player.STATE_ENDED) {
            // Current video ended — advance to next in queue
            playNextInQueue()
        }
    }
})
```

Add `playNextInQueue()`:

```kotlin
private fun playNextInQueue() {
    val nextIndex = queueIndex + 1
    val queue = try { org.json.JSONArray(queueJson) }
                catch (_: Exception) { return }

    if (nextIndex >= queue.length()) {
        // End of queue — stop the service
        android.util.Log.d("NativePlayerService", "Queue exhausted, stopping.")
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
        return
    }

    val nextItem = queue.getJSONObject(nextIndex)
    val nextId    = nextItem.getString("id")
    val nextTitle = nextItem.optString("title", "Video")
    val nextThumb = nextItem.optString("thumb", null.toString())
    val nextStart = nextItem.optLong("start", 0L) * 1000L // convert seconds → ms

    queueIndex = nextIndex

    android.util.Log.d("NativePlayerService", "Auto-advancing to queue index $nextIndex: $nextTitle")

    // Extract stream URL for the next video (runs in background coroutine)
    serviceScope.launch {
        val streamInfo = YtDlpExtractor.getStreamInfoById(nextId)
        if (streamInfo != null) {
            val mediaItem = MediaItem.Builder()
                .setUri(streamInfo.url)
                .setMediaMetadata(
                    MediaMetadata.Builder()
                        .setTitle(nextTitle)
                        .setArtist("yClippy")
                        .setArtworkUri(nextThumb.let {
                            if (it != "null") android.net.Uri.parse(it) else null
                        })
                        .build()
                )
                .build()

            // Switch to main thread to update ExoPlayer
            Handler(Looper.getMainLooper()).post {
                player?.setMediaItem(mediaItem)
                player?.prepare()
                player?.seekTo(nextStart)
                player?.play()
            }
        } else {
            android.util.Log.w("NativePlayerService", "Failed to extract stream for next video, skipping.")
            // Try the one after
            queueIndex = nextIndex
            playNextInQueue()
        }
    }
}
```

Clean up coroutine scope in `onDestroy()`:

```kotlin
override fun onDestroy() {
    serviceScope.cancel()         // Cancel all pending extractions
    handler.removeCallbacks(updateRunnable)
    mediaSession?.run {
        player?.release()
        release()
        mediaSession = null
    }
    super.onDestroy()
}
```

---

### STEP 10 — Handle ExoPlayer Stream URL Expiry (Robustness)

YouTube stream URLs expire. If the URL becomes stale (ExoPlayer gets a 403/410 error), retry extraction:

```kotlin
player?.addListener(object : Player.Listener {
    override fun onPlayerError(error: androidx.media3.common.PlaybackException) {
        val errorCode = error.errorCode
        // HTTP 403/410 = URL expired, re-extract
        if (errorCode == PlaybackException.ERROR_CODE_IO_BAD_HTTP_STATUS) {
            android.util.Log.w("NativePlayerService", "Stream URL expired, re-extracting...")
            val queue = try { org.json.JSONArray(queueJson) } catch (_: Exception) { return }
            if (queueIndex < queue.length()) {
                val item = queue.getJSONObject(queueIndex)
                val videoId = item.getString("id")
                serviceScope.launch {
                    val streamInfo = YtDlpExtractor.getStreamInfoById(videoId)
                    if (streamInfo != null) {
                        Handler(Looper.getMainLooper()).post {
                            val currentPos = player?.currentPosition ?: 0L
                            val mediaItem = MediaItem.Builder()
                                .setUri(streamInfo.url)
                                .setMediaMetadata(MediaMetadata.Builder()
                                    .setTitle(streamInfo.title)
                                    .build())
                                .build()
                            player?.setMediaItem(mediaItem)
                            player?.prepare()
                            player?.seekTo(currentPos)
                            player?.play()
                        }
                    }
                }
            }
        } else {
            // Other errors: log and advance to next
            android.util.Log.e("NativePlayerService", "Player error: ${error.message}")
            playNextInQueue()
        }
    }
})
```

Add import:
```kotlin
import androidx.media3.common.PlaybackException
```

---

## All Files to Create / Modify

| File | Action | Summary |
|------|--------|---------|
| `src-tauri/gen/android/settings.gradle` | MODIFY | Add JitPack repository |
| `src-tauri/gen/android/app/build.gradle.kts` | MODIFY | Add `youtubedl-android:library`, `kotlinx-coroutines-android`, `lifecycle-runtime-ktx`; add `abiFilters` |
| `src-tauri/gen/android/app/src/main/AndroidManifest.xml` | MODIFY | Add `android:extractNativeLibs="true"` |
| `src-tauri/gen/android/app/src/main/java/com/yclippy/app/YtDlpExtractor.kt` | **CREATE** | `object` that wraps `YoutubeDL.getInfo()` to return a direct stream URL + metadata |
| `src-tauri/gen/android/app/src/main/java/com/yclippy/app/MainActivity.kt` | MODIFY | Init `YoutubeDL` in `onCreate`; add `currentVideoUrl/Title/Thumbnail`; add `YClippyNative.notifyVideoStarted()`; add `onResume()` for position sync; rewrite `onPause()` for background handoff |
| `src-tauri/gen/android/app/src/main/java/com/yclippy/app/NativePlayerService.kt` | MODIFY | Accept queue JSON; add `serviceScope`; add `playNextInQueue()`; add `Player.Listener` for end-of-video and error recovery; persist position to SharedPreferences |
| `src/lib/NativePlayer.svelte` | MODIFY | Call `notifyVideoStarted()` in `onMount`; expose `getQueueJson()` and `onPlayerPaused/Resumed` on `window.yClippyCallbacks` |

---

## Extraction Speed & UX Notes

| Concern | Detail |
|---------|--------|
| **Extraction time** | 1–3 seconds on WiFi. During this time the notification won't appear yet. ExoPlayer starts immediately once URL arrives. |
| **First launch size** | The `youtubedl-android:library` AAR adds ~22MB to the APK (yt-dlp binary + Python 3.8). This is unavoidable. Consider using ABI splits to only ship the relevant ABI per device. |
| **Update yt-dlp** | YouTube periodically breaks extractors. The library can self-update: `YoutubeDL.getInstance().updateYoutubeDL(context, UpdateChannel.STABLE)`. Run this on app startup (in a background coroutine) to stay compatible. |
| **Quality** | `-f best[ext=mp4]/best` picks the best combined progressive stream, usually 720p. For audio-only, use `-f bestaudio` — but ExoPlayer will still play it (just no video). |
| **No ffmpeg needed** | By using `-f best` (combined stream), we don't need the separate `:ffmpeg` module, keeping APK size smaller. |

---

## Verification

1. Open the app, play a video.
2. Lock the screen — within 2–3 seconds, audio continues playing.
3. Pull down notification shade — verify media notification shows video title + thumbnail.
4. Tap **Pause** in the notification — audio stops.
5. Tap **Play** — audio resumes.
6. Let the video finish — verify the next video in the folder starts automatically.
7. Unlock and return to the app — verify the `YouTubePlayerView` resumes from the same position ExoPlayer was at.
8. Test on a device with **gesture navigation** (Android 10+) and **3-button navigation** (both should work identically).
