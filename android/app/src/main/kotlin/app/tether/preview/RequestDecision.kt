package app.tether.preview

/** What the preview does with one request its WebView makes. */
sealed interface RequestDecision {
    /** Answer from the project file at [path]. */
    data class Serve(
        val path: String,
    ) : RequestDecision

    /** Same origin, but the path is unsafe or missing: answer 404. */
    data object NotFound : RequestDecision

    /** Another host, or a scheme the preview never allows: answer 403 and count it. */
    data object Block : RequestDecision

    /** Let the system load it: a data or blob URL, or an https host with the network on. */
    data object Pass : RequestDecision
}

/** The one cleartext server origin a server preview may use. */
data class ServerOrigin(
    val port: Int,
)

/**
 * Decides one request. Data and blob URLs pass. Any scheme other than https is blocked, even with
 * the network on. The preview origin is served from the project. Any other https host passes only
 * when [networkAllowed] is true.
 */
fun requestDecision(
    url: String,
    networkAllowed: Boolean,
    serverOrigin: ServerOrigin? = null,
): RequestDecision {
    val uri = parseUri(url) ?: return RequestDecision.Block
    val scheme = uri.scheme.orEmpty().lowercase()
    return when {
        scheme == "data" || scheme == "blob" -> RequestDecision.Pass
        isServerOrigin(uri, serverOrigin) -> RequestDecision.Pass
        scheme == "http" && isLoopbackHost(uri.host) -> RequestDecision.Block
        scheme != "https" -> RequestDecision.Block
        isPreviewHost(uri) -> previewDecision(url)
        networkAllowed && uri.host != null -> RequestDecision.Pass
        else -> RequestDecision.Block
    }
}

/** The project file for a preview host URL, or 404 when the path is unsafe or missing. */
private fun previewDecision(url: String): RequestDecision =
    previewPathFor(url)?.let { RequestDecision.Serve(it) } ?: RequestDecision.NotFound

/** Whether [uri] names the preview host with no user info. */
private fun isPreviewHost(uri: java.net.URI): Boolean =
    uri.host.equals(PREVIEW_HOST, ignoreCase = true) && uri.userInfo == null

/** Whether [uri] is exactly the server origin, with no user info, port alias, or fragment. */
internal fun isServerOrigin(
    uri: java.net.URI,
    origin: ServerOrigin?,
): Boolean =
    origin != null &&
        uri.scheme.equals("http", ignoreCase = true) &&
        uri.host == LOOPBACK_HOST &&
        uri.port == origin.port &&
        uri.userInfo == null &&
        uri.rawFragment == null

private fun isLoopbackHost(host: String?): Boolean = host == LOOPBACK_HOST

private const val LOOPBACK_HOST = "127.0.0.1"
