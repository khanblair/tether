package app.tether.git

import java.io.ByteArrayOutputStream
import java.nio.charset.StandardCharsets

/**
 * Reads unified diff text, as the Git core prints it, into files, hunks, and lines. Pure Kotlin.
 * [parse] never throws: text it does not understand is skipped, so garbage gives an empty list.
 */
object UnifiedDiffParser {
    private const val TRUNCATION_PREFIX = "[diff truncated at "

    /** Parses [text] into one [DiffFile] per `diff --git` section, in the order they appear. */
    fun parse(text: String): List<DiffFile> {
        val reader = DiffReader()
        splitLines(text).forEach(reader::accept)
        return reader.finish()
    }

    /** True when the core cut the diff at its size limit. The cut is a line of its own. */
    fun isTruncated(text: String): Boolean =
        text.lineSequence().any { line -> line.removeSuffix("\r").startsWith(TRUNCATION_PREFIX) }

    /** Splits on newlines and drops the empty piece after a final newline. */
    private fun splitLines(text: String): List<String> {
        val lines = text.split('\n')
        return if (lines.isNotEmpty() && lines.last().isEmpty()) lines.dropLast(1) else lines
    }
}

/** Reads the diff one line at a time. State is the file being built and its open hunk. */
private class DiffReader {
    private val files = mutableListOf<FileDraft>()
    private var current: FileDraft? = null

    fun accept(rawLine: String) {
        val line = rawLine.removeSuffix("\r")
        val file = current
        when {
            line.startsWith(DIFF_HEADER) -> startFile(line.removePrefix(DIFF_HEADER))
            file == null -> Unit
            line.startsWith("\\") && file.lastHunk() != null -> file.lastHunk()?.addNote(line)
            file.hunkIsOpen() && isHunkLine(line) -> file.addHunkLine(line)
            line.startsWith("@@ ") -> file.startHunk(line)
            else -> file.readHeader(line)
        }
    }

    fun finish(): List<DiffFile> {
        current?.let(files::add)
        current = null
        return files.map(FileDraft::toFile)
    }

    private fun startFile(rest: String) {
        current?.let(files::add)
        val names = splitHeaderNames(rest)
        current = FileDraft(names?.second ?: rest)
    }

    private fun isHunkLine(line: String): Boolean =
        line.isEmpty() || line[0] == '+' || line[0] == '-' || line[0] == ' '

    private companion object {
        const val DIFF_HEADER = "diff --git "
    }
}

/** One file under construction. Hunks are kept as drafts until [toFile]. */
private class FileDraft(
    private val headerPath: String,
) {
    private val hunks = mutableListOf<HunkDraft>()
    private var hunk: HunkDraft? = null
    private var added = 0
    private var removed = 0
    private var binary = false
    private var minusPath: String? = null
    private var plusPath: String? = null
    private var renameFrom: String? = null
    private var renameTo: String? = null

    fun lastHunk(): HunkDraft? = hunk

    fun hunkIsOpen(): Boolean = hunk?.isOpen() == true

    fun startHunk(line: String) {
        val start = parseHunkHeader(line)
        hunk =
            start?.let {
                HunkDraft(line, it).also(hunks::add)
            }
    }

    fun addHunkLine(line: String) {
        val open = hunk ?: return
        val kind = open.addLine(line)
        when (kind) {
            DiffLineKind.Added -> added++
            DiffLineKind.Removed -> removed++
            else -> Unit
        }
    }

    fun readHeader(line: String) {
        when {
            line.startsWith("--- ") -> minusPath = parsePathToken(line.removePrefix("--- "), "a/")
            line.startsWith("+++ ") -> plusPath = parsePathToken(line.removePrefix("+++ "), "b/")
            line.startsWith("rename from ") ->
                renameFrom =
                    unquote(line.removePrefix("rename from "))
            line.startsWith("rename to ") -> renameTo = unquote(line.removePrefix("rename to "))
            line.startsWith("Binary files ") || line.startsWith("GIT binary patch") -> binary = true
            else -> Unit
        }
    }

    fun toFile(): DiffFile {
        val path = renameTo ?: plusPath ?: minusPath ?: headerPath
        val oldPath = renameFrom ?: minusPath?.takeIf { it != path }
        return DiffFile(
            path = path,
            oldPath = oldPath,
            hunks = hunks.map(HunkDraft::toHunk),
            added = added,
            removed = removed,
            binary = binary,
        )
    }
}

