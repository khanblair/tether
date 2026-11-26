package app.tether.session

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
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
 * session that is not in the database. The user can go back; nothing crashes. [reason], when set,
 * is the name of the failure, shown in small text.
 */
@Composable
fun SessionUnavailableScreen(
    onBack: () -> Unit,
    modifier: Modifier = Modifier,
    reason: String? = null,
) {
    Scaffold(
        modifier = modifier,
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = stringResource(R.string.session_unavailable_title),
                navigationIcon = { BackButton(onBack) },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(padding).padding(TetherSpace.space4),
            verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
        ) {
            Text(
                text = stringResource(R.string.session_unavailable_body),
                style = MaterialTheme.typography.bodyLarge,
                color = TetherTheme.colors.text2,
            )
            if (reason != null) {
                Text(
                    text = stringResource(R.string.session_unavailable_reason, reason),
                    style = TetherTheme.monoLabel,
                    color = TetherTheme.colors.text3,
                )
            }
        }
    }
}
