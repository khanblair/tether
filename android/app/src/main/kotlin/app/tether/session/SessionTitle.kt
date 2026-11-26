package app.tether.session

import androidx.compose.runtime.Composable
import androidx.compose.ui.res.stringResource
import app.tether.R

/**
 * [title], or [untitled] when [title] is blank. A new session has no title until its first
 * message.
 */
internal fun sessionTitleOr(
    title: String,
    untitled: String,
): String = if (title.isBlank()) untitled else title

/** The session title as the lists and the top bar show it. A blank title shows "New session". */
@Composable
fun sessionTitleText(title: String): String =
    sessionTitleOr(title, stringResource(R.string.session_untitled))
