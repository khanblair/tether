// Lucide icons for the agent's tool cards (ISC license, https://lucide.dev), converted to
// Compose ImageVector. Source: tools/prototype/icons.js. Grid 24 dp, stroke 1.75, round caps.
// Numeric coordinates are icon geometry, so MagicNumber is suppressed for this file.
@file:Suppress("MagicNumber")

package app.tether.design

import androidx.compose.ui.graphics.vector.ImageVector

private val filePlusIcon by lazy {
    tetherIcon("FilePlus") {
        strokeSvg(
            "M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 " +
                "2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z",
        )
        strokeSvg("M14 2v5a1 1 0 0 0 1 1h5")
        strokeSvg("M9 15h6")
        strokeSvg("M12 18v-6")
    }
}

private val triangleAlertIcon by lazy {
    tetherIcon("TriangleAlert") {
        strokeSvg(
            "m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3",
        )
        strokeSvg("M12 9v4")
        strokeSvg("M12 17h.01")
    }
}

private val banIcon by lazy {
    tetherIcon("Ban") {
        strokePath { circle(12f, 12f, 10f) }
        strokeSvg("m4.9 4.9 14.2 14.2")
    }
}

private val clockIcon by lazy {
    tetherIcon("Clock") {
        strokePath { circle(12f, 12f, 10f) }
        strokeSvg("M12 6v6l4 2")
    }
}

private val shieldAlertIcon by lazy {
    tetherIcon("ShieldAlert") {
        strokeSvg(
            "M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 " +
                "1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 " +
                "1 1z",
        )
        strokeSvg("M12 8v4")
        strokeSvg("M12 16h.01")
    }
}

private val infoIcon by lazy {
    tetherIcon("Info") {
        strokePath { circle(12f, 12f, 10f) }
        strokeSvg("M12 16v-4")
        strokeSvg("M12 8h.01")
    }
}

private val gitCommitIcon by lazy {
    tetherIcon("GitCommit") {
        strokePath { circle(12f, 12f, 3f) }
        strokeSvg("M3 12h6")
        strokeSvg("M15 12h6")
    }
}

private val gitPullRequestIcon by lazy {
    tetherIcon("GitPullRequest") {
        strokePath { circle(18f, 18f, 3f) }
        strokePath { circle(6f, 6f, 3f) }
        strokeSvg("M13 6h3a2 2 0 0 1 2 2v7")
        strokeSvg("M6 9v12")
    }
}

private val playIcon by lazy {
    tetherIcon("Play") {
        strokeSvg(
            "M5 5a2 2 0 0 1 3.008-1.728l11.997 6.998a2 2 0 0 1 .003 3.458l-12 7A2 2 0 0 1 5 19z",
        )
    }
}

/** A file with a plus, for a new file. */
val TetherIcons.FilePlus: ImageVector get() = filePlusIcon

/** A warning triangle with an exclamation mark. */
val TetherIcons.TriangleAlert: ImageVector get() = triangleAlertIcon

/** A circle with a diagonal bar, for a denied call. */
val TetherIcons.Ban: ImageVector get() = banIcon

/** A clock face, for a call that timed out. */
val TetherIcons.Clock: ImageVector get() = clockIcon

/** A shield with an exclamation mark, for a blocked call. */
val TetherIcons.ShieldAlert: ImageVector get() = shieldAlertIcon

/** A circle with an i, for a note. */
val TetherIcons.Info: ImageVector get() = infoIcon

/** A commit node on a line, for a git commit. */
val TetherIcons.GitCommit: ImageVector get() = gitCommitIcon

/** Two nodes joined by a line, for a pull request. */
val TetherIcons.GitPullRequest: ImageVector get() = gitPullRequestIcon

/** A play triangle, for a CI run. */
val TetherIcons.Play: ImageVector get() = playIcon
