package app.tether.appearance

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/** Holds the appearance choices for the whole app. Every change is saved at once. */
class AppearanceController(
    private val store: AppearanceStore,
) {
    private val state = MutableStateFlow(store.load().normalized())

    /** The current choices. It starts from the saved values. */
    val appearance: StateFlow<Appearance> = state.asStateFlow()

    /** Sets the interface size, in percent, and saves it. An unknown size snaps to the nearest. */
    fun setInterfacePercent(percent: Int) {
        update { it.copy(interfacePercent = percent) }
    }

    /** Sets the text size, in percent, and saves it. An unknown size becomes Follow system. */
    fun setTextPercent(percent: Int) {
        update { it.copy(textPercent = percent) }
    }

    /** Sets the accent color as ARGB, or clears it with null, and saves it. */
    fun setAccent(argb: Int?) {
        update { it.copy(accentArgb = argb) }
    }

    /** Restores the defaults and saves them. */
    fun reset() {
        update { Appearance() }
    }

    private fun update(change: (Appearance) -> Appearance) {
        val next = change(state.value).normalized()
        state.value = next
        store.save(next)
    }
}
