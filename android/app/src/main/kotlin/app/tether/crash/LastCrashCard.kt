package app.tether.crash

import android.content.Context
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.PrimaryButton
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSpace
import app.tether.messages.AppMessages
import app.tether.settings.github.copyToClipboard

/**
 * Shows the last crash on the About screen. It draws nothing when there is no saved report.
 * Copy puts the whole report on the clipboard and confirms it with a snackbar.
 */
@Composable
fun LastCrashSection(messages: AppMessages) {
    val context = LocalContext.current
    val copied = stringResource(R.string.crash_copied)
    val model: CrashViewModel =
        viewModel(factory = CrashViewModel.factory(crashStore(context)))
    val report by model.report.collectAsStateWithLifecycle()
    val text = report ?: return
    LastCrashCard(
        report = text,
        onCopy = {
            copyToClipboard(context, text)
            messages.show(copied)
        },
        onClear = model::clear,
    )
}

/** The card: a title, the first line of the report, and the copy and clear actions. */
@Composable
fun LastCrashCard(
    report: String,
    onCopy: () -> Unit,
    onClear: () -> Unit,
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
        Text(
            text = stringResource(R.string.crash_card_title),
            style = MaterialTheme.typography.titleSmall,
            color = TetherTheme.colors.text,
        )
        Text(
            text = report.lineSequence().firstOrNull().orEmpty(),
            style = TetherTheme.monoLabel,
            color = TetherTheme.colors.text2,
        )
        Text(
            text = stringResource(R.string.crash_card_body),
            style = MaterialTheme.typography.bodyMedium,
            color = TetherTheme.colors.text2,
        )
        PrimaryButton(
            label = stringResource(R.string.crash_copy_action),
            onClick = onCopy,
            modifier = Modifier.fillMaxWidth(),
        )
        TetherTextButton(
            label = stringResource(R.string.crash_clear_action),
            onClick = onClear,
        )
    }
}

/** The store for the saved report, in the app's files folder. */
internal fun crashStore(context: Context): CrashStore =
    FileCrashStore(context.filesDir.resolve(CRASH_DIR))
