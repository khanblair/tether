package app.tether.updates

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.core.CoreClient
import app.tether.core.generated.CoreEventView
import app.tether.design.TetherTheme
import app.tether.design.components.PrimaryButton
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSpace
import app.tether.messages.AppMessages

private const val EVENT_LOG_SIZE = 5

/**
 * The Rust core card for debug builds: the core version, the recent core events, and a button
 * that sends a test event through the core and back.
 */
@Composable
internal fun CoreCard(
    core: CoreClient,
    messages: AppMessages,
) {
    val events by produceState(initialValue = emptyList<CoreEventView>(), core) {
        core.events.collect { value = (value + it).takeLast(EVENT_LOG_SIZE) }
    }
    var sent by remember { mutableIntStateOf(0) }
    Surface(
        color = TetherTheme.colors.surface,
        shape = RoundedCornerShape(TetherRadius.md),
    ) {
        Column(
            modifier = Modifier.fillMaxWidth().padding(TetherSpace.space3),
            verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
        ) {
            Text(
                text = stringResource(R.string.projects_core_title),
                style = MaterialTheme.typography.titleSmall,
            )
            Text(
                text = stringResource(R.string.projects_core_version, core.coreVersion()),
                style = TetherTheme.monoLabel,
                color = TetherTheme.colors.accentInk,
            )
            Text(
                text = stringResource(R.string.projects_core_events_title),
                style = MaterialTheme.typography.labelSmall,
                color = TetherTheme.colors.text3,
            )
            if (events.isEmpty()) {
                Text(
                    text = stringResource(R.string.projects_core_events_empty),
                    style = MaterialTheme.typography.bodySmall,
                    color = TetherTheme.colors.text3,
                )
            }
            events.forEach { event ->
                Text(text = describe(event), style = TetherTheme.monoSmall)
            }
            val message = stringResource(R.string.projects_core_test_message)
            val sentToast = stringResource(R.string.core_test_event_sent)
            PrimaryButton(
                label = stringResource(R.string.projects_core_send_test_action),
                onClick = {
                    sent += 1
                    core.emitTestEvent("$message #$sent")
                    messages.show(sentToast)
                },
            )
        }
    }
}

@Composable
private fun describe(event: CoreEventView): String =
    when (event) {
        is CoreEventView.Ready -> stringResource(R.string.core_event_ready, event.version)
        is CoreEventView.Test -> stringResource(R.string.core_event_test, event.message)
        else -> stringResource(R.string.core_event_other)
    }
