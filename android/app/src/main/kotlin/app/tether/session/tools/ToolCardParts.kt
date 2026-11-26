package app.tether.session.tools

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.style.TextOverflow
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.SessionState
import app.tether.design.components.StatusMark
import app.tether.design.components.TetherIcon
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.session.agent.ToolStatus

/** The surface of a tool card: a full-width tonal block with the radius for cards. */
@Composable
internal fun ToolCardSurface(
    modifier: Modifier,
    content: @Composable () -> Unit,
) {
    val colors = TetherTheme.colors
    val background = if (TetherTheme.isDark) colors.surface else colors.surfaceSunken
    Column(
        modifier =
            modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(TetherRadius.md))
                .background(background),
    ) {
        content()
    }
}

/**
 * The status of a call: a glyph and a label. The label is announced politely when it changes.
 * A running call shows the spinning mark only when [animateRunning] is true.
 */
@Composable
internal fun ToolStatusLine(
    status: ToolStatus,
    animateRunning: Boolean,
    modifier: Modifier = Modifier,
) {
    val colors = TetherTheme.colors
    val presentation = toolStatusPresentation(status)
    val tint =
        when (presentation.tone) {
            ToolTone.Neutral -> colors.text2
            ToolTone.Success -> colors.success
            ToolTone.Danger -> colors.danger
        }
    Row(
        modifier = modifier.semantics { liveRegion = LiveRegionMode.Polite },
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (presentation.glyph == ToolStatusGlyph.Spinner && animateRunning) {
            StatusMark(SessionState.Running)
        } else {
            TetherIcon(
                icon = presentation.glyph.icon(),
                contentDescription = null,
                tint = tint,
                size = TetherSize.iconS,
            )
        }
        Spacer(Modifier.width(TetherSpace.space1))
        Text(
            text = stringResource(presentation.label.labelRes()),
            style = MaterialTheme.typography.labelSmall,
            color = tint,
        )
    }
}

/**
 * The verb and the object of a title, on one line. A fixed title (git, pull request, CI logs)
 * has no verb form, so its name is the verb. The object is set in the mono style.
 */
@Composable
internal fun ToolTitleText(
    title: ToolTitle,
    modifier: Modifier = Modifier,
) {
    val colors = TetherTheme.colors
    val fixed = fixedTitleRes(title.kind)
    val verbs = verbRes(title.kind)
    val verb =
        when {
            fixed != null -> stringResource(fixed)
            verbs != null -> stringResource(verbs.forTense(title.tense))
            else -> ""
        }
    val subject = if (title.kind == ToolTitleKind.Unknown) title.tool else title.target
    Row(modifier = modifier, verticalAlignment = Alignment.CenterVertically) {
        Text(
            text = verb,
            style = MaterialTheme.typography.labelLarge,
            color = colors.text,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
        if (subject.isNotBlank()) {
            Spacer(Modifier.width(TetherSpace.space1))
            Text(
                text = subject,
                style = TetherTheme.monoLabel,
                color = colors.text2,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.weight(1f),
            )
        }
    }
}

/**
 * The tappable header of a card. When [expandable] is true it is a button of at least 48 dp
 * that reports whether the card is expanded. Otherwise it is a plain row. [trailing] sits after
 * the content, before the chevron, and keeps its own size.
 */
@Composable
internal fun ToolHeader(
    icon: ImageVector,
    expandable: Boolean,
    expanded: Boolean,
    onToggle: () -> Unit,
    modifier: Modifier = Modifier,
    expandRes: Int = R.string.tools_output_expand,
    collapseRes: Int = R.string.tools_output_collapse,
    trailing: (@Composable () -> Unit)? = null,
    content: @Composable () -> Unit,
) {
    val colors = TetherTheme.colors
    val toggleLabel = stringResource(if (expanded) collapseRes else expandRes)
    val stateText =
        stringResource(
            if (expanded) R.string.tools_state_expanded else R.string.tools_state_collapsed,
        )
    val tapModifier =
        if (expandable) {
            Modifier
                .clickable(role = Role.Button, onClickLabel = toggleLabel, onClick = onToggle)
                .semantics { stateDescription = stateText }
        } else {
            Modifier
        }
    Row(
        modifier =
            modifier
                .fillMaxWidth()
                .heightIn(min = TetherSize.touchMin)
                .then(tapModifier)
                .padding(horizontal = TetherSpace.space3, vertical = TetherSpace.space2),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        TetherIcon(
            icon = icon,
            contentDescription = null,
            tint = colors.text2,
            size = TetherSize.iconS,
        )
        Spacer(Modifier.width(TetherSpace.space2))
        Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.Center) {
            content()
        }
        trailing?.invoke()
        if (expandable) {
            Spacer(Modifier.width(TetherSpace.space2))
            TetherIcon(
                icon = if (expanded) TetherIcons.ChevronUp else TetherIcons.ChevronDown,
                contentDescription = null,
                tint = colors.text2,
                size = TetherSize.iconS,
            )
        }
    }
}

/** The pieces of a body line: the inset that all card bodies share. */
internal fun Modifier.toolBodyPadding(): Modifier =
    this.padding(start = TetherSpace.space3, end = TetherSpace.space3, bottom = TetherSpace.space3)
