package app.tether.updates

private const val VERSION_PARTS = 3

/** Reads `MAJOR.MINOR.PATCH`, with or without a leading `v`, into three numbers, or null. */
internal fun parseVersion(text: String): Triple<Long, Long, Long>? {
    val parts = text.removePrefix("v").split('.')
    val numbers =
        parts.mapNotNull { part ->
            part.takeIf { it.isNotEmpty() && it.all { char -> char in '0'..'9' } }?.toLongOrNull()
        }
    val valid = parts.size == VERSION_PARTS && numbers.size == VERSION_PARTS
    return if (valid) Triple(numbers[0], numbers[1], numbers[2]) else null
}

/**
 * True when [candidate] is a newer version than [current]. A version that does not parse is
 * never newer.
 */
internal fun isNewerVersion(
    candidate: String,
    current: String,
): Boolean {
    val next = parseVersion(candidate)
    val now = parseVersion(current)
    return next != null &&
        now != null &&
        compareValuesBy(next, now, { it.first }, { it.second }, { it.third }) > 0
}
