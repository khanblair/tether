package app.tether.memory

private val WIKI_LINK = Regex("""\[\[([^\[\]|#]+)(?:#([^\[\]|]+))?(?:\|([^\[\]]+))?]]""")
private const val FENCE = "---"
private const val TARGET_GROUP = 1
private const val HEADING_GROUP = 2
private const val ALIAS_GROUP = 3

/** The lines after the first fence: the closing fence and the one that ends the frontmatter. */
private const val FENCE_LINES = 2

/** Splits a note body into text and wiki links. Every link starts out unresolved. */
fun parseBody(body: String): List<BodyPart> {
    val parts = mutableListOf<BodyPart>()
    var last = 0
    for (match in WIKI_LINK.findAll(body)) {
        val start = match.range.first
        if (start > last) parts += BodyPart.BodyText(body.substring(last, start))
        val target = match.groupValues[TARGET_GROUP].trim()
        val heading = match.groupValues[HEADING_GROUP].trim().ifEmpty { null }
        val alias = match.groupValues[ALIAS_GROUP].trim().ifEmpty { null }
        val label = alias ?: listOfNotNull(target, heading).joinToString(" > ")
        parts +=
            BodyPart.BodyLink(target = target, heading = heading, label = label, resolved = false)
        last = match.range.last + 1
    }
    if (last < body.length) parts += BodyPart.BodyText(body.substring(last))
    return parts
}

/**
 * Marks each link resolved when the core reports its target as found. [outgoing] is the core's
 * list of links written in the note. A link the core did not list stays unresolved.
 */
fun resolveBodyLinks(
    parts: List<BodyPart>,
    outgoing: List<VaultLink>,
): List<BodyPart> =
    parts.map { part ->
        if (part !is BodyPart.BodyLink) {
            part
        } else {
            val found =
                outgoing.firstOrNull {
                    it.target.trim() == part.target &&
                        it.heading?.trim() == part.heading
                }
            part.copy(resolved = found?.resolved ?: false)
        }
    }

/** The Markdown after the frontmatter block, or the whole [source] when it has none. */
fun noteBody(source: String): String {
    val lines = source.lines()
    val close = lines.drop(1).indexOfFirst { it.trim() == FENCE }
    val framed = lines.firstOrNull()?.trim() == FENCE && close >= 0
    return if (framed) {
        lines
            .drop(
                close + FENCE_LINES,
            ).joinToString("\n")
            .trimStart('\n')
    } else {
        source
    }
}

/**
 * The paths a link to [target] written in the note at [fromPath] may point to, in the order the
 * core tries them: the vault root first, then the note's own folder.
 */
fun linkCandidates(
    fromPath: String,
    target: String,
): List<String> {
    val file = "${target.removeSuffix(".md")}.md"
    val folder = fromPath.substringBeforeLast('/', "")
    val sibling = if (folder.isEmpty()) file else "$folder/$file"
    return listOf(file, sibling).distinct()
}

/**
 * The reason a note path cannot be used, or null when it can. A note path is relative, ends in
 * `.md`, and has no empty or dot-prefixed folder or file name, so `.obsidian` is never written.
 */
fun vaultPathProblem(path: String): AreaProblem? {
    val trimmed = path.trim()
    val segments = trimmed.split('/')
    val valid =
        trimmed.endsWith(".md") &&
            trimmed.length > MD_SUFFIX.length &&
            segments.all { it.isNotEmpty() && !it.startsWith(".") && it != ".." }
    return if (valid) null else AreaProblem.InvalidPath
}

private const val MD_SUFFIX = ".md"
