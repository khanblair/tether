package app.tether

import android.content.Intent
import android.graphics.Color
import android.hardware.biometrics.BiometricManager
import android.hardware.biometrics.BiometricPrompt
import android.os.Build
import android.os.Bundle
import android.os.CancellationSignal
import androidx.activity.ComponentActivity
import androidx.activity.SystemBarStyle
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.design.TetherTheme
import app.tether.nav.TetherNavHost

/** The single activity. All screens are Compose destinations. */
class MainActivity : ComponentActivity() {
    /**
     * Asks the system for a biometric check before a saved key is shown, or before a session
     * changes to Bypass. [title] defaults to the key reveal title. On Android 11 and later the
     * screen lock PIN, pattern, or password is also accepted, so a phone with no enrolled
     * fingerprint can still pass the check. [onError] runs for a cancel and for a failure alike.
     */
    fun authenticateForKeyReveal(
        title: String? = null,
        onSuccess: () -> Unit,
        onError: () -> Unit,
    ) {
        val builder =
            BiometricPrompt
                .Builder(
                    this,
                ).setTitle(title ?: getString(R.string.settings_reveal_auth_title))
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            builder.setAllowedAuthenticators(
                BiometricManager.Authenticators.BIOMETRIC_STRONG or
                    BiometricManager.Authenticators.DEVICE_CREDENTIAL,
            )
        } else {
            builder.setNegativeButton(
                getString(R.string.settings_cancel),
                mainExecutor,
            ) { _, _ -> onError() }
        }
        builder.build().authenticate(
            CancellationSignal(),
            mainExecutor,
            object : BiometricPrompt.AuthenticationCallback() {
                override fun onAuthenticationSucceeded(
                    result: BiometricPrompt.AuthenticationResult,
                ) {
                    onSuccess()
                }

                override fun onAuthenticationError(
                    errorCode: Int,
                    errString: CharSequence,
                ) {
                    onError()
                }
            },
        )
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val module = (application as TetherApp).module
        module.shareInbox.accept(intent)
        setContent {
            val mode by module.themeMode.collectAsStateWithLifecycle()
            TetherTheme(mode = mode) {
                SystemBars(dark = TetherTheme.isDark)
                TetherNavHost(module = module)
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        (application as TetherApp).module.shareInbox.accept(intent)
    }

    /** Keeps system bar icons readable when the in-app theme differs from the system theme. */
    @Composable
    private fun SystemBars(dark: Boolean) {
        DisposableEffect(dark) {
            val style = SystemBarStyle.auto(Color.TRANSPARENT, Color.TRANSPARENT) { dark }
            enableEdgeToEdge(statusBarStyle = style, navigationBarStyle = style)
            onDispose {}
        }
    }
}
