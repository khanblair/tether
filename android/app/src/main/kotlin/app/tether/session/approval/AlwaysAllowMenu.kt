package app.tether.session.approval

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.withStyle
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.PrimaryButton
import app.tether.design.components.TetherSheet
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherSpace
import app.tether.session.agent.AgentLoopFailure

/**
 * The Always allow menu. It shows the exact rule that the answer creates, in words and in mono,
 * and asks for Create rule and allow or Cancel.
 */
@Composable
fun AlwaysAllowMenu(
    rule: AlwaysAllowRule,
    busy: Boolean,
    failure: AgentLoopFailure?,
    onConfirm: () -> Unit,
    onCancel: () -> Unit,
) {
    TetherSheet(title = stringResource(R.string.approval_always_title), onDismiss = onCancel) {
        AlwaysAllowMenuContent(
            rule = rule,
            busy = busy,
            failure = failure,
            onConfirm = onConfirm,
            onCancel = onCancel,
        )
    }
}

/** The body of [AlwaysAllowMenu], without the sheet, so it can be previewed. */
@Composable
internal fun AlwaysAllowMenuContent(
    rule: AlwaysAllowRule,
    busy: Boolean,
    failure: AgentLoopFailure?,
    onConfirm: () -> Unit,
    onCancel: () -> Unit,
) {
    Column(
        modifier = Modifier.padding(horizontal = TetherSpace.space4),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
    ) {
        Text(
            text = ruleSentence(rule),
            style = MaterialTheme.typography.bodyLarge,
            color = TetherTheme.colors.text,
        )
        if (failure != null) {
            InlineBanner(
                text = stringResource(failure.messageRes()),
                tone = BannerTone.Danger,
            )
        }
        PrimaryButton(
            label = stringResource(R.string.approval_always_confirm),
            onClick = onConfirm,
            modifier = Modifier.fillMaxWidth(),
            enabled = !busy,
            loading = busy,
        )
        TetherTextButton(
            label = stringResource(R.string.approval_always_cancel),
            onClick = onCancel,
            modifier = Modifier.fillMaxWidth(),
            enabled = !busy,
        )
        Text(
            text = stringResource(R.string.approval_always_footer),
            style = MaterialTheme.typography.bodySmall,
            color = TetherTheme.colors.text3,
        )
    }
}

/**
 * The rule as one sentence. The rule value is set in mono, found by its position in the
 * translated sentence, so the sentence can move the value anywhere.
 */
@Composable
private fun ruleSentence(rule: AlwaysAllowRule) =
    run {
        val template =
            stringResource(
                if (rule.kind == AlwaysAllowKind.Command) {
                    R.string.approval_always_command
                } else {
                    R.string.approval_always_edit
                },
                rule.value,
            )
        val start = template.indexOf(rule.value)
        buildAnnotatedString {
            if (start < 0) {
                append(template)
            } else {
                append(template.substring(0, start))
                withStyle(SpanStyle(fontFamily = TetherTheme.mono.fontFamily)) {
                    append(rule.value)
                }
                append(template.substring(start + rule.value.length))
            }
        }
    }
