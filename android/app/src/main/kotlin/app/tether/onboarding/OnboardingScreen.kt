package app.tether.onboarding

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.PrimaryButton
import app.tether.design.components.TetherTextButton
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherSpace
import app.tether.shell.BackButton
import app.tether.workspace.WorkspaceCapabilities
import app.tether.workspace.WorkspaceViewModel

/**
 * The first-run flow: welcome, a provider key, a workspace, then a project. Back moves one step at
 * a time. Skip ends the flow except on the workspace step, where it keeps private storage.
 */
@Composable
fun OnboardingScreen(
    onboarding: OnboardingViewModel,
    addKey: AddKeyViewModel,
    workspace: WorkspaceViewModel,
    workspaceCapabilities: WorkspaceCapabilities,
    onExit: (OnboardingExit) -> Unit,
) {
    val state by onboarding.state.collectAsStateWithLifecycle()
    BackHandler(enabled = state.step != OnboardingStep.Welcome, onBack = onboarding::back)
    LaunchedEffect(state.exit) { state.exit?.let(onExit) }
    when (state.step) {
        OnboardingStep.Welcome ->
            WelcomeStep(
                name = state.name,
                onNameChange = onboarding::onNameChange,
                onContinue = onboarding::next,
                onSkip = onboarding::skip,
            )
        OnboardingStep.AddKey ->
            AddKeyStep(
                addKey = addKey,
                onBack = onboarding::back,
                onContinue = onboarding::next,
                onSkip = onboarding::skip,
            )
        OnboardingStep.Workspace ->
            WorkspaceStep(
                viewModel = workspace,
                capabilities = workspaceCapabilities,
                onBack = onboarding::back,
                onContinue = onboarding::next,
            )
        OnboardingStep.OpenProject ->
            OpenProjectStep(
                onBack = onboarding::back,
                onNewProject = onboarding::openNewProject,
                onClone = onboarding::openClone,
                onImport = onboarding::openImport,
                onSkip = onboarding::skip,
            )
    }
}

@Composable
private fun WelcomeStep(
    name: String,
    onNameChange: (String) -> Unit,
    onContinue: () -> Unit,
    onSkip: () -> Unit,
) {
    OnboardingPage(step = OnboardingStep.Welcome, onBack = null) {
        Text(
            text = stringResource(R.string.onboarding_welcome_headline),
            style = MaterialTheme.typography.headlineMedium,
            color = TetherTheme.colors.text,
        )
        Text(
            text = stringResource(R.string.onboarding_welcome_body),
            style = MaterialTheme.typography.bodyLarge,
            color = TetherTheme.colors.text2,
        )
        OutlinedTextField(
            value = name,
            onValueChange = onNameChange,
            modifier = Modifier.fillMaxWidth(),
            singleLine = true,
            label = { Text(text = stringResource(R.string.onboarding_name_label)) },
            supportingText = {
                Text(
                    text = stringResource(R.string.onboarding_name_note),
                    style = MaterialTheme.typography.bodySmall,
                    color = TetherTheme.colors.text3,
                )
            },
        )
        PrimaryButton(
            label = stringResource(R.string.onboarding_continue),
            onClick = onContinue,
            modifier = Modifier.fillMaxWidth(),
        )
        SkipButton(onSkip)
    }
}

/** The Skip text button that every onboarding step ends with. */
@Composable
internal fun SkipButton(onSkip: () -> Unit) {
    TetherTextButton(label = stringResource(R.string.onboarding_skip), onClick = onSkip)
}

/**
 * The layout every onboarding step shares: a top bar with a back arrow when [onBack] is set, a
 * step count, then one column of content.
 */
@Composable
internal fun OnboardingPage(
    step: OnboardingStep,
    onBack: (() -> Unit)?,
    content: @Composable () -> Unit,
) {
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = stringResource(titleRes(step)),
                navigationIcon = { if (onBack != null) BackButton(onBack) },
            )
        },
    ) { padding ->
        Column(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(padding)
                    .verticalScroll(rememberScrollState())
                    .padding(TetherSpace.space4),
            verticalArrangement = Arrangement.spacedBy(TetherSpace.space4),
        ) {
            Text(
                text =
                    stringResource(
                        R.string.onboarding_step,
                        step.ordinal + 1,
                        OnboardingStep.entries.size,
                    ),
                style = MaterialTheme.typography.labelMedium,
                color = TetherTheme.colors.text3,
            )
            content()
        }
    }
}

private fun titleRes(step: OnboardingStep): Int =
    when (step) {
        OnboardingStep.Welcome -> R.string.route_onboarding_title
        OnboardingStep.AddKey -> R.string.onboarding_key_title
        OnboardingStep.Workspace -> R.string.workspace_onboarding_title
        OnboardingStep.OpenProject -> R.string.onboarding_project_title
    }
