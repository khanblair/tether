package app.tether.session.loads

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.TetherIcon
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.home.relativeTime
import app.tether.home.text

/**
 * What a session in this project loads: the instructions file, memory entries, and skills, with
 * the last session of the project. It shows in the empty transcript of a new session.
 */
@Composable
fun SessionLoadsCard(
    loads: SessionLoads,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier = modifier.padding(TetherSpace.space4),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space4),
    ) {
        LoadedCard(loads)
        loads.lastSessionTitle?.let { title ->
            LastSessionLine(title = title, updatedAtMs = loads.lastSessionUpdatedAtMs)
        }
    }
}

@Composable
internal fun LoadedCard(loads: SessionLoads) {
    Surface(
        color = TetherTheme.colors.surfaceSunken,
        shape = RoundedCornerShape(TetherRadius.md),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(
            modifier = Modifier.padding(TetherSpace.space4),
            verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
        ) {
            Text(
                text = stringResource(R.string.session_loads_loaded_title),
                style = MaterialTheme.typography.titleSmall,
                color = TetherTheme.colors.text,
            )
            ContextRow(
                icon = TetherIcons.FileText,
                title =
                    loads.instructionsFile
                        ?: stringResource(R.string.session_loads_no_instructions),
                detail = instructionsLine(loads),
            )
            ContextRow(
                icon = TetherIcons.Brain,
                title = stringResource(R.string.session_loads_memory_title),
                detail = memoryLine(loads.memoryEntries),
            )
            ContextRow(
                icon = TetherIcons.Package,
                title = stringResource(R.string.session_loads_skills_title),
                detail = skillsLine(loads.skillsAvailable),
            )
        }
    }
}

@Composable
private fun ContextRow(
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    title: String,
    detail: String,
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(TetherSpace.space3),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        TetherIcon(
            icon = icon,
            contentDescription = null,
            tint = TetherTheme.colors.text2,
            size = TetherSize.iconS,
        )
        Column(modifier = Modifier.weight(1f)) {
            Text(text = title, style = MaterialTheme.typography.bodyLarge)
            Text(
                text = detail,
                style = MaterialTheme.typography.bodySmall,
                color = TetherTheme.colors.text3,
            )
        }
    }
}

@Composable
private fun instructionsLine(loads: SessionLoads): String =
    if (instructionsDetail(loads.instructionsFile)) {
        pluralStringResource(
            R.plurals.session_loads_instruction_lines,
            loads.instructionsLines,
            loads.instructionsLines,
        )
    } else {
        stringResource(R.string.session_loads_instructions_helper)
    }

@Composable
private fun memoryLine(count: Int): String =
    when (detailCount(count)) {
        DetailCount.None -> stringResource(R.string.session_loads_memory_none)
        DetailCount.One,
        DetailCount.Many,
        -> pluralStringResource(R.plurals.session_loads_memory_entries, count, count)
    }

@Composable
private fun skillsLine(count: Int): String =
    when (detailCount(count)) {
        DetailCount.None -> stringResource(R.string.session_loads_skills_none)
        DetailCount.One,
        DetailCount.Many,
        -> pluralStringResource(R.plurals.session_loads_skills_available, count, count)
    }

@Composable
internal fun LastSessionLine(
    title: String,
    updatedAtMs: Long?,
) {
    val time = updatedAtMs ?: return
    val now = remember { System.currentTimeMillis() }
    Text(
        text =
            stringResource(
                R.string.session_loads_last_session,
                title,
                relativeTime(now, time).text(),
            ),
        style = MaterialTheme.typography.bodySmall,
        color = TetherTheme.colors.text3,
        modifier = Modifier.fillMaxWidth(),
    )
}
