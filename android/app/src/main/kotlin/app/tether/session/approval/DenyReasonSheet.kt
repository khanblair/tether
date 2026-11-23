package app.tether.session.approval

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.PrimaryButton
import app.tether.design.components.TetherSheet
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSpace
import app.tether.session.agent.AgentLoopFailure

/** The reason length from which the characters-left counter shows. */
private const val COUNTER_FROM = 160

/**
 * The deny sheet. The reason is optional and one line. A blank reason denies without one.
 * [onReasonChange] receives the whole field text, already cut to [MAX_DENY_REASON_CHARS].
 */
@Composable
fun DenyReasonSheet(
    reason: String,
    busy: Boolean,
    failure: AgentLoopFailure?,
    onReasonChange: (String) -> Unit,
    onSubmit: () -> Unit,
    onCancel: () -> Unit,
) {
    TetherSheet(title = stringResource(R.string.approval_deny_title), onDismiss = onCancel) {
        DenyReasonContent(
            reason = reason,
            busy = busy,
            failure = failure,
            onReasonChange = onReasonChange,
            onSubmit = onSubmit,
            onCancel = onCancel,
        )
    }
}

/** The body of [DenyReasonSheet], without the sheet, so it can be previewed. */
@Composable
internal fun DenyReasonContent(
    reason: String,
    busy: Boolean,
    failure: AgentLoopFailure?,
    onReasonChange: (String) -> Unit,
    onSubmit: () -> Unit,
    onCancel: () -> Unit,
) {
    val focus = remember { FocusRequester() }
    LaunchedEffect(Unit) { focus.requestFocus() }
    Column(
        modifier = Modifier.padding(horizontal = TetherSpace.space4),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
    ) {
        OutlinedTextField(
            value = reason,
            onValueChange = onReasonChange,
            modifier = Modifier.fillMaxWidth().focusRequester(focus),
            enabled = !busy,
            singleLine = true,
            placeholder = {
                Text(
                    text = stringResource(R.string.approval_deny_hint),
                    style = MaterialTheme.typography.bodyMedium,
                    color = TetherTheme.colors.text3,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            },
            shape = RoundedCornerShape(TetherRadius.sm),
            colors =
                OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = TetherTheme.colors.accentInk,
                    unfocusedBorderColor = TetherTheme.colors.lineInput,
                ),
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
            keyboardActions = KeyboardActions(onDone = { if (!busy) onSubmit() }),
        )
        if (reason.length >= COUNTER_FROM) {
            val left = MAX_DENY_REASON_CHARS - reason.length
            Text(
                text = pluralStringResource(R.plurals.approval_reason_left, left, left),
                style = MaterialTheme.typography.labelSmall,
                color = TetherTheme.colors.text3,
            )
        }
        if (failure != null) {
            InlineBanner(
                text = stringResource(failure.messageRes()),
                tone = BannerTone.Danger,
            )
        }
        PrimaryButton(
            label = stringResource(R.string.approval_deny_submit),
            onClick = onSubmit,
            modifier = Modifier.fillMaxWidth(),
            enabled = !busy,
            loading = busy,
        )
        TetherTextButton(
            label = stringResource(R.string.approval_deny_cancel),
            onClick = onCancel,
            modifier = Modifier.fillMaxWidth(),
            enabled = !busy,
        )
    }
}
