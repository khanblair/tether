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
internal fun ToolTitleKind.icon(): ImageVector = TOOL_TITLE_ICONS[this] ?: TetherIcons.CircleDot

/** The icon for each tool kind. A kind missing from this table gets the neutral dot. */
private val TOOL_TITLE_ICONS: Map<ToolTitleKind, ImageVector> =
    buildMap {
        put(ToolTitleKind.ReadFile, TetherIcons.FileText)
        put(ToolTitleKind.WriteFile, TetherIcons.FilePlus)
        put(ToolTitleKind.EditFile, TetherIcons.FilePen)
        put(ToolTitleKind.DeleteFile, TetherIcons.Trash2)
        put(ToolTitleKind.ListDir, TetherIcons.FolderOpen)
        put(ToolTitleKind.Search, TetherIcons.Search)
        put(ToolTitleKind.Run, TetherIcons.Terminal)
        put(ToolTitleKind.RunServer, TetherIcons.Terminal)
        put(ToolTitleKind.ProcessList, TetherIcons.FolderOpen)
        put(ToolTitleKind.ProcessLogs, TetherIcons.FileText)
        put(ToolTitleKind.ProcessStop, TetherIcons.X)
        put(ToolTitleKind.Preview, TetherIcons.Globe)
        put(ToolTitleKind.CiRun, TetherIcons.Play)
        put(ToolTitleKind.CiLogs, TetherIcons.Play)
        put(ToolTitleKind.GitStatus, TetherIcons.GitBranch)
        put(ToolTitleKind.GitDiff, TetherIcons.GitBranch)
        put(ToolTitleKind.GitBranch, TetherIcons.GitBranch)
        put(ToolTitleKind.GitInit, TetherIcons.GitBranch)
        put(ToolTitleKind.GithubRepoCreate, TetherIcons.GitBranch)
        put(ToolTitleKind.GitLog, TetherIcons.GitCommit)
        put(ToolTitleKind.GitCommit, TetherIcons.GitCommit)
        put(ToolTitleKind.GitPush, TetherIcons.Upload)
        put(ToolTitleKind.PrCreate, TetherIcons.GitPullRequest)
        put(ToolTitleKind.PrStatus, TetherIcons.GitPullRequest)
        put(ToolTitleKind.Unknown, TetherIcons.CircleDot)
    }
