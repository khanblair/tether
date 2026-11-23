package app.tether.session.compose

import android.content.Context
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalContext
import app.tether.R

/**
 * The [CommandWords] for the current locale, read from `strings_compose.xml`. The strings are
 * read once per composition context.
 */
@Composable
fun rememberCommandWords(): CommandWords {
    val context = LocalContext.current.applicationContext
    return remember(context) { commandWordsFrom(context) }
}

/** Builds [CommandWords] from [context]'s string resources. */
internal fun commandWordsFrom(context: Context): CommandWords =
    CommandWords(
        unknown = { name, suggestion ->
            if (suggestion == null) {
                context.getString(R.string.compose_command_unknown, name)
            } else {
                context.getString(R.string.compose_command_unknown_suggestion, name, suggestion)
            }
        },
        notAvailableYet = context.getString(R.string.compose_command_not_yet),
        failed = context.getString(R.string.compose_command_failed),
    )
