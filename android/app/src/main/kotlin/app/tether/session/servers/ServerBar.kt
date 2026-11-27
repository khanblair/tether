package app.tether.session.servers

import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherIconButton
import app.tether.design.components.TetherTextButton
import app.tether.design.components.TextButtonTone
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

/** The compact process bar above the composer and its optional preview-ready line. */
@Composable
fun ServerBar(
    state: ServerState,
    preview: ServerPreviewTarget?,
    onPreview: (ServerPreviewTarget) -> Unit,
    onStop: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    val running = state.running
    if (running.isEmpty() && preview !is ServerPreviewTarget.File) return
    var expanded by remember(running.size) { mutableStateOf(false) }
    val shape = RoundedCornerShape(TetherRadius.md)
    Column(
        modifier =
            modifier
                .fillMaxWidth()
                .border(TetherBorder.hairline, TetherTheme.colors.line, shape)
                .padding(TetherSpace.space3),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space1),
    ) {
        if (running.isNotEmpty()) {
            RunningServers(
                running = running,
                expanded = expanded,
                onToggleExpanded = { expanded = !expanded },
                onPreview = onPreview,
                onStop = onStop,
            )
            if (expanded && running.size > 1) {
                running.forEach { process -> ProcessRow(process) }
            }
        }
        preview?.let { target -> PreviewReady(onOpen = { onPreview(target) }) }
    }
}

@Composable
private fun RunningServers(
    running: List<ServerProcess>,
    expanded: Boolean,
    onToggleExpanded: () -> Unit,
    onPreview: (ServerPreviewTarget) -> Unit,
    onStop: (String) -> Unit,
) {
    val first = running.first()
    val preview = running.firstNotNullOfOrNull { it.previewTarget() }
    val title =
        when (val label = ServerState(running).barLabel()) {
            is ServerBarLabel.Port -> stringResource(R.string.server_on_port, label.port.toInt())
            is ServerBarLabel.Count ->
                pluralStringResource(R.plurals.server_running_count, label.count, label.count)
        }
    Column {
        Row(verticalAlignment = Alignment.CenterVertically) {
            TetherIcon(
                icon = TetherIcons.Globe,
                contentDescription = null,
                tint = TetherTheme.colors.text2,
                size = TetherSize.iconS,
            )
            Spacer(Modifier.width(TetherSpace.space2))
            Text(
                text = title,
                style = MaterialTheme.typography.titleSmall,
                color = TetherTheme.colors.text,
                modifier = Modifier.weight(1f),
            )
            if (running.size > 1) {
                TetherIconButton(
                    icon = if (expanded) TetherIcons.ChevronUp else TetherIcons.ChevronDown,
                    contentDescription =
                        stringResource(
                            if (expanded) R.string.server_collapse else R.string.server_expand,
                        ),
                    onClick = onToggleExpanded,
                )
            }
        }
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.End,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            TetherTextButton(
                label = stringResource(R.string.server_preview),
                onClick = { preview?.let(onPreview) },
                enabled = preview != null,
            )
            TetherTextButton(
                label = stringResource(R.string.server_stop),
                onClick = { onStop(first.id) },
                tone = TextButtonTone.Danger,
            )
        }
    }
}

@Composable
private fun ProcessRow(process: ServerProcess) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(start = TetherSpace.space6),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = process.command.ifBlank { process.id },
            style = TetherTheme.monoSmall,
            color = TetherTheme.colors.text2,
            modifier = Modifier.weight(1f),
        )
        process.port?.let { port ->
            Text(
                text = stringResource(R.string.server_port, port.toInt()),
                style = MaterialTheme.typography.bodySmall,
                color = TetherTheme.colors.text3,
            )
        }
    }
}

@Composable
private fun PreviewReady(onOpen: () -> Unit) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(top = TetherSpace.space2),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = stringResource(R.string.server_preview_ready),
            style = MaterialTheme.typography.bodySmall,
            color = TetherTheme.colors.text2,
            modifier = Modifier.weight(1f),
        )
        TetherTextButton(label = stringResource(R.string.server_open), onClick = onOpen)
    }
}

private fun ServerProcess.previewTarget(): ServerPreviewTarget? =
    port?.let { port -> ServerPreviewTarget.Server("http://127.0.0.1:$port", id) }
