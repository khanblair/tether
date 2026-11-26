package app.tether.home

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.ListRow
import app.tether.design.components.SectionTitle
import app.tether.design.components.StatusMark
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.session.sessionTitleText

/**
 * The archived sessions, at the foot of the drawer list. Nothing shows when none is archived. The
 * caller hides the block while the search has text. The toggle line opens the list. An archived
 * row does not open anything, because an archived session cannot be opened until it is restored.
 */
fun LazyListScope.archivedItems(
    archived: List<SessionSummary>,
    showArchived: Boolean,
    now: Long,
    onToggle: () -> Unit,
    onUnarchive: (SessionSummary) -> Unit,
) {
    if (archived.isEmpty()) return
    item(key = "archived-toggle") {
        ArchivedToggle(count = archived.size, expanded = showArchived, onClick = onToggle)
    }
    if (showArchived) {
        item(key = "archived-title") {
            SectionTitle(text = stringResource(R.string.drawer_archived_title))
        }
        items(archived, key = { "archived-${it.id}" }) { session ->
            ListRow(
                title = sessionTitleText(session.title),
                subtitle =
                    stringResource(
                        R.string.drawer_archived_subtitle,
                        session.projectName,
                        relativeTime(now, session.updatedAt).text(),
                    ),
                leading = { StatusMark(state = session.state) },
                trailing = {
                    TetherTextButton(
                        label = stringResource(R.string.drawer_archived_unarchive),
                        onClick = { onUnarchive(session) },
                    )
                },
            )
        }
    }
}

/** The quiet line that shows or hides the archived sessions. The whole line is the button. */
@Composable
private fun ArchivedToggle(
    count: Int,
    expanded: Boolean,
    onClick: () -> Unit,
) {
    val label =
        if (expanded) {
            stringResource(R.string.drawer_archived_hide)
        } else {
            pluralStringResource(R.plurals.drawer_archived_show, count, count)
        }
    Box(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = TetherSize.touchMin)
                .clickable(role = Role.Button, onClick = onClick)
                .padding(horizontal = TetherSpace.space4),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.bodySmall,
            color = TetherTheme.colors.text3,
        )
    }
}
