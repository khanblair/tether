package app.tether.settings

import app.tether.core.generated.ProviderView

/**
 * The model a new session starts with when the user has chosen one: a provider and its model id.
 */
data class DefaultModel(
    val provider: ProviderView,
    val modelId: String,
)

/** Reads the app-wide default model. Null means no default is saved. */
interface AppDefaultSource {
    /** The saved app default model, or null when none is saved. */
    fun defaultModel(): DefaultModel?
}
