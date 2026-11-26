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

/**
 * Decides one request. Data and blob URLs pass. Any scheme other than https is blocked, even with
 * the network on. The preview origin is served from the project. Any other https host passes only
 * when [networkAllowed] is true.
 */
fun requestDecision(
    url: String,
    networkAllowed: Boolean,
): RequestDecision {
    val uri = parseUri(url) ?: return RequestDecision.Block
    val scheme = uri.scheme.orEmpty().lowercase()
    return when {
        scheme == "data" || scheme == "blob" -> RequestDecision.Pass
        scheme != "https" -> RequestDecision.Block
        uri.host.equals(PREVIEW_HOST, ignoreCase = true) && uri.userInfo == null ->
            previewPathFor(url)?.let { RequestDecision.Serve(it) } ?: RequestDecision.NotFound
        networkAllowed && uri.host != null -> RequestDecision.Pass
        else -> RequestDecision.Block
    }
}
