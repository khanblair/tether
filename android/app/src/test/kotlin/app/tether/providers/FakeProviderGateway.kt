package app.tether.providers

import app.tether.core.generated.ConnectionStatusView
import app.tether.core.generated.ProviderStatusView
import app.tether.core.generated.ProviderView

/** A gateway for unit tests. It keeps keys and statuses in memory and records every call. */
class FakeProviderGateway : ProviderGateway {
    val keys = mutableMapOf<ProviderView, String>()
    val statuses = mutableMapOf<ProviderView, ProviderStatusView>()
    val calls = mutableListOf<String>()

    /** Throws this from the next call of any kind, then clears itself. */
    var failNext: Exception? = null

    /** What [testConnection] returns. */
    var testResult = ProviderStatusView(ConnectionStatusView.OK, 3u, 1_000L)

    private fun record(call: String) {
        calls += call
        failNext?.let {
            failNext = null
            throw it
        }
    }

    override fun status(provider: ProviderView): ProviderStatusView {
        record("status")
        return statuses[provider] ?: ProviderStatusView(ConnectionStatusView.UNSET, 0u, null)
    }

    override fun hasKey(provider: ProviderView): Boolean {
        record("hasKey")
        return provider in keys
    }

    override fun setKey(
        provider: ProviderView,
        key: String,
    ) {
        record("setKey")
        keys[provider] = key.trim()
        statuses[provider] = ProviderStatusView(ConnectionStatusView.UNSET, 0u, null)
    }

    override fun removeKey(provider: ProviderView) {
        record("removeKey")
        keys.remove(provider)
        statuses.remove(provider)
    }

    override fun revealKey(provider: ProviderView): String? {
        record("revealKey")
        return keys[provider]
    }

    override fun testTarget(provider: ProviderView): String {
        record("testTarget")
        return "https://example.test/${provider.name.lowercase()}/models"
    }

    override fun testConnection(provider: ProviderView): ProviderStatusView {
        record("testConnection")
        statuses[provider] = testResult
        return testResult
    }
}
