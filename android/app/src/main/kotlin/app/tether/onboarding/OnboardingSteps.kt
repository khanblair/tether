package app.tether.onboarding

import android.view.WindowManager
import androidx.activity.compose.LocalActivity
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalClipboard
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.MainActivity
import app.tether.R
import app.tether.core.generated.ConnectionStatusView
import app.tether.core.generated.ProviderView
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.ListGroup
import app.tether.design.components.ListRow
import app.tether.design.components.PrimaryButton
import app.tether.design.components.SecretField
import app.tether.design.components.TetherIcon
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherSpace
import app.tether.providers.ProviderMessage
import app.tether.providers.connectionMessage
import app.tether.providers.firstText
import app.tether.providers.providerNameRes
import app.tether.settings.ChevronTrailing
import app.tether.workspace.WorkspaceCapabilities
import app.tether.workspace.WorkspaceContent
import app.tether.workspace.WorkspacePermissionEffect
import app.tether.workspace.WorkspaceViewModel
import kotlinx.coroutines.launch

/** The add-key step: pick a provider, paste its key, and save it with a connection test. */
@Composable
internal fun AddKeyStep(
    addKey: AddKeyViewModel,
    onBack: () -> Unit,
    onContinue: () -> Unit,
    onSkip: () -> Unit,
) {
    val state by addKey.state.collectAsStateWithLifecycle()
    OnboardingPage(step = OnboardingStep.AddKey, onBack = onBack) {
        Text(
            text = stringResource(R.string.onboarding_key_intro),
            style = MaterialTheme.typography.bodyLarge,
            color = TetherTheme.colors.text2,
        )
        ProviderChips(
            selected = state.provider,
            enabled = !state.busy,
            onSelect = addKey::selectProvider,
        )
        KeyField(state = state, addKey = addKey)
        if (state.accepted) {
            PrimaryButton(
                label = stringResource(R.string.onboarding_continue),
                onClick = onContinue,
                modifier = Modifier.fillMaxWidth(),
            )
        } else {
            PrimaryButton(
                label = stringResource(R.string.onboarding_save_and_test),
                onClick = addKey::saveAndTest,
                modifier = Modifier.fillMaxWidth(),
                enabled = state.canSave,
                loading = state.busy,
            )
        }
        KeyResult(state)
        SkipButton(onSkip)
    }
}

@Composable
private fun ProviderChips(
    selected: ProviderView,
    enabled: Boolean,
    onSelect: (ProviderView) -> Unit,
) {
    Row(
        modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
        horizontalArrangement = Arrangement.spacedBy(TetherSpace.space2),
    ) {
        ProviderView.entries.forEach { provider ->
            FilterChip(
                selected = provider == selected,
                onClick = { onSelect(provider) },
                enabled = enabled,
                label = { Text(text = stringResource(providerNameRes(provider))) },
                leadingIcon = if (provider == selected) SelectedCheck else null,
            )
        }
    }
}

@Composable
private fun KeyField(
    state: AddKeyUiState,
    addKey: AddKeyViewModel,
) {
    val clipboard = LocalClipboard.current
    val scope = rememberCoroutineScope()
    RevealGuards(revealed = state.revealed, onStopped = addKey::onStopped)
    SecretField(
        value = state.keyInput,
        onValueChange = addKey::onKeyChange,
        label = stringResource(R.string.onboarding_key_label),
        revealed = state.revealed,
        onToggleReveal = if (state.keyInput.isEmpty()) null else addKey::toggleReveal,
        revealLabel = stringResource(R.string.provider_detail_reveal),
        hideLabel = stringResource(R.string.provider_detail_hide),
        pasteLabel = stringResource(R.string.provider_detail_paste),
        hint = stringResource(R.string.onboarding_key_hint),
        error = keyError(state)?.let { stringResource(it) },
        enabled = !state.busy,
        onPaste = { scope.launch { clipboard.firstText()?.let(addKey::onKeyChange) } },
    )
}

/** The check mark on the selected provider chip, so the selection is not shown by color alone. */
private val SelectedCheck: @Composable () -> Unit = {
    TetherIcon(
        icon = TetherIcons.CircleCheck,
        contentDescription = null,
        tint = TetherTheme.colors.accentInk,
    )
}

/**
 * The revealed key must not reach the recents thumbnail or stay visible after the screen stops.
 * Same guards as the provider detail screen.
 */
@Composable
private fun RevealGuards(
    revealed: Boolean,
    onStopped: () -> Unit,
) {
    val activity = LocalActivity.current as? MainActivity
    val lifecycleOwner = LocalLifecycleOwner.current
    DisposableEffect(lifecycleOwner) {
        val observer =
            LifecycleEventObserver { _, event ->
                if (event == Lifecycle.Event.ON_STOP) onStopped()
            }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
    }
    DisposableEffect(activity, revealed) {
        if (revealed) activity?.window?.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        onDispose {
            if (revealed) activity?.window?.clearFlags(WindowManager.LayoutParams.FLAG_SECURE)
        }
    }
}

