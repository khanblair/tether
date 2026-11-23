package app.tether.session.tools

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.components.BadgeTone
import app.tether.design.components.StatusBadge

/**
 * The label of the executor a run uses: Sandbox, Linux, or CI. It is hidden when [executor] is
 * null, because the target had no label.
 */
@Composable
fun ExecutorBadge(
    executor: RunExecutor?,
    modifier: Modifier = Modifier,
) {
    if (executor == null) return
    val label =
        when (executor) {
            RunExecutor.Sandbox -> stringResource(R.string.tools_executor_sandbox)
            RunExecutor.Linux -> stringResource(R.string.tools_executor_linux)
            RunExecutor.Ci -> stringResource(R.string.tools_executor_ci)
        }
    StatusBadge(label = label, tone = BadgeTone.Neutral, modifier = modifier)
}
