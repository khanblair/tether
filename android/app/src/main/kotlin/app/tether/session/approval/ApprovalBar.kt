package app.tether.session.approval

import androidx.compose.foundation.background
import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.PrimaryButton
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherIconButton
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherElevation
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.session.agent.AgentLoopFailure
import app.tether.session.agent.ApprovalUi

/** Characters of the target shown when collapsed. The rest is cut in the middle. */
private const val COLLAPSED_TARGET_CHARS = 30

/**
 * The first pending approval, above the composer. Allow is the one accent on the bar. [position]
 * and [total] show "1 of 3" when more than one approval waits. Always allow shows only when the
 * call gets a rule. Pushes say in the expanded detail that they are asked every time.
 */
@Composable
fun ApprovalBar(
    approval: ApprovalUi,
    position: Int,
    total: Int,
    expanded: Boolean,
    busy: Boolean,
    failure: AgentLoopFailure?,
    onToggleExpanded: () -> Unit,
    onAllow: () -> Unit,
    onDeny: () -> Unit,
    onAlwaysAllow: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val kind =
        remember(approval.tool, approval.target) { approvalKind(approval.tool, approval.target) }
    val canAlways =
        remember(approval.tool, approval.target) {
            alwaysAllowRule(approval.tool, approval.target) !=
                null
        }
    val focus = remember { FocusRequester() }
    LaunchedEffect(approval.toolCallId) { focus.requestFocus() }
    val shape = RoundedCornerShape(TetherRadius.md)
    Row(
        modifier =
            modifier
                .fillMaxWidth()
                .heightIn(min = TetherSize.approvalBarMin)
                .height(IntrinsicSize.Min)
                .shadow(TetherElevation.level2, shape)
                .clip(shape)
                .background(TetherTheme.colors.surfaceRaised)
                .focusRequester(focus)
                .focusable(),
    ) {
        // The 4 dp accent stripe. It is the one place the accent marks the whole bar.
        Box(
            modifier =
                Modifier
                    .width(TetherSpace.space1)
                    .fillMaxHeight()
                    .background(TetherTheme.colors.accent),
        )
        Column(
            modifier =
                Modifier
                    .weight(1f)
                    .padding(horizontal = TetherSpace.space3, vertical = TetherSpace.space3),
        ) {
            BarHeader(kind = kind, position = position, total = total)
            TargetRow(target = approval.target, expanded = expanded, onToggle = onToggleExpanded)
            if (expanded && (kind == ApprovalKind.Push || kind == ApprovalKind.ForcePush)) {
                Text(
                    text = stringResource(R.string.approval_push_asked_every_time),
                    style = MaterialTheme.typography.bodySmall,
                    color = TetherTheme.colors.text3,
                    modifier = Modifier.padding(top = TetherSpace.space2),
                )
            }
            if (failure != null) {
                InlineBanner(
                    text = stringResource(failure.messageRes()),
                    tone = BannerTone.Danger,
                    modifier = Modifier.padding(top = TetherSpace.space2),
                )
            }
            BarActions(
                canAlways = canAlways,
                busy = busy,
                onDeny = onDeny,
                onAlwaysAllow = onAlwaysAllow,
                onAllow = onAllow,
            )
        }
    }
}

/** The kind icon, the kind summary, and the position in the waiting list. */
@Composable
private fun BarHeader(
    kind: ApprovalKind,
    position: Int,
    total: Int,
) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        TetherIcon(
            icon = kind.icon(),
            contentDescription = null,
            size = TetherSize.iconS,
            tint =
                if (kind ==
                    ApprovalKind.ForcePush
                ) {
                    TetherTheme.colors.danger
                } else {
                    TetherTheme.colors.text2
                },
        )
        Spacer(Modifier.width(TetherSpace.space2))
        Text(
            text = stringResource(kind.labelRes()),
            style = MaterialTheme.typography.titleSmall,
            color = TetherTheme.colors.text,
            modifier = Modifier.weight(1f).semantics { liveRegion = LiveRegionMode.Polite },
        )
        if (total > 1) {
            Text(
                text = stringResource(R.string.approval_position, position, total),
                style = MaterialTheme.typography.labelSmall,
                color = TetherTheme.colors.text3,
            )
        }
    }
}

/** The exact target in mono, cut in the middle until the user expands it. */
@Composable
private fun TargetRow(
    target: String,
    expanded: Boolean,
    onToggle: () -> Unit,
) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(top = TetherSpace.space2),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = if (expanded) target else middleEllipsize(target, COLLAPSED_TARGET_CHARS),
            style = TetherTheme.mono,
            color = TetherTheme.colors.text,
            maxLines = if (expanded) Int.MAX_VALUE else 1,
            modifier =
                Modifier
                    .weight(1f)
                    .background(TetherTheme.colors.codeBg, RoundedCornerShape(TetherRadius.sm))
                    .padding(TetherSpace.space2),
        )
        TetherIconButton(
            icon = if (expanded) TetherIcons.ChevronUp else TetherIcons.ChevronDown,
            contentDescription =
                stringResource(
                    if (expanded) R.string.approval_collapse else R.string.approval_expand,
                ),
            onClick = onToggle,
        )
    }
}

/** Deny and Always allow on the left, Allow under the thumb on the right. */
@Composable
private fun BarActions(
    canAlways: Boolean,
    busy: Boolean,
    onDeny: () -> Unit,
    onAlwaysAllow: () -> Unit,
    onAllow: () -> Unit,
) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(top = TetherSpace.space2),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(TetherSpace.space1),
    ) {
        TetherTextButton(
            label = stringResource(R.string.approval_deny),
            onClick = onDeny,
            enabled = !busy,
        )
        if (canAlways) {
            TetherTextButton(
                label = stringResource(R.string.approval_always),
                onClick = onAlwaysAllow,
                icon = TetherIcons.ChevronDown,
                enabled = !busy,
            )
        }
        Spacer(Modifier.weight(1f))
        PrimaryButton(
            label = stringResource(R.string.approval_allow),
            onClick = onAllow,
            enabled = !busy,
            loading = busy,
        )
    }
}
