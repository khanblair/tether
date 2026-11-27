package app.tether.linux

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.TetherIconButton
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherSpace
import app.tether.messages.AppMessages
import app.tether.shell.BackButton

/** The Linux shell route. The ViewModel keeps the session and buffer across rotation. */
@Composable
fun LinuxShellScreen(
    viewModel: LinuxShellViewModel,
    projectName: String?,
    onBack: () -> Unit,
    onOpenExecutors: () -> Unit,
    onClose: () -> Unit,
    modifier: Modifier = Modifier,
    messages: AppMessages? = null,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    val frame by viewModel.frame.collectAsStateWithLifecycle()
    val projectLabel = projectName ?: stringResource(R.string.linux_no_project)
    val copy = rememberCopyTerminal(messages)
    Scaffold(
        modifier = modifier,
        containerColor = TetherTheme.colors.bg,
        topBar = { LinuxShellTopBar(projectLabel, onBack, viewModel::clear, onClose) },
    ) { padding ->
        Column(modifier = Modifier.fillMaxSize().padding(padding)) {
            when (state) {
                LinuxShellState.NotReady -> ShellMessage(R.string.linux_not_ready, onOpenExecutors)
                LinuxShellState.Busy -> ShellMessage(R.string.linux_busy, onClose)
                is LinuxShellState.Failed -> ShellMessage(R.string.linux_failed, onBack)
                else ->
                    ShellTerminal(
                        frame = frame,
                        onInput = viewModel::write,
                        onResize = viewModel::resize,
                        onCopy = copy,
                        modifier = Modifier.weight(1f).fillMaxWidth(),
                    )
            }
        }
    }
}

/** The top bar of the shell route: back, the project label, clear, and close. */
@Composable
private fun LinuxShellTopBar(
    projectLabel: String,
    onBack: () -> Unit,
    onClear: () -> Unit,
    onClose: () -> Unit,
) {
    TetherTopBar(
        title = stringResource(R.string.linux_title),
        subtitle = projectLabel,
        navigationIcon = { BackButton(onBack) },
        actions = {
            TetherIconButton(
                icon = TetherIcons.Trash2,
                contentDescription = stringResource(R.string.linux_clear),
                onClick = onClear,
            )
            TetherIconButton(
                icon = TetherIcons.X,
                contentDescription = stringResource(R.string.linux_close),
                onClick = onClose,
            )
        },
    )
}

/** The Play build's capability screen. It does not create a shell or a ViewModel. */
@Composable
fun LinuxUnavailableScreen(onBack: () -> Unit) {
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = stringResource(R.string.linux_title),
                navigationIcon = { BackButton(onBack) },
            )
        },
    ) { padding ->
        Text(
            text = stringResource(R.string.linux_not_available),
            color = TetherTheme.colors.text2,
            modifier = Modifier.padding(padding).padding(TetherSpace.space4),
        )
    }
}

@Composable
private fun ShellMessage(
    message: Int,
    onAction: () -> Unit,
) {
    Column(
        modifier = Modifier.fillMaxSize().padding(TetherSpace.space4),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
    ) {
        Text(text = stringResource(message), color = TetherTheme.colors.text2)
        TextButton(onClick = onAction, modifier = Modifier.heightIn(min = TetherSpace.space12)) {
            Text(
                text =
                    stringResource(
                        if (message == R.string.linux_not_ready) {
                            R.string.linux_open_executors
                        } else {
                            R.string.linux_close
                        },
                    ),
            )
        }
    }
}
