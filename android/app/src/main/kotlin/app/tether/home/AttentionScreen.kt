package app.tether.home

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.ListGroup
import app.tether.design.components.ListRow
import app.tether.design.components.SectionTitle
import app.tether.design.components.SessionState
import app.tether.design.components.StatusMark
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherSpace
import app.tether.session.sessionTitleText
import app.tether.shell.BackButton

/**
 * Needs attention: what needs the user, in three groups. A row opens the session it names, or the
 * About screen for an update. An empty list says so in one sentence.
 */
@Composable
fun AttentionScreen(
    items: List<AttentionItem>,
    onOpenSession: (String) -> Unit,
    onOpenAbout: () -> Unit,
    onBack: () -> Unit,
) {
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = stringResource(R.string.route_attention_title),
                navigationIcon = { BackButton(onBack) },
            )
        },
    ) { padding ->
        if (items.isEmpty()) {
            Text(
                text = stringResource(R.string.attention_empty),
                style = MaterialTheme.typography.bodyLarge,
                color = TetherTheme.colors.text2,
                modifier = Modifier.padding(padding).padding(TetherSpace.space4),
            )
        } else {
            AttentionGroups(
                items = items,
                onOpenSession = onOpenSession,
                onOpenAbout = onOpenAbout,
                modifier = Modifier.padding(padding),
            )
        }
    }
}

@Composable
private fun AttentionGroups(
    items: List<AttentionItem>,
    onOpenSession: (String) -> Unit,
    onOpenAbout: () -> Unit,
    modifier: Modifier,
) {
    val now = remember(items) { System.currentTimeMillis() }
    LazyColumn(
        modifier = modifier.fillMaxSize(),
        contentPadding = PaddingValues(vertical = TetherSpace.space2),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
    ) {
        approvalGroup(items.filterIsInstance<AttentionItem.Approval>(), onOpenSession)
        failureGroup(items.filterIsInstance<AttentionItem.Failure>(), now, onOpenSession)
        updateGroup(items.filterIsInstance<AttentionItem.Update>(), onOpenAbout)
    }
}

private fun LazyListScope.approvalGroup(
    rows: List<AttentionItem.Approval>,
    onOpenSession: (String) -> Unit,
) {
    if (rows.isEmpty()) return
    item {
        AttentionGroup(titleRes = R.string.attention_group_waiting) {
            rows.forEachIndexed { index, row ->
                if (index > 0) RowDivider()
                ApprovalRow(row = row, onClick = { onOpenSession(row.sessionId) })
            }
        }
    }
}

private fun LazyListScope.failureGroup(
    rows: List<AttentionItem.Failure>,
    now: Long,
    onOpenSession: (String) -> Unit,
) {
    if (rows.isEmpty()) return
    item {
        AttentionGroup(titleRes = R.string.attention_group_failed) {
            rows.forEachIndexed { index, row ->
                if (index > 0) RowDivider()
                FailureRow(row = row, now = now, onClick = { onOpenSession(row.sessionId) })
            }
        }
    }
}

private fun LazyListScope.updateGroup(
    rows: List<AttentionItem.Update>,
    onOpenAbout: () -> Unit,
) {
    if (rows.isEmpty()) return
    item {
        AttentionGroup(titleRes = R.string.attention_group_updates) {
            rows.forEach { row -> UpdateRow(row = row, onClick = onOpenAbout) }
        }
    }
}

@Composable
private fun AttentionGroup(
    titleRes: Int,
    rows: @Composable () -> Unit,
) {
    SectionTitle(text = stringResource(titleRes))
    ListGroup(modifier = Modifier.padding(horizontal = TetherSpace.space4)) {
        rows()
    }
}

@Composable
private fun RowDivider() {
    HorizontalDivider(thickness = TetherBorder.hairline, color = TetherTheme.colors.line)
}

@Composable
internal fun ApprovalRow(
    row: AttentionItem.Approval,
    onClick: () -> Unit,
) {
    ListRow(
        title = sessionTitleText(row.title),
        subtitle = stringResource(R.string.attention_approval_sub, row.projectName),
        leading = { StatusMark(state = SessionState.Waiting) },
        trailing = { Chevron() },
        onClick = onClick,
    )
}

@Composable
internal fun FailureRow(
    row: AttentionItem.Failure,
    now: Long,
    onClick: () -> Unit,
) {
    val ago = relativeTime(now, row.updatedAt).text()
    ListRow(
        title = sessionTitleText(row.title),
        subtitle = stringResource(R.string.attention_failed_sub, row.projectName, ago),
        leading = { StatusMark(state = SessionState.Failed) },
        trailing = { Chevron() },
        onClick = onClick,
    )
}

@Composable
internal fun UpdateRow(
    row: AttentionItem.Update,
    onClick: () -> Unit,
) {
    ListRow(
        title = stringResource(R.string.updates_available_title, row.newVersion),
        subtitle = stringResource(R.string.attention_update_sub, row.currentVersion),
        leading = {
            TetherIcon(
                icon = TetherIcons.CircleDot,
                contentDescription = null,
                tint = TetherTheme.colors.text2,
            )
        },
        trailing = { Chevron() },
        onClick = onClick,
    )
}

/** One Needs attention row for any item. Home uses it, so both lists word and act the same. */
@Composable
internal fun AttentionItemRow(
    item: AttentionItem,
    now: Long,
    onOpenSession: (String) -> Unit,
    onOpenAbout: () -> Unit,
) {
    when (item) {
        is AttentionItem.Approval ->
            ApprovalRow(
                row = item,
                onClick = { onOpenSession(item.sessionId) },
            )
        is AttentionItem.Failure ->
            FailureRow(row = item, now = now, onClick = { onOpenSession(item.sessionId) })
        is AttentionItem.Update -> UpdateRow(row = item, onClick = onOpenAbout)
    }
}

@Composable
private fun Chevron() {
    TetherIcon(
        icon = TetherIcons.ChevronRight,
        contentDescription = null,
        tint = TetherTheme.colors.text3,
    )
}
