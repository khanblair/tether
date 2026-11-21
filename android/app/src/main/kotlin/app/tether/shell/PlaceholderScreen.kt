package app.tether.shell

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
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.TetherIconButton
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherSpace
import app.tether.nav.Route

/** Stand-in for a screen that ships in a later phase. */
@Composable
fun PlaceholderScreen(
    route: Route,
    onBack: () -> Unit,
) {
    PlaceholderScreen(
        title = stringResource(route.title),
        explanation = stringResource(R.string.placeholder_body),
        onBack = onBack,
    )
}

/**
 * A screen for a feature the app cannot do yet: its name, one sentence on what it needs, and a back
 * arrow.
 */
@Composable
fun PlaceholderScreen(
    title: String,
    explanation: String,
    onBack: () -> Unit,
) {
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = title,
                navigationIcon = { BackButton(onBack) },
            )
        },
    ) { padding ->
        Box(modifier = Modifier.fillMaxSize().padding(padding).padding(TetherSpace.space4)) {
            Text(
                text = explanation,
                style = MaterialTheme.typography.bodyLarge,
                color = TetherTheme.colors.text2,
            )
        }
    }
}

/** The back arrow in a top bar. The label is the content description for TalkBack. */
@Composable
fun BackButton(onBack: () -> Unit) {
    TetherIconButton(
        icon = TetherIcons.ArrowLeft,
        contentDescription = stringResource(R.string.nav_back_label),
        onClick = onBack,
    )
}
