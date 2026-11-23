package app.tether.session.cards

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.PrimaryButton
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.session.agent.AgentLoopFailure
import app.tether.session.agent.QuestionUi

/**
 * The agent's question with its options, as full-width rows. A text field appears when the
 * question allows free text. After the answer the card shows the answer and takes no input. The
 * accent is on the Send button only; the option rows are neutral.
 */
@Composable
fun AskUserCard(
    question: QuestionUi,
    answer: String?,
    draft: String,
    busy: Boolean,
    failure: AgentLoopFailure?,
    onDraftChange: (String) -> Unit,
    onOption: (String) -> Unit,
    onSend: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val colors = TetherTheme.colors
    val shape = RoundedCornerShape(TetherRadius.md)
    Column(
        modifier =
            modifier
                .fillMaxWidth()
                .background(colors.surface, shape)
                .border(TetherBorder.hairline, colors.line, shape)
                .padding(TetherSpace.space4),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
    ) {
        Text(
            text = stringResource(R.string.cards_ask_title),
            style = MaterialTheme.typography.bodySmall,
            color = colors.text2,
        )
        Text(
            text = question.question,
            style = MaterialTheme.typography.bodyLarge,
            color = colors.text,
        )
        if (answer != null) {
            AnsweredBlock(answer)
        } else {
            question.options.forEach { option ->
                AnswerOption(text = option, enabled = !busy, onClick = { onOption(option) })
            }
            if (question.allowFreeText) {
                FreeTextAnswer(draft, busy, onDraftChange, onSend)
            }
        }
        if (failure != null) {
            InlineBanner(text = stringResource(failure.messageRes()), tone = BannerTone.Danger)
        }
    }
}

/** The answer the user gave, shown in place of the options. */
@Composable
private fun AnsweredBlock(answer: String) {
    val colors = TetherTheme.colors
    Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space1)) {
        Text(
            text = stringResource(R.string.cards_ask_answered),
            style = MaterialTheme.typography.bodySmall,
            color = colors.text3,
        )
        Text(
            text = answer,
            style = MaterialTheme.typography.bodyLarge,
            color = colors.text,
            modifier =
                Modifier
                    .fillMaxWidth()
                    .background(colors.surfaceSunken, RoundedCornerShape(TetherRadius.sm))
                    .padding(TetherSpace.space3),
        )
    }
}

/** One option: a full-width row with a soft border. It is neutral, with no accent. */
@Composable
private fun AnswerOption(
    text: String,
    enabled: Boolean,
    onClick: () -> Unit,
) {
    val colors = TetherTheme.colors
    val shape = RoundedCornerShape(TetherRadius.sm)
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = TetherSize.touchMin)
                .border(TetherBorder.input, colors.lineInput, shape)
                .clip(shape)
                .clickable(enabled = enabled, role = Role.Button, onClick = onClick)
                .padding(horizontal = TetherSpace.space4, vertical = TetherSpace.space3),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = text,
            style = MaterialTheme.typography.bodyLarge,
            color = if (enabled) colors.text else colors.textDisabled,
        )
    }
}

/** The free-text answer: a field with a counter near the limit, and the Send button. */
@Composable
private fun FreeTextAnswer(
    draft: String,
    busy: Boolean,
    onDraftChange: (String) -> Unit,
    onSend: () -> Unit,
) {
    val counter: (@Composable () -> Unit)? =
        if (draft.length >= ANSWER_COUNTER_FROM) {
            {
                Text(
                    text =
                        stringResource(
                            R.string.cards_ask_counter,
                            draft.length,
                            ANSWER_MAX_CHARS,
                        ),
                )
            }
        } else {
            null
        }
    Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space2)) {
        OutlinedTextField(
            value = draft,
            onValueChange = onDraftChange,
            modifier = Modifier.fillMaxWidth(),
            enabled = !busy,
            placeholder = { Text(text = stringResource(R.string.cards_ask_hint)) },
            supportingText = counter,
            minLines = 1,
            maxLines = 4,
            shape = RoundedCornerShape(TetherRadius.sm),
        )
        PrimaryButton(
            label = stringResource(R.string.cards_ask_send),
            onClick = onSend,
            modifier = Modifier.fillMaxWidth(),
            enabled = !busy && answerText(draft) != null,
            loading = busy,
        )
    }
}
