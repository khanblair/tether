package app.tether.providers

import android.view.WindowManager
import androidx.activity.compose.LocalActivity
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalClipboard
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.MainActivity
import app.tether.R
import app.tether.core.generated.ConnectionStatusView
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.PrimaryButton
import app.tether.design.components.SecretField
import app.tether.design.components.TestConnectionButton
import app.tether.design.components.TestState
import app.tether.design.components.TetherTextButton
import app.tether.design.components.TetherTopBar
import app.tether.design.components.TextButtonTone
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.shell.BackButton
import kotlinx.coroutines.launch

/** One provider's key, its connection test, and its status. */
@Composable
fun ProviderDetailScreen(
    viewModel: ProviderDetailViewModel,
    onBack: () -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()
    val clipboard = LocalClipboard.current
    val activity = LocalActivity.current as? MainActivity
    val lifecycleOwner = LocalLifecycleOwner.current
    val revealed = state.revealed

    DisposableEffect(lifecycleOwner, viewModel) {
        val observer =
            LifecycleEventObserver { _, event ->
                if (event == Lifecycle.Event.ON_STOP) viewModel.onStopped()
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

    ProviderDetailContent(
        state = state,
        onKeyChange = viewModel::onKeyChange,
        onPaste = {
            scope.launch {
                val clip = clipboard.getClipEntry()?.clipData
                val text =
                    clip
                        ?.takeIf { it.itemCount > 0 }
                        ?.getItemAt(0)
                        ?.text
                        ?.toString()
                if (!text.isNullOrEmpty()) viewModel.onKeyChange(text)
            }
        },
        onToggleReveal = {
            when (state.revealAction) {
                RevealAction.Hide -> viewModel.hide()
                RevealAction.Show -> viewModel.show()
                RevealAction.Authenticate ->
                    activity?.authenticateForKeyReveal(
                        onSuccess = viewModel::reveal,
                        onError = {},
                    )
                RevealAction.None -> Unit
            }
        },
        onSave = viewModel::save,
        onTest = viewModel::test,
        onRemove = viewModel::remove,
        onDismissMessage = viewModel::dismissMessage,
        onBack = onBack,
    )
}

/** The stateless detail screen. [ProviderDetailScreen] supplies the state and the actions. */
@Suppress("LongParameterList")
@Composable
fun ProviderDetailContent(
    state: ProviderDetailUiState,
    onKeyChange: (String) -> Unit,
    onPaste: () -> Unit,
    onToggleReveal: () -> Unit,
    onSave: () -> Unit,
    onTest: () -> Unit,
    onRemove: () -> Unit,
    onDismissMessage: () -> Unit,
    onBack: () -> Unit,
) {
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = stringResource(providerNameRes(state.provider)),
                navigationIcon = { BackButton(onBack) },
            )
        },
    ) { padding ->
        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .verticalScroll(rememberScrollState())
                    .padding(TetherSpace.space4),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Column(
                modifier = Modifier.widthIn(max = TetherSize.transcriptMax).fillMaxWidth(),
                verticalArrangement = Arrangement.spacedBy(TetherSpace.space4),
            ) {
                KeySection(state, onKeyChange, onPaste, onToggleReveal, onSave)
                MessageBanner(state.message, onDismissMessage)
                ResultBanner(state)
                Text(
                    text = stringResource(R.string.provider_detail_test_target, state.testTarget),
                    style = TetherTheme.mono,
                    color = TetherTheme.colors.text3,
                )
                TestSection(state, onTest)
                RemoveKeyAction(state, onRemove)
            }
        }
    }
}

@Composable
private fun KeySection(
    state: ProviderDetailUiState,
    onKeyChange: (String) -> Unit,
    onPaste: () -> Unit,
    onToggleReveal: () -> Unit,
    onSave: () -> Unit,
) {
    SecretField(
        value = state.keyInput,
        onValueChange = onKeyChange,
        label = stringResource(R.string.provider_detail_key_label),
        revealed = state.revealed,
        onToggleReveal = if (state.revealAction == RevealAction.None) null else onToggleReveal,
        revealLabel = stringResource(R.string.provider_detail_reveal),
        hideLabel = stringResource(R.string.provider_detail_hide),
        pasteLabel = stringResource(R.string.provider_detail_paste),
        placeholder = stringResource(placeholderRes(state.hasKey)),
        hint =
            if (state.hasKey) {
                stringResource(R.string.provider_detail_key_hint)
            } else {
                null
            },
        error =
            if (state.message == ProviderMessage.EmptyKey) {
                stringResource(R.string.provider_detail_error_empty)
            } else {
                null
            },
        enabled = !state.saving,
        onPaste = onPaste,
    )
    PrimaryButton(
        label = stringResource(R.string.provider_detail_save),
        onClick = onSave,
        modifier = Modifier.fillMaxWidth(),
        enabled = state.canSave,
        loading = state.saving,
    )
}