/** The start numbers and remaining line counts of one hunk. */
private data class HunkStart(
    val oldStart: Int,
    val oldCount: Int,
    val newStart: Int,
    val newCount: Int,
)

/** One hunk under construction. It stays open until both line counts from its header run out. */
private class HunkDraft(
    private val header: String,
    start: HunkStart,
) {
    private val lines = mutableListOf<DiffLineModel>()
    private var oldNo = start.oldStart
    private var newNo = start.newStart
    private var oldLeft = start.oldCount
    private var newLeft = start.newCount

    fun isOpen(): Boolean = oldLeft > 0 || newLeft > 0

    /** Adds one content line and returns its kind. */
    fun addLine(line: String): DiffLineKind =
        when (line.firstOrNull()) {
            '+' -> {
                lines.add(DiffLineModel(DiffLineKind.Added, null, newNo, line.drop(1)))
                newNo++
                newLeft--
                DiffLineKind.Added
            }
            '-' -> {
                lines.add(DiffLineModel(DiffLineKind.Removed, oldNo, null, line.drop(1)))
                oldNo++
                oldLeft--
                DiffLineKind.Removed
            }
            else -> {
                lines.add(DiffLineModel(DiffLineKind.Context, oldNo, newNo, line.drop(1)))
                oldNo++
                newNo++
                oldLeft--
                newLeft--
                DiffLineKind.Context
            }
        }

    fun addNote(line: String) {
        lines.add(DiffLineModel(DiffLineKind.Note, null, null, line.removePrefix("\\").trimStart()))
    }

    fun toHunk(): DiffHunk = DiffHunk(header, lines.toList())
}

/** A missing count in a hunk header means one line. */
private const val DEFAULT_LINE_COUNT = 1

/** Reads `@@ -a,b +c,d @@`. A missing count means one line. Returns null for any other text. */
private fun parseHunkHeader(line: String): HunkStart? {
    val match = HUNK_HEADER.find(line) ?: return null
    val (old, new) = match.destructured
    val oldRange = parseRange(old)
    val newRange = parseRange(new)
    return if (oldRange != null && newRange != null) {
        HunkStart(oldRange.first, oldRange.second, newRange.first, newRange.second)
    } else {
        null
    }
}

/** Reads one `-a,b` or `+c,d` part of a hunk header into its start and line count. */
private fun parseRange(text: String): Pair<Int, Int>? {
    val match = RANGE_PART.find(text) ?: return null
    val (first, count) = match.destructured
    val start = first.toIntOrNull()
    val lines = lineCount(count)
    return if (start != null && lines != null) start to lines else null
}

private fun lineCount(text: String): Int? =
    if (text.isEmpty()) DEFAULT_LINE_COUNT else text.toIntOrNull()

private val HUNK_HEADER = Regex("""^@@ (-\d+(?:,\d*)?) (\+\d+(?:,\d*)?) @@""")

private val RANGE_PART = Regex("""[-+](\d+)(?:,(\d*))?""")

/**
 * Reads the two names from the rest of a `diff --git` line. Returns the old and new names with
 * their `a/` and `b/` prefixes removed, or null when the line does not have that shape.
 */
private fun splitHeaderNames(rest: String): Pair<String, String>? =
    if (rest.startsWith("\"")) quotedNames(rest) else plainNames(rest)

