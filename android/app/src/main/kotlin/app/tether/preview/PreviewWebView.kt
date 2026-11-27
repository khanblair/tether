package app.tether.preview

import android.annotation.SuppressLint
import android.content.Context
import android.net.Uri
import android.util.Log
import android.webkit.CookieManager
import android.webkit.GeolocationPermissions
import android.webkit.PermissionRequest
import android.webkit.ValueCallback
import android.webkit.WebChromeClient
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebSettings
import android.webkit.WebStorage
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Modifier
import androidx.compose.ui.viewinterop.AndroidView
import app.tether.files.FilesFailure
import app.tether.files.FilesProblem
import java.io.ByteArrayInputStream

private const val PREVIEW_HTML_BASE = "$PREVIEW_ORIGIN/$INDEX_FILE"
private const val STATUS_OK = 200
private const val STATUS_FORBIDDEN = 403
private const val STATUS_NOT_FOUND = 404
private const val STATUS_TOO_LARGE = 413
private const val STATUS_FAILED = 500
private const val TYPE_PLAIN = "text/plain"
private const val UTF_8 = "utf-8"

/**
 * The WebView that shows a preview. Its settings and permissions follow docs/specs/preview.md
 * section 3: JavaScript is on for the pages the user chose, and no app object is exposed to them.
 * [readFile] and [networkAllowed] run on a WebView thread. [onBlocked] is called on the main
 * thread, and [onAddressBlocked] for a link the preview would not open.
 */
@Composable
fun PreviewWebView(
    load: PreviewLoad,
    networkAllowed: () -> Boolean,
    readFile: (String) -> FileBytes,
    onBlocked: (String) -> Unit,
    onAddressBlocked: (String) -> Unit,
    modifier: Modifier = Modifier,
    serverOrigin: ServerOrigin? = null,
) {
    val allowed by rememberUpdatedState(networkAllowed)
    val client =
        PreviewClient(
            networkAllowed = { allowed() },
            serverOrigin = serverOrigin,
            readFile = readFile,
            onBlocked = onBlocked,
            onAddressBlocked = onAddressBlocked,
        )
    AndroidView(
        modifier = modifier.fillMaxSize(),
        factory = { context -> newPreviewWebView(context, client, load) },
        onRelease = { view -> releasePreviewWebView(view) },
    )
}

/**
 * Builds the WebView with the security settings of the spec, then loads [load]. The page is
 * loaded here because the caller rebuilds the view for each reload.
 */
@SuppressLint("SetJavaScriptEnabled") // Pages need JavaScript; see docs/specs/preview.md section 3.
// The file-URL settings are deprecated, but they must be set off, so the warning is expected.
@Suppress("DEPRECATION")
private fun newPreviewWebView(
    context: Context,
    client: WebViewClient,
    load: PreviewLoad,
): WebView {
    WebView.setWebContentsDebuggingEnabled(false)
    return WebView(context).apply {
        settings.apply {
            javaScriptEnabled = true
            domStorageEnabled = true
            allowFileAccess = false
            allowContentAccess = false
            allowFileAccessFromFileURLs = false
            allowUniversalAccessFromFileURLs = false
            setGeolocationEnabled(false)
            setSupportMultipleWindows(false)
            mediaPlaybackRequiresUserGesture = true
            cacheMode = WebSettings.LOAD_NO_CACHE
        }
        webViewClient = client
        webChromeClient = DenyingChromeClient
        when (load) {
            is PreviewLoad.Page -> loadUrl(load.url)
            is PreviewLoad.Html ->
                loadDataWithBaseURL(PREVIEW_HTML_BASE, load.html, "text/html", UTF_8, null)
            else -> Unit
        }
    }
}

/** Clears the cookies and storage of the preview, then destroys the view. */
private fun releasePreviewWebView(view: WebView) {
    view.stopLoading()
    CookieManager.getInstance().removeAllCookies(null)
    WebStorage.getInstance().deleteAllData()
    view.destroy()
}

