package app.tether.settings

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.PrimaryButton
import app.tether.design.tokens.TetherSpace

/** The only screen shown while the app lock is on. It has one action: unlock. */
@Composable
fun AppLockScreen(onUnlock: () -> Unit) {
    Scaffold(containerColor = TetherTheme.colors.bg) { padding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(padding).padding(TetherSpace.space4),
            verticalArrangement =
                Arrangement.spacedBy(
                    TetherSpace.space3,
                    Alignment.CenterVertically,
                ),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(
                text = stringResource(R.string.settings_area_lock_title),
                style = MaterialTheme.typography.titleLarge,
                color = TetherTheme.colors.text,
            )
            Text(
                text = stringResource(R.string.settings_area_lock_body),
                style = MaterialTheme.typography.bodyMedium,
                color = TetherTheme.colors.text2,
            )
            PrimaryButton(label = stringResource(R.string.settings_area_unlock), onClick = onUnlock)
        }
    }
}
