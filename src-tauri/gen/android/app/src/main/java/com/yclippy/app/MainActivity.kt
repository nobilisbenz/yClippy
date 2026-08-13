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

class MainActivity : TauriActivity() {

    private var webView: WebView? = null
    private val pendingSharedVideoIds = mutableListOf<String>()
    private var webViewReady = false

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
            val videoId = data.getQueryParameter("v") ?: return
            deliverSharedVideo(videoId)
            return
        }
        val videoId = extractYouTubeId(data.toString()) ?: return
        deliverSharedVideo(videoId)
    }

    override fun onWebViewCreate(webView: WebView) {
        super.onWebViewCreate(webView)
        webView.settings.javaScriptEnabled = true
        webView.addJavascriptInterface(YClippyNative(this), "yClippyNative")
        this.webView = webView
    }

    private fun handleShareIntent(intent: Intent?) {
        if (intent?.action != Intent.ACTION_SEND) return
        if (intent.type != "text/plain") return
        val text = intent.getStringExtra(Intent.EXTRA_TEXT) ?: return
        val videoId = extractYouTubeId(text) ?: run {
            Log.w(TAG, "Shared text has no YouTube ID: $text")
            Toast.makeText(this, "Shared text isn't a YouTube link", Toast.LENGTH_SHORT).show()
            return
        }
        Log.d(TAG, "Received shared video: $videoId")
        Toast.makeText(this, "Importing video: $videoId", Toast.LENGTH_SHORT).show()
        deliverSharedVideo(videoId)
    }

    private fun deliverSharedVideo(videoId: String) {
        val js = "if (window.__yclippyOnSharedVideo) { window.__yclippyOnSharedVideo('$videoId'); } else { console.warn('Svelte app not ready'); }"
        if (webViewReady) {
            webView?.evaluateJavascript(js, null)
        } else {
            pendingSharedVideoIds.add(videoId)
        }
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
            Log.d(TAG, "Svelte app ready, draining ${pendingSharedVideoIds.size} pending shared videos")
            pendingSharedVideoIds.forEach { id ->
                webView?.evaluateJavascript(
                    "if (window.__yclippyOnSharedVideo) { window.__yclippyOnSharedVideo('$id'); }",
                    null
                )
            }
            pendingSharedVideoIds.clear()
        }
    }

    companion object {
        private const val TAG = "MainActivity"
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
