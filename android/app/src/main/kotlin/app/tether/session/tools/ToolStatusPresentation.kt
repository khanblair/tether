package app.tether.session.tools

import app.tether.session.agent.ToolStatus

/** The icon that stands for a tool status. The composable maps each glyph to a Lucide icon. */
enum class ToolStatusGlyph {
    Dot,
    Pause,
    Spinner,
    Check,
    Cross,
    Clock,
    Ban,
    Shield,
    Stop,
}

/** The text that names a tool status. The composable maps each label to a string resource. */
enum class ToolStatusLabel {
    Proposed,
    Allowed,
    NeedsApproval,
    Running,
    Done,
    Failed,
    TimedOut,
    Denied,
    Blocked,
    Cancelled,
}

/**
 * The color role of a status. Only failed, timed out, and blocked calls use [Danger]. A status
 * always keeps its glyph and label, so the tone is never the only signal.
 */
enum class ToolTone {
    Neutral,
    Success,
    Danger,
}

/** How one [ToolStatus] looks: its glyph, its label, and its tone. */
data class ToolStatusPresentation(
    val glyph: ToolStatusGlyph,
    val label: ToolStatusLabel,
    val tone: ToolTone,
)

/**
 * The presentation of each tool status. The `when` has no `else` branch, so a new [ToolStatus]
 * fails to compile until it gets a row here.
 */
fun toolStatusPresentation(status: ToolStatus): ToolStatusPresentation =
    when (status) {
        ToolStatus.Proposed ->
            ToolStatusPresentation(ToolStatusGlyph.Dot, ToolStatusLabel.Proposed, ToolTone.Neutral)
        ToolStatus.Allowed ->
            ToolStatusPresentation(ToolStatusGlyph.Dot, ToolStatusLabel.Allowed, ToolTone.Neutral)
        ToolStatus.AwaitingApproval ->
            ToolStatusPresentation(
                ToolStatusGlyph.Pause,
                ToolStatusLabel.NeedsApproval,
                ToolTone.Neutral,
            )
        ToolStatus.Running ->
            ToolStatusPresentation(
                ToolStatusGlyph.Spinner,
                ToolStatusLabel.Running,
                ToolTone.Neutral,
            )
        ToolStatus.Succeeded ->
            ToolStatusPresentation(ToolStatusGlyph.Check, ToolStatusLabel.Done, ToolTone.Success)
        ToolStatus.Failed ->
            ToolStatusPresentation(ToolStatusGlyph.Cross, ToolStatusLabel.Failed, ToolTone.Danger)
        ToolStatus.TimedOut ->
            ToolStatusPresentation(ToolStatusGlyph.Clock, ToolStatusLabel.TimedOut, ToolTone.Danger)
        ToolStatus.Denied ->
            ToolStatusPresentation(ToolStatusGlyph.Ban, ToolStatusLabel.Denied, ToolTone.Neutral)
        ToolStatus.Blocked ->
            ToolStatusPresentation(ToolStatusGlyph.Shield, ToolStatusLabel.Blocked, ToolTone.Danger)
        ToolStatus.Cancelled ->
            ToolStatusPresentation(
                ToolStatusGlyph.Stop,
                ToolStatusLabel.Cancelled,
                ToolTone.Neutral,
            )
    }
