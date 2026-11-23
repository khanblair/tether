package app.tether.settings.budgets

import java.util.Locale

/** One dollar in USD micros. */
const val MICROS_PER_DOLLAR = 1_000_000L

/** One cent in USD micros. */
const val MICROS_PER_CENT = 10_000L

/** The smallest limit the core accepts: one cent. */
const val MIN_LIMIT_MICROS = 10_000L

/** The largest limit the core accepts: ten thousand dollars. */
const val MAX_LIMIT_MICROS = 10_000_000_000L

/** Cents in one dollar. */
private const val CENTS_PER_DOLLAR = 100L

/** Whole dollars allowed in a typed amount. The largest limit has five digits. */
private const val MAX_WHOLE_DIGITS = 5

/** Digits after the decimal mark: the amount is in cents at most. */
private const val MAX_DECIMALS = 2

private val AMOUNT = Regex("""(\d+)(?:[.,](\d*))?""")

/** Why a typed amount is not a limit. Each one has a message in the strings file. */
enum class DollarProblem {
    NotANumber,
    TooManyDecimals,
    Negative,
    BelowMinimum,
    AboveMaximum,
}

/** What one limit field holds: nothing (no limit), a valid amount, or a problem. */
sealed interface DollarInput {
    /** The field is empty. That means no limit. */
    data object Blank : DollarInput

    /** A limit of [micros], inside the core's range. */
    data class Valid(
        val micros: Long,
    ) : DollarInput

    /** The text is not a limit. [problem] says why. */
    data class Invalid(
        val problem: DollarProblem,
    ) : DollarInput
}

/**
 * Reads one limit field. A blank field is no limit. Accepts whole dollars and up to two decimal
 * places after a dot or a comma, for example `2`, `2.5`, `2,50` or `0.01`. Rejects a leading
 * minus sign, a dollar sign, letters, more than two decimals, and amounts outside the core's range.
 */
fun readDollars(text: String): DollarInput {
    val trimmed = text.trim()
    return if (trimmed.isEmpty()) DollarInput.Blank else checkedAmount(trimmed)
}

/** Reads text that is not blank: the sign and the shape first, then the digits and the range. */
private fun checkedAmount(text: String): DollarInput {
    val match = AMOUNT.matchEntire(text)
    return when {
        text.startsWith("-") -> DollarInput.Invalid(DollarProblem.Negative)
        match == null -> DollarInput.Invalid(DollarProblem.NotANumber)
        else -> withinRange(match.groupValues[1], match.groupValues[2])
    }
}

/** Checks the decimals and the whole-dollar size, then the core's range. */
private fun withinRange(
    whole: String,
    fraction: String,
): DollarInput {
    val dollars = whole.trimStart('0')
    return when {
        fraction.length > MAX_DECIMALS -> DollarInput.Invalid(DollarProblem.TooManyDecimals)
        dollars.length > MAX_WHOLE_DIGITS -> DollarInput.Invalid(DollarProblem.AboveMaximum)
        else -> rangeChecked(micros(dollars, fraction))
    }
}

/** The micros for whole dollars and the decimal digits after the mark. */
private fun micros(
    dollars: String,
    fraction: String,
): Long {
    val wholeDollars = dollars.ifEmpty { "0" }.toLong()
    val cents = fraction.padEnd(MAX_DECIMALS, '0').toLong()
    return wholeDollars * MICROS_PER_DOLLAR + cents * MICROS_PER_CENT
}

private fun rangeChecked(micros: Long): DollarInput =
    when {
        micros < MIN_LIMIT_MICROS -> DollarInput.Invalid(DollarProblem.BelowMinimum)
        micros > MAX_LIMIT_MICROS -> DollarInput.Invalid(DollarProblem.AboveMaximum)
        else -> DollarInput.Valid(micros)
    }

/** The limit in micros for [text], or null when the field is blank or not a valid limit. */
fun parseDollars(text: String): Long? = (readDollars(text) as? DollarInput.Valid)?.micros

/**
 * Money for people to read: "$0.42", "$1,250.00", or [lessThanCent] for a small positive amount
 * (the caller passes the words from the string resources). Zero reads as "$0.00". Amounts round
 * half up to the cent. Locale.US keeps the separators the same on every device.
 */
fun formatDollars(
    micros: Long,
    lessThanCent: String,
): String {
    val value = micros.coerceAtLeast(0L)
    return when {
        value == 0L -> "\$0.00"
        value < MIN_LIMIT_MICROS -> lessThanCent
        else -> {
            val (dollars, cents) = splitCents(value)
            String.format(Locale.US, "\$%,d.%02d", dollars, cents)
        }
    }
}

/** The amount for an edit field: plain digits with two decimals, such as "1250.00". */
fun editableDollars(micros: Long): String {
    val (dollars, cents) = splitCents(micros.coerceAtLeast(0L))
    return String.format(Locale.US, "%d.%02d", dollars, cents)
}

/** The edit text for a saved limit, or blank when there is no limit. */
fun editableOrBlank(micros: Long?): String = micros?.let(::editableDollars).orEmpty()

/** Whole dollars and the cents left over, for micros rounded half up to the cent. */
private fun splitCents(micros: Long): Pair<Long, Long> {
    val cents = (micros + MICROS_PER_CENT / 2) / MICROS_PER_CENT
    return cents / CENTS_PER_DOLLAR to cents % CENTS_PER_DOLLAR
}
