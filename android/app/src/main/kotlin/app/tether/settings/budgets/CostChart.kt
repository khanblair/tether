package app.tether.settings.budgets

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.UsageBarChart
import app.tether.design.tokens.TetherSpace

/**
 * Daily spend as bars, oldest day first, with the first and last dates under the plot. The
 * chart reads as one node for TalkBack: the range, the highest day, and its cost. [days] must
 * not be empty.
 */
@Composable
internal fun CostChart(
    days: List<CostDay>,
    modifier: Modifier = Modifier,
) {
    val locale = LocalConfiguration.current.locales[0]
    val peak = days.maxBy { it.costMicros }
    val span = pluralStringResource(R.plurals.budgets_days, days.size, days.size)
    val description =
        stringResource(
            R.string.budgets_chart_description,
            span,
            dayLabel(peak.day, locale),
            dollarsText(peak.costMicros),
        )
    val axis = TetherTheme.chart.axisText
    Column(modifier = modifier.fillMaxWidth()) {
        UsageBarChart(
            values = days.map { it.costMicros },
            labels = emptyList(),
            contentDescription = description,
        )
        Row(modifier = Modifier.fillMaxWidth().padding(top = TetherSpace.space1)) {
            Text(
                text = dayLabel(days.first().day, locale),
                style = MaterialTheme.typography.labelSmall,
                color = axis,
            )
            Spacer(Modifier.weight(1f))
            Text(
                text = dayLabel(days.last().day, locale),
                style = MaterialTheme.typography.labelSmall,
                color = axis,
            )
        }
    }
}
