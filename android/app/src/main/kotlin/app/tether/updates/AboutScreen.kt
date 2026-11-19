package app.tether.updates

import android.text.format.Formatter
import androidx.annotation.StringRes
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.core.CoreClient
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.PrimaryButton
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherTextButton
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.messages.AppMessages
import app.tether.shell.BackButton

/**
 * The About screen: the app name, the installed version, and the updates card. Debug builds also
 * show the Rust core card at the bottom when [showCoreCard] is true.
 */
@Composable
fun AboutScreen(
    controller: UpdatesController,
    core: CoreClient,
    messages: AppMessages,
    showCoreCard: Boolean,
    onBack: () -> Unit,
) {
    val state by controller.state.collectAsStateWithLifecycle()
    val lifecycleOwner = LocalLifecycleOwner.current
    LaunchedEffect(Unit) { controller.checkNow() }
    DisposableEffect(lifecycleOwner, controller) {
        val observer =
            LifecycleEventObserver { _, event ->
                if (event == Lifecycle.Event.ON_RESUME) controller.onInstallPermissionResult()
            }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
    }
    val coreCard: @Composable () -> Unit = { CoreCard(core = core, messages = messages) }
    AboutContent(
        state = state,
        onAction = { action -> controller.perform(action) },
        onBack = onBack,
        coreCard = if (showCoreCard) coreCard else null,
    )
}

/**
 * The stateless About screen. [AboutScreen] supplies the state and the actions. [coreCard] is an
 * optional card shown below the updates card.
 */
@Composable
fun AboutContent(
    state: UpdatesUiState,
    onAction: (UpdateAction) -> Unit,
    onBack: () -> Unit,
    coreCard: (@Composable () -> Unit)? = null,
) {
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = stringResource(R.string.about_title),
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
                AppHeader(version = state.currentVersion)
                UpdateCard(state = state, card = updateCardFor(state), onAction = onAction)
                coreCard?.invoke()
            }
        }
    }
}

private fun UpdatesController.perform(action: UpdateAction) =
    when (action) {
        UpdateAction.Check -> checkNow()
        UpdateAction.Download -> download()
        UpdateAction.Cancel -> cancelDownload()
        UpdateAction.Install -> install()
        UpdateAction.OpenSettings -> openInstallSettings()
        UpdateAction.OpenStore -> openStorePage()
    }

@Composable
private fun AppHeader(version: String) {
    Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space1)) {
        Text(
            text = stringResource(R.string.about_app_name),
            style = MaterialTheme.typography.titleLarge,
            color = TetherTheme.colors.text,
        )
        Text(
            text = stringResource(R.string.about_version, version),
            style = TetherTheme.monoLabel,
            color = TetherTheme.colors.text2,
        )
    }
}

@Composable
private fun UpdateCard(
    state: UpdatesUiState,
    card: UpdateCardUi,
    onAction: (UpdateAction) -> Unit,
) {
    val shape = RoundedCornerShape(TetherRadius.md)
    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .clip(shape)
                .background(TetherTheme.colors.surface)
                .border(TetherBorder.hairline, TetherTheme.colors.line, shape)
                .padding(TetherSpace.space4),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
    ) {
        if (card.failed) FailedBanner(card) else TitleRow(state = state, card = card)
        card.body?.let { BodyText(text = bodyText(state, it)) }
        if (card.showSize) SizeText(state)
        if (card.showNotes) state.release?.let { NotesBox(notes = it.notes) }
        if (card.showProgress) ProgressRow(state)
        if (card.showSpinner) SpinnerRow()
        card.primary?.let { action ->
            PrimaryButton(
                label = stringResource(labelFor(action)),
                onClick = { onAction(action) },
                modifier = Modifier.fillMaxWidth(),
            )
        }
        card.secondary?.let { action ->
            TetherTextButton(
                label = stringResource(secondaryLabelFor(action)),
                onClick = { onAction(action) },
            )
        }
    }
}

@Composable
private fun FailedBanner(card: UpdateCardUi) {
    InlineBanner(
        text = stringResource(card.problem ?: card.title),
        title = stringResource(card.title),
        tone = BannerTone.Danger,
    )
}

