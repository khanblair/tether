package app.tether.home

/** A percent is a hundredth of the earlier number. */
internal const val PERCENT_SCALE = 100.0

/** How a number compares with an earlier one, as the cards word it. */
sealed interface Change {
    /** The two numbers are equal, or a percent change rounds to zero. */
    data object Same : Change

    /** The earlier number is zero, so no percent change can be shown. */
    data object NoBaseline : Change

    /** The number is higher by [n]. */
    data class MoreCount(
        val n: Int,
    ) : Change

    /** The number is lower by [n]. */
    data class LessCount(
        val n: Int,
    ) : Change

    /** The number is higher by [percent] percent. */
    data class MorePercent(
        val percent: Int,
    ) : Change

    /** The number is lower by [percent] percent. */
    data class LessPercent(
        val percent: Int,
    ) : Change
}

/** The change from [previous] to [current] as a count. */
fun countChange(
    current: Int,
    previous: Int,
): Change {
    val difference = current - previous
    return when {
        difference == 0 -> Change.Same
        difference > 0 -> Change.MoreCount(difference)
        else -> Change.LessCount(-difference)
    }
}

/** The change from [previous] to [current] as a rounded percent. */
fun percentChange(
    current: Long,
    previous: Long,
): Change {
    if (previous == 0L) {
        return if (current == 0L) Change.Same else Change.NoBaseline
    }
    val percent = Math.round((current - previous) * PERCENT_SCALE / previous)
    return when {
        percent == 0L -> Change.Same
        percent > 0L -> Change.MorePercent(percent.toInt())
        else -> Change.LessPercent((-percent).toInt())
    }
}
