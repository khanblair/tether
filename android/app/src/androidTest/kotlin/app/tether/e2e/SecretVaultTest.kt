package app.tether.e2e

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.tether.core.KeystoreSecretsPlatform
import app.tether.core.generated.SecretVaultException
import app.tether.core.keystoreSecretVault
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.security.KeyStore
import java.util.UUID
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors

/**
 * Proves secrets survive a restart, and that the file holds no plaintext, with the real Android
 * Keystore. A new vault over the same folder and the same Keystore alias stands in for a restart:
 * it shares nothing with the first one except what is on disk and in the Keystore.
 */
@RunWith(AndroidJUnit4::class)
class SecretVaultTest {
    private val alias = "tether_secrets_test_${UUID.randomUUID()}"
    private val extraAliases = mutableListOf<String>()
    private lateinit var dir: File

    @Before
    fun setUp() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        dir = File(context.filesDir, "secrets-test-${UUID.randomUUID()}")
    }

    @After
    fun tearDown() {
        dir.deleteRecursively()
        val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (extraAliases + alias).forEach { store.deleteEntry(it) }
    }

    private fun newVault() = keystoreSecretVault(dir, KeystoreSecretsPlatform(alias))

    @Test
    fun a_stored_key_is_read_back_after_a_restart() {
        newVault().use { it.put("provider-anthropic", SECRET) }

        val restarted = newVault()

        assertEquals(SECRET, restarted.reveal("provider-anthropic"))
        assertTrue(restarted.has("provider-anthropic"))
    }

    @Test
    fun the_file_contains_no_plaintext() {
        newVault().use { it.put("provider-anthropic", SECRET) }

        val bytes = File(dir, "provider-anthropic.bin").readBytes()

        assertFalse(String(bytes, Charsets.ISO_8859_1).contains("TESTVALUE"))
        assertTrue(bytes.size > SECRET.length)
    }

    @Test
    fun a_file_moved_to_another_reference_does_not_reveal() {
        newVault().use { it.put("github-token", SECRET) }
        File(dir, "github-token.bin").copyTo(File(dir, "provider-anthropic.bin"))

        assertThrows(SecretVaultException::class.java) { newVault().reveal("provider-anthropic") }
    }

    @Test
    fun a_deleted_key_is_gone() {
        val vault = newVault()
        vault.put("provider-gemini", SECRET)

        vault.delete("provider-gemini")

        assertNull(vault.reveal("provider-gemini"))
    }

    @Test
    fun platforms_created_at_the_same_time_share_one_key() {
        repeat(RACE_ROUNDS) {
            val raceAlias = "tether_secrets_race_${UUID.randomUUID()}"
            extraAliases += raceAlias
            val blobs = wrapAtOnce(raceAlias)

            val reader = KeystoreSecretsPlatform(raceAlias)
            blobs.forEach { assertEquals(SECRET, String(reader.unwrap(it, CONTEXT))) }
        }
    }

    /** Wraps [SECRET] from several threads at once, each with its own platform on [raceAlias]. */
    private fun wrapAtOnce(raceAlias: String): List<ByteArray> {
        val start = CountDownLatch(1)
        val pool = Executors.newFixedThreadPool(RACE_THREADS)
        try {
            val jobs =
                List(RACE_THREADS) {
                    pool.submit<ByteArray> {
                        start.await()
                        KeystoreSecretsPlatform(raceAlias).wrap(SECRET.toByteArray(), CONTEXT)
                    }
                }
            start.countDown()
            return jobs.map { it.get() }
        } finally {
            pool.shutdown()
        }
    }

    private companion object {
        val CONTEXT = "provider-anthropic".toByteArray()
        const val RACE_ROUNDS = 5
        const val RACE_THREADS = 8

        const val SECRET = "sk-ant-api03-TESTVALUE-not-a-real-key"
    }
}
