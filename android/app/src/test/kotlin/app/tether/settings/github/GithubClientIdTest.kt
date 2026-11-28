package app.tether.settings.github

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class GithubClientIdTest {
    @Test
    fun a_client_id_reaches_the_runtime_trimmed() {
        val given = mutableListOf<String>()

        configureDeviceFlow("  Iv1.abc123  ") { given += it }

        assertEquals(listOf("Iv1.abc123"), given)
    }

    @Test
    fun a_blank_client_id_leaves_sign_in_with_github_off() {
        val given = mutableListOf<String>()

        configureDeviceFlow("") { given += it }
        configureDeviceFlow("   ") { given += it }

        assertTrue(given.isEmpty())
    }
}
