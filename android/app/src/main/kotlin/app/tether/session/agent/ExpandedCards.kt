package app.tether.session.agent

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.Saver
import androidx.compose.runtime.saveable.listSaver
import androidx.compose.runtime.setValue

/** The ids of the tool cards the user expanded. A group uses "group:" plus its first call's id. */
class ExpandedCards(
    initial: Set<String> = emptySet(),
) {
    /** The expanded ids. */
    var ids: Set<String> by mutableStateOf(initial)
        private set

    /** Expands the card [id], or collapses it when it is already expanded. */
    fun toggle(id: String) {
        ids = if (id in ids) ids - id else ids + id
    }
}

/** Saves the expanded ids across configuration changes. */
val expandedCardsSaver: Saver<ExpandedCards, Any> =
    listSaver(save = { it.ids.toList() }, restore = { ExpandedCards(it.toSet()) })
