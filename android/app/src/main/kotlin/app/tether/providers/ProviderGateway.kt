package app.tether.providers

import app.tether.core.generated.ProviderSettings
import app.tether.core.generated.ProviderStatusView
import app.tether.core.generated.ProviderView

/**
 * What the provider screens need from the Rust core. Every call blocks, so callers run it off the
 * main thread. A fake implements this in unit tests, so no test loads the native library.
 */
interface ProviderGateway {
    /** The saved status, model count, and test time for [provider]. */
    fun status(provider: ProviderView): ProviderStatusView

    /** Whether a key is saved for [provider]. */
    fun hasKey(provider: ProviderView): Boolean

    /** Saves [key] for [provider]. The core trims it and rejects an empty key. */
    fun setKey(
        provider: ProviderView,
        key: String,
    )

    /** Removes the saved key and the cached model list for [provider]. */
    fun removeKey(provider: ProviderView)

    /**
     * The saved key text, or null when none is saved. Callers must ask for a biometric check
     * first.
     */
    fun revealKey(provider: ProviderView): String?

    /** The exact URL a connection test calls for [provider]. */
    fun testTarget(provider: ProviderView): String

    /** Calls the provider's model list with the saved key and saves the result. */
    fun testConnection(provider: ProviderView): ProviderStatusView
}

/**
 * The real gateway: forwards each call to the UniFFI [ProviderSettings] object. The object is
 * created on the first call, which runs off the main thread, so opening the database never blocks
 * the UI.
 */
class CoreProviderGateway(
    private val lazySettings: Lazy<ProviderSettings>,
) : ProviderGateway {
    private val settings: ProviderSettings by lazySettings

    override fun status(provider: ProviderView): ProviderStatusView = settings.status(provider)

    override fun hasKey(provider: ProviderView): Boolean = settings.hasKey(provider)

    override fun setKey(
        provider: ProviderView,
        key: String,
    ) = settings.setKey(provider, key)

    override fun removeKey(provider: ProviderView) = settings.removeKey(provider)

    override fun revealKey(provider: ProviderView): String? = settings.revealKey(provider)

    override fun testTarget(provider: ProviderView): String = settings.testTarget(provider)

    override fun testConnection(provider: ProviderView): ProviderStatusView =
        settings.testConnection(provider)
}
