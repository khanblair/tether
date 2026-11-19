package app.tether.home

import java.time.LocalDate
import java.time.format.TextStyle
import java.util.Locale

private const val THOUSAND = 1_000L
private const val TEN_THOUSAND = 10_000L
private const val MILLION = 1_000_000L
private const val TENTHS = 10L

/**
 * A token count in short form: "950", "1.2k", "212k", "1.2M". Values below zero count as zero.
 * Locale.ROOT keeps the decimal point the same on every device.
 */
fun formatTokens(tokens: Long): String {
    val value = tokens.coerceAtLeast(0L)
    return when {
        value < THOUSAND -> value.toString()
        value < TEN_THOUSAND -> oneDecimal(value, THOUSAND, "k")
        value < MILLION -> wholeThousands(value)
        else -> oneDecimal(value, MILLION, "M")
    }
}

/** Whole thousands, rounded. A value that rounds up to 1000k reads as "1M". */
private fun wholeThousands(value: Long): String {
    val thousands = (value + THOUSAND / 2) / THOUSAND
    return if (thousands >= THOUSAND) "1M" else "${thousands}k"
}

/** One decimal place, with a ".0" dropped, in [unit] steps. */
private fun oneDecimal(
    value: Long,
    unit: Long,
    suffix: String,
): String {
    val tenths = (value * TENTHS + unit / 2) / unit
    val whole = tenths / TENTHS
    val fraction = tenths % TENTHS
    return if (fraction == 0L) {
        "$whole$suffix"
    } else {
        String.format(Locale.ROOT, "%d.%d%s", whole, fraction, suffix)
    }
}

/**
 * The labels for the last seven days, oldest first. The last label is [today]. Each one is the
 * weekday name in [style] and [locale].
 */
fun weekdayLabels(
    today: LocalDate,
    locale: Locale,
    style: TextStyle,
): List<String> =
    (WEEK_DAYS - 1 downTo 0).map { daysAgo ->
        today.minusDays(daysAgo.toLong()).dayOfWeek.getDisplayName(style, locale)
    }

/** One model's share of the week's tokens. [fraction] is from 0 to 1. */
data class ModelShare(
    val model: String,
    val tokens: Long,
    val percent: Int,
    val fraction: Float,
)

/**
 * The models with their shares, largest first. Models with no tokens are dropped, and ties sort
 * by name. An empty list or a zero total gives an empty list.
 */
fun modelShares(byModel: List<ModelUsage>): List<ModelShare> {
    val used = byModel.filter { it.tokens > 0L }
    val total = used.sumOf { it.tokens }
    if (total <= 0L) return emptyList()
    return used
        .sortedWith(compareByDescending<ModelUsage> { it.tokens }.thenBy { it.model })
        .map { usage ->
            ModelShare(
                model = usage.model,
                tokens = usage.tokens,
                percent = Math.round(usage.tokens * PERCENT_SCALE / total).toInt(),
                fraction = usage.tokens.toFloat() / total,
            )
        }
}
