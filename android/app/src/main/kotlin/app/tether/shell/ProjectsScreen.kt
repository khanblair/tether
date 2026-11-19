package app.tether.shell

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
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
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.core.CoreClient
import app.tether.core.generated.CoreEventView
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.PrimaryButton
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSpace
import app.tether.nav.Route
import app.tether.updates.UpdatesUiState
import kotlinx.coroutines.flow.StateFlow

private const val EVENT_LOG_SIZE = 5

/**
 * P0 home: proves the Rust to Compose event path and lists every placeholder route. A banner
 * at the top names a newer version when the update flow knows one.
 */
@Composable
fun ProjectsScreen(
    core: CoreClient,
    updates: StateFlow<UpdatesUiState>,
    onOpenAbout: () -> Unit,
    onOpen: (Route) -> Unit,
) {
    val updateState by updates.collectAsStateWithLifecycle()
    val events by produceState(initialValue = emptyList<CoreEventView>(), core) {
        core.events.collect { value = (value + it).takeLast(EVENT_LOG_SIZE) }
    }

    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = { TetherTopBar(title = stringResource(Route.Projects.title)) },
    ) { padding ->
        LazyColumn(
            modifier = Modifier.padding(padding),
            contentPadding =
                androidx.compose.foundation.layout
                    .PaddingValues(TetherSpace.space4),
            verticalArrangement = Arrangement.spacedBy(TetherSpace.space4),
        ) {
            if (updateState.updateAvailable) {
                item {
                    InlineBanner(
                        text =
                            stringResource(
                                R.string.home_update_available,
                                updateState.newerVersion.orEmpty(),
                            ),
                        tone = BannerTone.Info,
                        actionLabel = stringResource(R.string.home_update_view),
                        onAction = onOpenAbout,
                    )
                }
            }
            item {
                Text(
                    text = stringResource(R.string.projects_intro_body),
                    style = MaterialTheme.typography.bodyLarge,
                    color = TetherTheme.colors.text2,
                )
            }
            item { CoreCard(core = core, events = events) }
            item {
                Text(
                    text = stringResource(R.string.projects_screens_title),
                    style = MaterialTheme.typography.titleSmall,
                )
            }
            items(Route.entries.filter { it != Route.Projects }, key = { it.path }) { route ->
                OutlinedButton(
                    onClick = { onOpen(route) },
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Text(text = stringResource(route.title))
                }
            }
        }
    }
}

@Composable
private fun CoreCard(
    core: CoreClient,
    events: List<CoreEventView>,
) {
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
            PrimaryButton(
                label = stringResource(R.string.projects_core_send_test_action),
                onClick = {
                    sent += 1
                    core.emitTestEvent("$message #$sent")
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
