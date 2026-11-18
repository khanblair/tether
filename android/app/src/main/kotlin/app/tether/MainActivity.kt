package app.tether

import android.graphics.Color
import android.os.Bundle
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
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val module = (application as TetherApp).module
        setContent {
            val mode by module.themeMode.collectAsStateWithLifecycle()
            TetherTheme(mode = mode) {
                SystemBars(dark = TetherTheme.isDark)
                TetherNavHost(module = module)
            }
        }
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
