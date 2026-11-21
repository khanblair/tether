package app.tether.memory

/**
 * The two memory files. [cap] is the character limit the core enforces for the file. The core
 * does not report it on read, so it is mirrored here; a change in the core must change this too.
 */
enum class MemoryFile(
    val cap: Int,
) {
    /** Agent notes, kept in MEMORY.md. */
    Memory(MEMORY_CAP),

    /** Preferences about the user, kept in USER.md. */
    User(USER_CAP),
}

private const val MEMORY_CAP = 2_200
private const val USER_CAP = 1_375
private const val SEPARATOR = "\n§\n"

/** Splits a memory file into its entries. Blank pieces are dropped, as the core drops them. */
fun splitEntries(content: String): List<String> =
    content.split(SEPARATOR).filter { it.isNotBlank() }

/** The number of Unicode characters in [text], which is what the core counts against the cap. */
fun characterCount(text: String): Int = text.codePointCount(0, text.length)

/** Digits with a comma every three places, such as "1,240". Independent of the device locale. */
fun groupDigits(value: Int): String =
    value
        .toString()
        .reversed()
        .chunked(DIGIT_GROUP)
        .joinToString(",")
        .reversed()

/** The share of [cap] that [used] takes, from 0 to 1, for the meter. */
fun capFraction(
    used: Int,
    cap: Int,
): Float = (used.toFloat() / cap).coerceIn(0f, 1f)

private const val DIGIT_GROUP = 3
