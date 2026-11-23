package app.tether.session.tools

import androidx.compose.ui.graphics.vector.ImageVector
import app.tether.design.Ban
import app.tether.design.Clock
import app.tether.design.FilePlus
import app.tether.design.GitCommit
import app.tether.design.GitPullRequest
import app.tether.design.Play
import app.tether.design.ShieldAlert
import app.tether.design.TetherIcons

/** The Lucide icon for a status glyph. */
internal fun ToolStatusGlyph.icon(): ImageVector =
    when (this) {
        ToolStatusGlyph.Dot -> TetherIcons.CircleDot
        ToolStatusGlyph.Pause -> TetherIcons.CirclePause
        ToolStatusGlyph.Spinner -> TetherIcons.Loader
        ToolStatusGlyph.Check -> TetherIcons.CircleCheck
        ToolStatusGlyph.Cross -> TetherIcons.CircleX
        ToolStatusGlyph.Clock -> TetherIcons.Clock
        ToolStatusGlyph.Ban -> TetherIcons.Ban
        ToolStatusGlyph.Shield -> TetherIcons.ShieldAlert
        ToolStatusGlyph.Stop -> TetherIcons.X
    }

/** The Lucide icon that names what a tool kind does. Unknown tools get a neutral dot. */
internal fun ToolTitleKind.icon(): ImageVector =
    when (this) {
        ToolTitleKind.ReadFile -> TetherIcons.FileText
        ToolTitleKind.WriteFile -> TetherIcons.FilePlus
        ToolTitleKind.EditFile -> TetherIcons.FilePen
        ToolTitleKind.DeleteFile -> TetherIcons.Trash2
        ToolTitleKind.ListDir -> TetherIcons.FolderOpen
        ToolTitleKind.Search -> TetherIcons.Search
        ToolTitleKind.Run -> TetherIcons.Terminal
        ToolTitleKind.CiRun,
        ToolTitleKind.CiLogs,
        -> TetherIcons.Play
        ToolTitleKind.GitStatus,
        ToolTitleKind.GitDiff,
        ToolTitleKind.GitBranch,
        -> TetherIcons.GitBranch
        ToolTitleKind.GitLog,
        ToolTitleKind.GitCommit,
        -> TetherIcons.GitCommit
        ToolTitleKind.GitPush -> TetherIcons.Upload
        ToolTitleKind.PrCreate,
        ToolTitleKind.PrStatus,
        -> TetherIcons.GitPullRequest
        ToolTitleKind.Unknown -> TetherIcons.CircleDot
    }