/** The field message: an empty key, or a key the provider rejected. Null otherwise. */
private fun keyError(state: AddKeyUiState): Int? =
    when {
        state.message == ProviderMessage.EmptyKey -> R.string.provider_detail_error_empty
        state.phase == AddKeyPhase.Tested && state.result == ConnectionStatusView.BAD_KEY ->
            R.string.onboarding_key_rejected
        else -> null
    }

/** The outcome of the last save and test, under the button. Nothing shows before the first try. */
@Composable
private fun KeyResult(state: AddKeyUiState) {
    when {
        state.phase == AddKeyPhase.Testing ->
            InlineBanner(
                text = stringResource(R.string.onboarding_testing_body),
                title = stringResource(R.string.provider_detail_testing_title),
                tone = BannerTone.Info,
            )
        state.accepted ->
            InlineBanner(
                text = stringResource(R.string.onboarding_key_accepted_body),
                title = stringResource(R.string.provider_detail_accepted_title),
                tone = BannerTone.Success,
            )
        state.phase == AddKeyPhase.Tested ->
            connectionMessage(state.result)?.let { message ->
                InlineBanner(
                    text = stringResource(message.body),
                    title = stringResource(message.title),
                    tone = message.tone,
                )
            }
        state.phase == AddKeyPhase.Failed ->
            failureRes(state.message)?.let { res ->
                InlineBanner(text = stringResource(res), tone = BannerTone.Danger)
            }
    }
}

/** The text for a failure that is not a connection result. An empty key shows under the field. */
private fun failureRes(message: ProviderMessage?): Int? =
    when (message) {
        ProviderMessage.SaveFailed -> R.string.provider_detail_message_save
        ProviderMessage.TestFailed -> R.string.provider_detail_message_test
        else -> null
    }

/** The workspace step: keep the private default, or choose a shared folder in the full build. */
@Composable
internal fun WorkspaceStep(
    viewModel: WorkspaceViewModel,
    capabilities: WorkspaceCapabilities,
    onBack: () -> Unit,
    onContinue: () -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    WorkspacePermissionEffect(viewModel, capabilities)
    OnboardingPage(step = OnboardingStep.Workspace, onBack = onBack) {
        WorkspaceContent(
            state = state,
            sharedFolderAllowed = capabilities.sharedFolderAllowed,
            showCurrentFolder = false,
            showDataPaths = true,
            onChoose = viewModel::choose,
            onPathChange = viewModel::onPathChange,
            onSave = { afterSave ->
                viewModel.save {
                    afterSave()
                    onContinue()
                }
            },
            onUsePrivate = { afterSave ->
                viewModel.usePrivateStorage {
                    afterSave()
                    onContinue()
                }
            },
            onAllowAccess = { context -> capabilities.openAllFilesAccessSettings(context) },
        )
        SkipButton { viewModel.usePrivateStorage(onContinue) }
    }
}

/** The open-project step: a new empty project, a GitHub clone, or a zip import. */
@Composable
internal fun OpenProjectStep(
    onBack: () -> Unit,
    onNewProject: () -> Unit,
    onClone: () -> Unit,
    onImport: () -> Unit,
    onSkip: () -> Unit,
) {
    OnboardingPage(step = OnboardingStep.OpenProject, onBack = onBack) {
        Text(
            text = stringResource(R.string.onboarding_project_intro),
            style = MaterialTheme.typography.bodyLarge,
            color = TetherTheme.colors.text2,
        )
        PrimaryButton(
            label = stringResource(R.string.onboarding_new_project),
            onClick = onNewProject,
            modifier = Modifier.fillMaxWidth(),
        )
        ListGroup {
            ListRow(
                title = stringResource(R.string.onboarding_clone_title),
                subtitle = stringResource(R.string.onboarding_clone_sub),
                leading = { SourceIcon(TetherIcons.GitBranch) },
                trailing = { ChevronTrailing() },
                onClick = onClone,
            )
            HorizontalDivider(thickness = TetherBorder.hairline, color = TetherTheme.colors.line)
            ListRow(
                title = stringResource(R.string.onboarding_import_title),
                subtitle = stringResource(R.string.onboarding_import_sub),
                leading = { SourceIcon(TetherIcons.Package) },
                trailing = { ChevronTrailing() },
                onClick = onImport,
            )
        }
        SkipButton(onSkip)
    }
}

@Composable
private fun SourceIcon(icon: ImageVector) {
    TetherIcon(icon = icon, contentDescription = null, tint = TetherTheme.colors.text2)
}
