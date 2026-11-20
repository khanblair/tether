package app.tether.session

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherSpace
import app.tether.shell.BackButton

/**
 * Shown when a session cannot be opened, for example when it was deleted or the link points at a
 * session that is not in the database. The user can go back; nothing crashes.
 */
@Composable
fun SessionUnavailableScreen(onBack: () -> Unit) {
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = stringResource(R.string.session_unavailable_title),
                navigationIcon = { BackButton(onBack) },
            )
        },
    ) { padding ->
        Box(modifier = Modifier.fillMaxSize().padding(padding).padding(TetherSpace.space4)) {
            Text(
                text = stringResource(R.string.session_unavailable_body),
                style = MaterialTheme.typography.bodyLarge,
                color = TetherTheme.colors.text2,
            )
        }
    }
}