@Composable
private fun TitleRow(
    state: UpdatesUiState,
    card: UpdateCardUi,
) {
    Row(
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(TetherSpace.space2),
    ) {
        if (card.success) {
            TetherIcon(
                icon = TetherIcons.CircleCheck,
                contentDescription = null,
                tint = TetherTheme.colors.success,
                size = TetherSize.iconS,
            )
        }
        Text(
            text = titleText(state, card.title),
            modifier = Modifier.weight(1f),
            style = MaterialTheme.typography.titleSmall,
            color = TetherTheme.colors.text,
        )
    }
}

@Composable
private fun BodyText(text: String) {
    Text(
        text = text,
        style = MaterialTheme.typography.bodyMedium,
        color = TetherTheme.colors.text2,
    )
}

@Composable
private fun SizeText(state: UpdatesUiState) {
    val release = state.release ?: return
    val size = Formatter.formatShortFileSize(LocalContext.current, release.apkSizeBytes)
    Text(
        text = stringResource(R.string.updates_download_size, size),
        style = MaterialTheme.typography.bodySmall,
        color = TetherTheme.colors.text3,
    )
}

@Composable
private fun NotesBox(notes: String) {
    val shape = RoundedCornerShape(TetherRadius.sm)
    Text(
        text = notes,
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(max = TetherSize.notesMax)
                .clip(shape)
                .background(TetherTheme.colors.surfaceSunken)
                .border(TetherBorder.hairline, TetherTheme.colors.line, shape)
                .verticalScroll(rememberScrollState())
                .padding(TetherSpace.space3),
        style = MaterialTheme.typography.bodySmall,
        color = TetherTheme.colors.text2,
    )
}

@Composable
private fun ProgressRow(state: UpdatesUiState) {
    val percent = state.percent
    val barColor = TetherTheme.colors.text2
    val trackColor = TetherTheme.colors.surfaceSunken
    Row(
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(TetherSpace.space3),
    ) {
        if (percent == null) {
            LinearProgressIndicator(
                modifier = Modifier.weight(1f),
                color = barColor,
                trackColor = trackColor,
            )
        } else {
            LinearProgressIndicator(
                progress = { percent / PERCENT_MAX },
                modifier = Modifier.weight(1f),
                color = barColor,
                trackColor = trackColor,
            )
            Text(
                text = stringResource(R.string.updates_progress_percent, percent),
                style = TetherTheme.monoLabel,
                color = TetherTheme.colors.text2,
            )
        }
    }
}

@Composable
private fun SpinnerRow() {
    CircularProgressIndicator(
        modifier = Modifier.size(TetherSize.iconS),
        color = TetherTheme.colors.text2,
        strokeWidth = TetherBorder.inputFocus,
    )
}

@Composable
private fun titleText(
    state: UpdatesUiState,
    @StringRes title: Int,
): String =
    if (state.phase in VERSIONED_TITLES) {
        stringResource(title, state.newerVersion.orEmpty())
    } else {
        stringResource(title)
    }

@Composable
private fun bodyText(
    state: UpdatesUiState,
    @StringRes body: Int,
): String =
    if (state.phase == UpdatePhase.UpToDate) {
        stringResource(body, state.currentVersion)
    } else {
        stringResource(body)
    }

@StringRes
private fun labelFor(action: UpdateAction): Int =
    when (action) {
        UpdateAction.Check -> R.string.updates_check_action
        UpdateAction.Download -> R.string.updates_download_action
        UpdateAction.Cancel -> R.string.updates_cancel_action
        UpdateAction.Install -> R.string.updates_install_action
        UpdateAction.OpenSettings -> R.string.updates_settings_action
        UpdateAction.OpenStore -> R.string.updates_store_action
    }

@StringRes
private fun secondaryLabelFor(action: UpdateAction): Int =
    if (action == UpdateAction.Check) R.string.updates_check_again else labelFor(action)

private const val PERCENT_MAX = 100f
private val VERSIONED_TITLES =
    setOf(
        UpdatePhase.Available,
        UpdatePhase.Downloading,
        UpdatePhase.Ready,
        UpdatePhase.Installing,
        UpdatePhase.Installed,
    )
