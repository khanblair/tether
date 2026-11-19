package app.tether.core

import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.security.keystore.StrongBoxUnavailableException
import app.tether.core.generated.SecretVault
import app.tether.core.generated.SecretsPlatform
import app.tether.core.generated.SecretsPlatformFailure
import java.io.File
import java.security.GeneralSecurityException
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * Wraps secrets with an AES-256-GCM key held in the Android Keystore, in StrongBox when the
 * device has it. The key needs no user authentication, so a background turn can use it. The UI
 * asks for biometrics before it reveals a secret.
 *
 * The secret's reference is passed in as authenticated data, so a wrapped file moved to another
 * reference fails to unwrap. The wrapped form is the 12-byte IV followed by the ciphertext and tag.
 */
class KeystoreSecretsPlatform(
    private val alias: String = DEFAULT_ALIAS,
) : SecretsPlatform {
    override fun wrap(
        plaintext: ByteArray,
        context: ByteArray,
    ): ByteArray =
        guarded("wrap") {
            val cipher = Cipher.getInstance(TRANSFORMATION)
            cipher.init(Cipher.ENCRYPT_MODE, key())
            cipher.updateAAD(context)
            cipher.iv + cipher.doFinal(plaintext)
        }

    override fun unwrap(
        wrapped: ByteArray,
        context: ByteArray,
    ): ByteArray =
        guarded("unwrap") {
            require(wrapped.size >= IV_BYTES + TAG_BYTES) { "wrapped bytes are too short" }
            val cipher = Cipher.getInstance(TRANSFORMATION)
            val params = GCMParameterSpec(TAG_BITS, wrapped, 0, IV_BYTES)
            cipher.init(Cipher.DECRYPT_MODE, key(), params)
            cipher.updateAAD(context)
            cipher.doFinal(wrapped, IV_BYTES, wrapped.size - IV_BYTES)
        }

    @Synchronized
    private fun key(): SecretKey {
        val store = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
        return (store.getKey(alias, null) as SecretKey?) ?: generateKey()
    }

    private fun generateKey(): SecretKey =
        try {
            generateKey(strongBox = true)
        } catch (_: StrongBoxUnavailableException) {
            generateKey(strongBox = false)
        }

    private fun generateKey(strongBox: Boolean): SecretKey {
        val spec =
            KeyGenParameterSpec
                .Builder(alias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(KEY_BITS)
                .setUserAuthenticationRequired(false)
                .setIsStrongBoxBacked(strongBox)
                .build()
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, ANDROID_KEYSTORE)
        generator.init(spec)
        return generator.generateKey()
    }

    /** Reports only the failure kind to Rust, never the Keystore message. */
    private inline fun <T> guarded(
        action: String,
        block: () -> T,
    ): T =
        try {
            block()
        } catch (e: GeneralSecurityException) {
            throw failure(action, e)
        } catch (e: IllegalArgumentException) {
            throw failure(action, e)
        }

    private fun failure(
        action: String,
        cause: Exception,
    ): SecretsPlatformFailure =
        SecretsPlatformFailure
            .Failed("keystore $action failed: ${cause.javaClass.simpleName}")
            .apply { initCause(cause) }

    companion object {
        /** Keystore alias of the key that wraps every secret. */
        const val DEFAULT_ALIAS = "tether_secrets_v1"

        private const val ANDROID_KEYSTORE = "AndroidKeyStore"
        private const val TRANSFORMATION = "AES/GCM/NoPadding"
        private const val KEY_BITS = 256
        private const val TAG_BITS = 128
        private const val TAG_BYTES = TAG_BITS / 8
        private const val IV_BYTES = 12
    }
}

/** Opens the secret vault over [secretsDir] (the app's private `files/secrets` folder). */
fun keystoreSecretVault(
    secretsDir: File,
    platform: SecretsPlatform = KeystoreSecretsPlatform(),
): SecretVault = SecretVault(secretsDir.absolutePath, platform)
