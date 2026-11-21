package app.tether.home

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.PrimaryButton
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherSpace
import app.tether.shell.BackButton

/**
 * The New empty project screen. The name field checks the text as the user types. Create makes
 * the project and [onCreated] opens its home with this screen removed from the back stack.
 */
@Composable
fun NewProjectScreen(
    state: NewProjectUiState,
    onBack: () -> Unit,
    onNameChange: (String) -> Unit,
    onCreate: () -> Unit,
    onCreated: (String) -> Unit,
) {
    LaunchedEffect(state.projectId) {
        state.projectId?.let(onCreated)
    }
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = stringResource(R.string.route_new_project_title),
                navigationIcon = { BackButton(onBack) },
            )
        },
    ) { padding ->
        Column(
            verticalArrangement = Arrangement.spacedBy(TetherSpace.space4),
            modifier = Modifier.fillMaxSize().padding(padding).padding(TetherSpace.space4),
        ) {
            NameField(state = state, onNameChange = onNameChange)
            if (state.phase == NewProjectPhase.Failed) {
                InlineBanner(
                    text = stringResource(R.string.new_project_failed),
                    tone = BannerTone.Danger,
                    actionLabel = stringResource(R.string.new_project_try_again),
                    onAction = onCreate,
                )
            }
            PrimaryButton(
                label = stringResource(R.string.new_project_create),
                onClick = onCreate,
                enabled = state.canCreate,
                loading = state.phase == NewProjectPhase.Creating,
                modifier = Modifier.fillMaxWidth(),
            )
        }
    }
}

/** The name field. A problem replaces the helper line, in the danger color and as text. */
@Composable
private fun NameField(
    state: NewProjectUiState,
    onNameChange: (String) -> Unit,
) {
    val problem = state.problem
    val supporting =
        when (problem) {
            ProjectNameProblem.TooLong -> stringResource(R.string.new_project_too_long)
            ProjectNameProblem.ControlCharacter -> stringResource(R.string.new_project_control)
            null -> stringResource(R.string.new_project_helper)
        }
    OutlinedTextField(
        value = state.name,
        onValueChange = onNameChange,
        label = { Text(stringResource(R.string.new_project_name_label)) },
        supportingText = {
            Text(
                text = supporting,
                style = MaterialTheme.typography.bodySmall,
                color =
                    if (problem ==
                        null
                    ) {
                        TetherTheme.colors.text2
                    } else {
                        TetherTheme.colors.danger
                    },
            )
        },
        isError = problem != null,
        singleLine = true,
        enabled = state.phase != NewProjectPhase.Creating,
        modifier = Modifier.fillMaxWidth(),
    )
}
