package app.tether.core

import app.tether.core.generated.SecretsPlatformFailure
import kotlinx.coroutines.CancellationException
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test
import java.security.ProviderException

class KeystoreFailureTest {
    @Test
    fun failure_names_the_action_and_the_class_only() {
        val cause = ProviderException("secret keystore detail sk-test-not-real")

        val failure = keystoreFailure("wrap", cause)

        assertTrue(failure is SecretsPlatformFailure.Failed)
        assertEquals(
            "keystore wrap failed: ProviderException",
            (failure as SecretsPlatformFailure.Failed).detail,
        )
        assertFalse(failure.message.orEmpty().contains("sk-test-not-real"))
        assertSame(cause, failure.cause)
    }

    @Test
    fun an_illegal_state_error_maps_to_the_same_failure_kind() {
        val failure = keystoreFailure("unwrap", IllegalStateException("boom"))

        assertEquals(
            "keystore unwrap failed: IllegalStateException",
            (failure as SecretsPlatformFailure.Failed).detail,
        )
    }

    @Test
    fun a_cancellation_passes_through_unchanged() {
        val cancelled = CancellationException("stopped")

        assertSame(cancelled, keystoreFailure("wrap", cancelled))
    }
}
