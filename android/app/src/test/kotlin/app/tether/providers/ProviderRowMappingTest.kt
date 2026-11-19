package app.tether.providers

import app.tether.R
import app.tether.core.generated.ConnectionStatusView
import app.tether.core.generated.ProviderView
import app.tether.design.components.ProviderRowStatus
import org.junit.Assert.assertEquals
import org.junit.Test

class ProviderRowMappingTest {
    private val testedAt = 1_000L
    private val relative = "2 minutes ago"

    private fun summary(
        status: ConnectionStatusView,
        hasKey: Boolean,
        tested: Boolean,
    ) = ProviderSummary(
        provider = ProviderView.ANTHROPIC,
        hasKey = hasKey,
        status = if (hasKey) status else ConnectionStatusView.UNSET,
        modelCount = 14,
        testedAtMs = if (tested) testedAt else null,
    )

    @Test
    fun no_key_maps_to_unset_for_every_status() {
        ConnectionStatusView.entries.forEach { status ->
            val row = providerRowFor(summary(status, hasKey = false, tested = true), relative)
            assertEquals(ProviderRowStatus.Unset, row.status)
            assertEquals(ProviderText.Plain(R.string.providers_subtitle_no_key), row.subtitle)
            assertEquals(R.string.providers_status_no_key, row.statusDescription)
        }
    }

    @Test
    fun ok_with_key_and_test_time_shows_count_and_time() {
        val row =
            providerRowFor(summary(ConnectionStatusView.OK, hasKey = true, tested = true), relative)
        assertEquals(ProviderRowStatus.Ok, row.status)
        assertEquals(
            ProviderText.Plural(R.plurals.providers_subtitle_ok, 14, listOf(14, relative)),
            row.subtitle,
        )
        assertEquals(R.string.providers_status_connected, row.statusDescription)
    }

    @Test
    fun ok_with_key_and_no_test_time_shows_count_only() {
        val row =
            providerRowFor(summary(ConnectionStatusView.OK, hasKey = true, tested = false), null)
        assertEquals(ProviderRowStatus.Ok, row.status)
        assertEquals(ProviderText.Plural(R.plurals.providers_models, 14, listOf(14)), row.subtitle)
        assertEquals(R.string.providers_status_connected, row.statusDescription)
    }

    @Test
    fun unset_with_key_is_not_tested() {
        val row =
            providerRowFor(summary(ConnectionStatusView.UNSET, hasKey = true, tested = false), null)
        assertEquals(ProviderRowStatus.Unset, row.status)
        assertEquals(ProviderText.Plain(R.string.providers_subtitle_key_not_tested), row.subtitle)
        assertEquals(R.string.providers_status_not_tested, row.statusDescription)
    }

    @Test
    fun testing_with_key_is_not_tested() {
        val row =
            providerRowFor(
                summary(ConnectionStatusView.TESTING, hasKey = true, tested = false),
                null,
            )
        assertEquals(ProviderRowStatus.Unset, row.status)
        assertEquals(ProviderText.Plain(R.string.providers_subtitle_key_not_tested), row.subtitle)
        assertEquals(R.string.providers_status_not_tested, row.statusDescription)
    }

    @Test
    fun bad_key_with_key_is_key_rejected() {
        val row =
            providerRowFor(
                summary(ConnectionStatusView.BAD_KEY, hasKey = true, tested = true),
                relative,
            )
        assertEquals(ProviderRowStatus.BadKey, row.status)
        assertEquals(ProviderText.Plain(R.string.providers_subtitle_bad_key), row.subtitle)
        assertEquals(R.string.providers_status_key_rejected, row.statusDescription)
    }

    @Test
    fun network_error_with_key_is_network_error() {
        val row =
            providerRowFor(
                summary(ConnectionStatusView.NETWORK_ERROR, hasKey = true, tested = true),
                relative,
            )
        assertEquals(ProviderRowStatus.NetworkError, row.status)
        assertEquals(ProviderText.Plain(R.string.providers_subtitle_network), row.subtitle)
        assertEquals(R.string.providers_status_network_error, row.statusDescription)
    }

    @Test
    fun blocked_with_key_is_blocked() {
        val row =
            providerRowFor(
                summary(ConnectionStatusView.BLOCKED, hasKey = true, tested = true),
                relative,
            )
        assertEquals(ProviderRowStatus.Blocked, row.status)
        assertEquals(ProviderText.Plain(R.string.providers_subtitle_blocked), row.subtitle)
        assertEquals(R.string.providers_status_blocked, row.statusDescription)
    }

    @Test
    fun provider_names_match_the_list_order() {
        assertEquals(R.string.providers_name_anthropic, providerNameRes(ProviderView.ANTHROPIC))
        assertEquals(R.string.providers_name_openai, providerNameRes(ProviderView.OPEN_AI))
        assertEquals(R.string.providers_name_gemini, providerNameRes(ProviderView.GEMINI))
        assertEquals(R.string.providers_name_deepseek, providerNameRes(ProviderView.DEEP_SEEK))
        assertEquals(R.string.providers_name_openrouter, providerNameRes(ProviderView.OPEN_ROUTER))
    }
}
