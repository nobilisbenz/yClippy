package com.yclippy.app

import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Bundle
import android.util.Log
import android.view.ViewGroup
import android.webkit.JavascriptInterface
import android.webkit.WebView
import android.widget.Toast
import org.json.JSONObject

class MainActivity : TauriActivity() {

    private var webView: WebView? = null
    private val pendingIntents = mutableListOf<VideoIntent>()
    private var webViewReady = false

    /** A video id plus what to do with it: play at a second, or offer to import. */
    private data class VideoIntent(
        val videoId: String,
        val startSeconds: Int = 0,
        val play: Boolean = false,
    )

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        handleIntent(intent)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        handleIntent(intent)
    }

    private fun handleIntent(intent: Intent?) {
        when (intent?.action) {
            Intent.ACTION_SEND -> handleShareIntent(intent)
            Intent.ACTION_VIEW -> handleViewIntent(intent)
        }
    }

    private fun handleViewIntent(intent: Intent) {
        val data = intent.data ?: return
        if (data.scheme == "yclippy") {
            // Anything in this URI came from another app on the device, so the
            // id is validated to the YouTube shape before it goes anywhere near
            // the webview, and the timestamp is parsed as an integer.
            val raw = data.getQueryParameter("v")
            val videoId = raw?.takeIf { VIDEO_ID.matches(it) } ?: run {
                Log.w(TAG, "Rejected deep link with a malformed video id")
                Toast.makeText(this, "That link isn't a YouTube video", Toast.LENGTH_SHORT).show()
                return
            }
            val start = parseStartSeconds(data.getQueryParameter("t"))
            deliver(VideoIntent(videoId, start, play = true))
            return
        }

        val url = data.toString()
        val videoId = extractYouTubeId(url) ?: run {
            Log.w(TAG, "VIEW intent has no YouTube video id: $url")
            Toast.makeText(this, "No video in that link", Toast.LENGTH_SHORT).show()
            return
        }
        deliver(VideoIntent(videoId, parseStartSeconds(data.getQueryParameter("t")), play = true))
    }

    /** Accepts `90`, `90s`, `6m54s`, `1h2m3s` — the shapes YouTube itself emits. */
    private fun parseStartSeconds(raw: String?): Int {
        val value = raw?.trim().orEmpty()
        if (value.isEmpty()) return 0
        value.toIntOrNull()?.let { return maxOf(0, it) }

        val match = Regex("^(?:(\\d+)h)?(?:(\\d+)m)?(?:(\\d+)s)?$").find(value) ?: return 0
        val (h, m, s) = match.destructured
        val total = (h.toIntOrNull() ?: 0) * 3600 +
            (m.toIntOrNull() ?: 0) * 60 +
            (s.toIntOrNull() ?: 0)
        return maxOf(0, total)
    }

    override fun onWebViewCreate(webView: WebView) {
        super.onWebViewCreate(webView)
        webView.settings.javaScriptEnabled = true
        webView.addJavascriptInterface(YClippyNative(this), "yClippyNative")
        this.webView = webView
    }

    private fun handleShareIntent(intent: Intent?) {
        if (intent?.action != Intent.ACTION_SEND) return
        // The manifest advertises text/*; accepting only text/plain here made
        // yClippy appear in share sheets and then do nothing.
        val type = intent.type.orEmpty()
        if (!type.startsWith("text/")) return

        val text = intent.getStringExtra(Intent.EXTRA_TEXT)
            ?: intent.getStringExtra(Intent.EXTRA_TITLE)
            ?: intent.data?.toString()
            ?: return
        val videoId = extractYouTubeId(text) ?: run {
            Log.w(TAG, "Shared text has no YouTube ID: $text")
            Toast.makeText(this, "Shared text isn't a YouTube link", Toast.LENGTH_SHORT).show()
            return
        }
        Log.d(TAG, "Received shared video: $videoId")
        deliver(VideoIntent(videoId, startSeconds = timestampIn(text)))
    }

    /** Pulls `?t=` / `&t=` out of a shared URL so a shared moment stays a moment. */
    private fun timestampIn(text: String): Int {
        val match = Regex("[?&]t=([0-9hms]+)").find(text) ?: return 0
        return parseStartSeconds(match.groupValues.getOrNull(1))
    }

    private fun deliver(intent: VideoIntent) {
        if (webViewReady) {
            dispatch(intent)
        } else {
            pendingIntents.add(intent)
        }
    }

    private fun dispatch(intent: VideoIntent) {
        // Values are JSON-encoded rather than interpolated: the deep-link path
        // carries text from other apps on the device.
        val payload = JSONObject()
            .put("videoId", intent.videoId)
            .put("startSeconds", intent.startSeconds)
            .put("play", intent.play)
            .toString()
        val js = """
            (function () {
              var p = $payload;
              if (p.play && window.__yclippyOnPlay) { window.__yclippyOnPlay(p); }
              else if (window.__yclippyOnSharedVideo) { window.__yclippyOnSharedVideo(p.videoId); }
              else { console.warn('yClippy frontend not ready'); }
            })();
        """.trimIndent()
        webView?.evaluateJavascript(js, null)
    }

    private fun extractYouTubeId(input: String): String? {
        for (line in input.lines()) {
            val patterns = listOf(
                Regex("(?:v=|/v/|youtu\\.be/|/embed/|/shorts/)([a-zA-Z0-9_-]{11})"),
                Regex("^([a-zA-Z0-9_-]{11})$")
            )
            for (p in patterns) {
                p.find(line)?.groupValues?.getOrNull(1)?.let { return it }
            }
        }
        return null
    }

    fun openInExternalPlayer(videoId: String, startSeconds: Int = 0) {
        val watchUrl = if (startSeconds > 0) {
            "https://www.youtube.com/watch?v=$videoId&t=${startSeconds}s"
        } else {
            "https://www.youtube.com/watch?v=$videoId"
        }
        val candidates = listOf(
            "app.rvx.android.youtube"       to "ReVanced Extended",
            "app.revanced.android.youtube"  to "ReVanced",
            "app.revanced.android.apps.youtube" to "ReVanced (legacy)",
            "com.google.android.youtube"    to "YouTube"
        )

        for ((pkg, label) in candidates) {
            val intent = Intent(Intent.ACTION_VIEW, Uri.parse(watchUrl)).apply {
                setPackage(pkg)
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            }
            try {
                Log.d(TAG, "Trying to open in $pkg ($label) at $startSeconds s")
                startActivity(intent)
                Log.d(TAG, "Successfully opened in $pkg")
                Toast.makeText(
                    this,
                    "✓ $label — background audio will continue",
                    Toast.LENGTH_LONG
                ).show()
                return
            } catch (e: Exception) {
                Log.w(TAG, "Failed to open in $pkg: ${e.javaClass.simpleName}: ${e.message}")
            }
        }

        try {
            val chooserIntent = Intent(Intent.ACTION_VIEW, Uri.parse(watchUrl)).apply {
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            }
            startActivity(Intent.createChooser(chooserIntent, "Open video with…"))
        } catch (e: Exception) {
            Log.e(TAG, "No app can handle this intent", e)
            Toast.makeText(this, "No YouTube app installed", Toast.LENGTH_LONG).show()
        }
    }

    fun onSvelteAppReady() {
        runOnUiThread {
            webViewReady = true
            Log.d(TAG, "Frontend ready, draining ${pendingIntents.size} pending intents")
            pendingIntents.forEach { dispatch(it) }
            pendingIntents.clear()
        }
    }

    companion object {
        private const val TAG = "MainActivity"
        private val VIDEO_ID = Regex("^[a-zA-Z0-9_-]{11}$")
    }
}

class YClippyNative(private val activity: MainActivity) {
    @JavascriptInterface
    fun openInRevanced(videoId: String, startSeconds: Int = 0) {
        activity.runOnUiThread {
            activity.openInExternalPlayer(videoId, startSeconds)
        }
    }

    @JavascriptInterface
    fun onAppReady() {
        activity.onSvelteAppReady()
    }
}