/** Two names where the first one is quoted. The second may be quoted too. */
private fun quotedNames(rest: String): Pair<String, String>? {
    val end = closingQuote(rest)
    return if (end < 0) {
        null
    } else {
        val old = unquote(rest.substring(0, end + 1))
        val new = unquote(rest.substring(end + 1).trimStart())
        old.removePrefix("a/") to new.removePrefix("b/")
    }
}

/** Two names with no quotes. The split is the space before `b/` where both names are equal. */
private fun plainNames(rest: String): Pair<String, String>? {
    val index = plainSplitIndex(rest) ?: return null
    val old = rest.substring(0, index).removePrefix("a/")
    val new = rest.substring(index + 1).removePrefix("b/")
    return old to new
}

private fun plainSplitIndex(rest: String): Int? {
    val splits = Regex(" b/").findAll(rest).map { it.range.first }.toList()
    return splits.firstOrNull { index -> namesMatch(rest, index) } ?: splits.firstOrNull()
}

private fun namesMatch(
    rest: String,
    index: Int,
): Boolean {
    val left = rest.substring(0, index)
    val right = rest.substring(index + 1)
    return left.startsWith("a/") &&
        right.startsWith("b/") &&
        left.removePrefix("a/") == right.removePrefix("b/")
}

/** The index of the quote that closes the one at index 0, or -1. Backslash escapes are skipped. */
private fun closingQuote(text: String): Int {
    var index = 1
    while (index < text.length) {
        when (text[index]) {
            '\\' -> index++
            '"' -> return index
        }
        index++
    }
    return -1
}

/** Reads the name in a `---` or `+++` line. `/dev/null` means no file on that side. */
private fun parsePathToken(
    raw: String,
    prefix: String,
): String? {
    val token = raw.trimEnd('\t')
    if (token == DEV_NULL) return null
    return unquote(token).removePrefix(prefix)
}

private const val DEV_NULL = "/dev/null"

/**
 * Removes the quotes Git puts around a name with special characters, and decodes its escapes.
 * Octal escapes are bytes of UTF-8, so they are decoded together. Any other text comes back as is.
 */
internal fun unquote(token: String): String {
    if (token.length < 2 || token.first() != '"' || token.last() != '"') return token
    val bytes = ByteArrayOutputStream()
    val end = token.length - 1
    var index = 1
    while (index < end) {
        val c = token[index]
        if (c == '\\' && index + 1 < end) {
            index += writeEscape(token, index, end, bytes)
        } else {
            val codePoint = token.codePointAt(index)
            bytes.write(String(Character.toChars(codePoint)).toByteArray(StandardCharsets.UTF_8))
            index += Character.charCount(codePoint)
        }
    }
    return bytes.toString(StandardCharsets.UTF_8.name())
}

/** Writes the escape that starts at [start] and returns how many characters it used. */
private fun writeEscape(
    token: String,
    start: Int,
    end: Int,
    bytes: ByteArrayOutputStream,
): Int {
    val next = token[start + 1]
    val octal =
        if (next in OCTAL_FIRST_DIGITS && start + OCTAL_DIGITS < end) {
            token.substring(start + 1, start + 1 + OCTAL_DIGITS)
        } else {
            null
        }
    val octalValue = octal?.toIntOrNull(OCTAL_RADIX)
    if (octalValue != null) {
        bytes.write(octalValue)
        return OCTAL_ESCAPE_LENGTH
    }
    val decoded =
        when (next) {
            'n' -> "\n"
            't' -> "\t"
            else -> next.toString()
        }
    bytes.write(decoded.toByteArray(StandardCharsets.UTF_8))
    return SIMPLE_ESCAPE_LENGTH
}

private const val OCTAL_RADIX = 8
private const val OCTAL_DIGITS = 3
private const val OCTAL_ESCAPE_LENGTH = OCTAL_DIGITS + 1
private const val SIMPLE_ESCAPE_LENGTH = 2
private val OCTAL_FIRST_DIGITS = '0'..'3'