/** Denies every permission and file chooser, and opens no new windows. */
private object DenyingChromeClient : WebChromeClient() {
    override fun onPermissionRequest(request: PermissionRequest?) {
        request?.deny()
    }

    override fun onGeolocationPermissionsShowPrompt(
        origin: String?,
        callback: GeolocationPermissions.Callback?,
    ) {
        callback?.invoke(origin, false, false)
    }

    override fun onShowFileChooser(
        webView: WebView?,
        filePathCallback: ValueCallback<Array<Uri>>?,
        fileChooserParams: WebChromeClient.FileChooserParams?,
    ): Boolean {
        filePathCallback?.onReceiveValue(null)
        return true
    }
}

/** Answers each request from [PreviewFilesGateway] or blocks it, as [requestDecision] says. */
private class PreviewClient(
    private val networkAllowed: () -> Boolean,
    private val serverOrigin: ServerOrigin?,
    private val readFile: (String) -> FileBytes,
    private val onBlocked: (String) -> Unit,
    private val onAddressBlocked: (String) -> Unit,
) : WebViewClient() {
    override fun shouldInterceptRequest(
        view: WebView?,
        request: WebResourceRequest?,
    ): WebResourceResponse? {
        val url = request?.url?.toString() ?: return textResponse(STATUS_FORBIDDEN)
        return when (val decision = requestDecision(url, networkAllowed(), serverOrigin)) {
            is RequestDecision.Serve -> serve(decision.path)
            RequestDecision.NotFound -> textResponse(STATUS_NOT_FOUND)
            RequestDecision.Block -> {
                view?.post { onBlocked(url) }
                textResponse(STATUS_FORBIDDEN)
            }
            RequestDecision.Pass -> null
        }
    }

    override fun shouldOverrideUrlLoading(
        view: WebView?,
        request: WebResourceRequest?,
    ): Boolean {
        val url = request?.url?.toString().orEmpty()
        val allowed =
            if (serverOrigin == null) {
                previewPathFor(url) != null
            } else {
                parseUri(url)?.let { isServerOrigin(it, serverOrigin) } == true
            }
        if (!allowed) onAddressBlocked(url)
        return !allowed
    }

    private fun serve(path: String): WebResourceResponse =
        try {
            val file = readFile(path)
            if (file.truncated) {
                textResponse(STATUS_TOO_LARGE)
            } else {
                val mime = mimeFor(path)
                WebResourceResponse(
                    mime.mimeType,
                    mime.encoding,
                    STATUS_OK,
                    "OK",
                    mapOf("Cache-Control" to "no-store"),
                    ByteArrayInputStream(file.bytes),
                )
            }
        } catch (failure: FilesFailure) {
            textResponse(statusFor(failure.problem))
        } catch (
            @Suppress("TooGenericExceptionCaught") failure: Exception,
        ) {
            textResponse(STATUS_FAILED, cause = failure)
        }

    private fun statusFor(problem: FilesProblem): Int =
        if (problem == FilesProblem.NotFound) STATUS_NOT_FOUND else STATUS_FAILED

    private fun textResponse(
        status: Int,
        cause: Exception? = null,
    ): WebResourceResponse {
        cause?.let { Log.w(TAG, "preview read failed: ${it.javaClass.simpleName}") }
        return WebResourceResponse(
            TYPE_PLAIN,
            UTF_8,
            status,
            reasonFor(status),
            mapOf("Cache-Control" to "no-store"),
            ByteArrayInputStream(ByteArray(0)),
        )
    }

    private fun reasonFor(status: Int): String =
        when (status) {
            STATUS_FORBIDDEN -> "Forbidden"
            STATUS_NOT_FOUND -> "Not Found"
            STATUS_TOO_LARGE -> "Payload Too Large"
            else -> "Error"
        }
}

private const val TAG = "Tether"
