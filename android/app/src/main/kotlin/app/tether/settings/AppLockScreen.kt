package app.tether.settings

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.PointerInputScope
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.clearAndSetSemantics
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.PrimaryButton
import app.tether.design.tokens.TetherSpace

/**
 * Draws [content] and, while [locked], an opaque lock screen over it. [content] stays composed in
 * both states. Leaving the composition would dispose the session and its streaming reply, and
 * unlocking would reopen the app at Home. While locked the content is hidden from accessibility,
 * and the cover takes every touch so none reaches the content.
 */
@Composable
fun AppLockLayers(
    locked: Boolean,
    onUnlock: () -> Unit,
    content: @Composable () -> Unit,
) {
    val layers = appLockLayers(locked)
    Box(modifier = Modifier.fillMaxSize()) {
        val hostModifier =
            if (layers.hostSemanticsHidden) {
                Modifier.fillMaxSize().clearAndSetSemantics {}
            } else {
                Modifier.fillMaxSize()
            }
        Box(modifier = hostModifier) { content() }
        if (layers.overlayVisible) {
            Box(
                modifier =
                    Modifier
                        .fillMaxSize()
                        .background(TetherTheme.colors.bg)
                        .pointerInput(Unit) { consumeEveryPointer() },
            ) {
                AppLockScreen(onUnlock = onUnlock)
            }
        }
    }
}

/** Takes every pointer change that reaches this layer, so no tap passes through to the content. */
private suspend fun PointerInputScope.consumeEveryPointer() {
    awaitPointerEventScope {
        while (true) {
            awaitPointerEvent().changes.forEach { it.consume() }
        }
    }
}

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