@Composable
private fun MessageBanner(
    message: ProviderMessage?,
    onDismiss: () -> Unit,
) {
    if (message == null || message == ProviderMessage.EmptyKey) return
    InlineBanner(
        text = stringResource(messageRes(message)),
        tone = BannerTone.Danger,
        actionLabel = stringResource(R.string.provider_detail_dismiss),
        onAction = onDismiss,
    )
}

private fun placeholderRes(hasKey: Boolean): Int =
    if (hasKey) R.string.provider_detail_key_saved else R.string.provider_detail_key_empty

private fun messageRes(message: ProviderMessage): Int =
    when (message) {
        ProviderMessage.LoadFailed -> R.string.provider_detail_message_load
        ProviderMessage.EmptyKey -> R.string.provider_detail_error_empty
        ProviderMessage.SaveFailed -> R.string.provider_detail_message_save
        ProviderMessage.RemoveFailed -> R.string.provider_detail_message_remove
        ProviderMessage.RevealFailed -> R.string.provider_detail_message_reveal
        ProviderMessage.TestFailed -> R.string.provider_detail_message_test
    }

@Composable
private fun ResultBanner(state: ProviderDetailUiState) {
    when {
        state.testing ->
            InlineBanner(
                text = state.testTarget,
                title = stringResource(R.string.provider_detail_testing_title),
                tone = BannerTone.Info,
            )
        state.hasKey && state.status == ConnectionStatusView.OK ->
            InlineBanner(
                text =
                    ProviderText
                        .Plural(
                            R.plurals.provider_detail_models_available,
                            state.modelCount,
                            listOf(state.modelCount),
                        ).resolve(),
                title = stringResource(R.string.provider_detail_accepted_title),
                tone = BannerTone.Success,
            )
        state.status == ConnectionStatusView.BAD_KEY ->
            InlineBanner(
                text = stringResource(R.string.provider_detail_rejected_body),
                title = stringResource(R.string.provider_detail_rejected_title),
                tone = BannerTone.Danger,
            )
        state.status == ConnectionStatusView.NETWORK_ERROR ->
            InlineBanner(
                text = stringResource(R.string.provider_detail_offline_body),
                title = stringResource(R.string.provider_detail_offline_title),
                tone = BannerTone.Warning,
            )
        state.status == ConnectionStatusView.BLOCKED ->
            InlineBanner(
                text = stringResource(R.string.provider_detail_blocked_body),
                title = stringResource(R.string.provider_detail_blocked_title),
                tone = BannerTone.Danger,
            )
    }
}

@Composable
private fun TestSection(
    state: ProviderDetailUiState,
    onTest: () -> Unit,
) {
    val testState =
        when {
            state.testing -> TestState.Testing
            state.status == ConnectionStatusView.OK -> TestState.Ok
            state.status == ConnectionStatusView.BAD_KEY ||
                state.status == ConnectionStatusView.NETWORK_ERROR ||
                state.status == ConnectionStatusView.BLOCKED -> TestState.Failed
            else -> TestState.Idle
        }
    val label =
        when (testState) {
            TestState.Testing -> stringResource(R.string.provider_detail_testing)
            TestState.Ok, TestState.Failed -> stringResource(R.string.provider_detail_test_again)
            TestState.Idle -> stringResource(R.string.provider_detail_test)
        }
    TestConnectionButton(
        state = testState,
        label = label,
        onTest = onTest,
        enabled = state.canTest,
    )
}

@Composable
private fun RemoveKeyAction(
    state: ProviderDetailUiState,
    onRemove: () -> Unit,
) {
    var confirming by rememberSaveable { mutableStateOf(false) }
    if (!state.hasKey) return
    TetherTextButton(
        label = stringResource(R.string.provider_detail_remove),
        onClick = { confirming = true },
        tone = TextButtonTone.Danger,
        icon = TetherIcons.Trash2,
        enabled = !state.saving && !state.testing,
    )
    if (confirming) {
        RemoveKeyDialog(
            providerName = stringResource(providerNameRes(state.provider)),
            onConfirm = {
                confirming = false
                onRemove()
            },
            onDismiss = { confirming = false },
        )
    }
}

@Composable
private fun RemoveKeyDialog(
    providerName: String,
    onConfirm: () -> Unit,
    onDismiss: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = {
            Text(
                text = stringResource(R.string.provider_detail_remove_title, providerName),
            )
        },
        text = { Text(text = stringResource(R.string.provider_detail_remove_body)) },
        confirmButton = {
            TetherTextButton(
                label = stringResource(R.string.provider_detail_remove),
                onClick = onConfirm,
                tone = TextButtonTone.Danger,
            )
        },
        dismissButton = {
            TetherTextButton(
                label = stringResource(R.string.settings_cancel),
                onClick = onDismiss,
            )
        },
    )
}
