package app.tether.session

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.SectionTitle
import app.tether.design.components.SegmentedControl
import app.tether.design.tokens.TetherSpace

/** The choices in the order the control shows them: the model default, then the three levels. */
private val EFFORT_CHOICES: List<ThinkingEffort?> =
    listOf(null, ThinkingEffort.LOW, ThinkingEffort.MEDIUM, ThinkingEffort.HIGH)

/** The name of a thinking effort, or of the model default for null. */
fun ThinkingEffort?.labelRes(): Int =
    when (this) {
        null -> R.string.session_effort_default
        ThinkingEffort.LOW -> R.string.session_effort_low
        ThinkingEffort.MEDIUM -> R.string.session_effort_medium
        ThinkingEffort.HIGH -> R.string.session_effort_high
    }

/**
 * The thinking section of the model sheet: a helper line and a segmented control with the model
 * default and the three levels. [effort] is the selected choice; null selects the model default.
 */
@Composable
fun EffortSection(
    effort: ThinkingEffort?,
    enabled: Boolean,
    onChoose: (ThinkingEffort?) -> Unit,
    modifier: Modifier = Modifier,
) {
    val labels = EFFORT_CHOICES.associateWith { stringResource(it.labelRes()) }
    Column(modifier = modifier.fillMaxWidth()) {
        SectionTitle(text = stringResource(R.string.session_effort_title))
        Text(
            text = stringResource(R.string.session_effort_helper),
            style = MaterialTheme.typography.bodySmall,
            color = TetherTheme.colors.text2,
            modifier =
                Modifier.padding(
                    horizontal = TetherSpace.space4,
                    vertical = TetherSpace.space2,
                ),
        )
        SegmentedControl(
            options = EFFORT_CHOICES,
            selected = effort,
            onSelect = onChoose,
            label = { labels.getValue(it) },
            enabled = enabled,
            modifier = Modifier.padding(horizontal = TetherSpace.space4),
        )
    }
}
