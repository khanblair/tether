package app.tether.settings.github

import java.util.Locale

/**
 * The `owner/repo` slug from a GitHub clone URL, or null when the URL has no owner and name.
 * Accepts `https://github.com/owner/repo.git` and `git@github.com:owner/repo.git`.
 */
fun repoSlug(cloneUrl: String): String? {
    val path =
        cloneUrl
            .trim()
            .substringAfter("github.com", "")
            .trimStart('/', ':')
            .removeSuffix("/")
            .removeSuffix(".git")
    val parts = path.split('/')
    return if (parts.size == 2 && parts.all { it.isNotBlank() }) "${parts[0]}/${parts[1]}" else null
}

/**
 * The project name a clone starts with: the repository name from the slug. When the URL is not a
 * GitHub slug, it is the last path segment without `.git`. Empty when neither gives a name.
 */
fun defaultProjectName(cloneUrl: String): String {
    repoSlug(cloneUrl)?.let { return it.substringAfter('/') }
    return cloneUrl
        .trim()
        .trimEnd('/')
        .substringAfterLast('/')
        .removeSuffix(".git")
}

/**
 * The repositories whose slug or clone URL contains [query], ignoring case. A blank query keeps
 * every repository, in the order given.
 */
fun filterRepos(
    repos: List<GithubRepo>,
    query: String,
): List<GithubRepo> {
    val needle = query.trim().lowercase(Locale.ROOT)
    if (needle.isEmpty()) return repos
    return repos.filter { repo ->
        val slug = repoSlug(repo.cloneUrl).orEmpty()
        slug.lowercase(Locale.ROOT).contains(needle) ||
            repo.cloneUrl.lowercase(Locale.ROOT).contains(needle)
    }
}

/** Megabytes for a size in kilobytes, rounded to the nearest whole number. */
fun megabytesFromKb(sizeKb: Long): Long = (sizeKb + KB_PER_MB / 2) / KB_PER_MB

/** Megabytes for a byte count, to one decimal place as text with a dot. */
fun megabytesText(bytes: Long): String =
    String.format(Locale.ROOT, "%.1f", bytes.toDouble() / BYTES_PER_MB)

private const val KB_PER_MB = 1024L
private const val BYTES_PER_MB = 1024.0 * 1024.0
