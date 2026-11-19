package app.tether.home

import androidx.annotation.StringRes
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.HorizontalDivider
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.ListGroup
import app.tether.design.components.ListRow
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherSheet
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherSpace

/** The New project sheet: clone, start empty, or import a zip. Projects and Home both open it. */
@Composable
internal fun NewProjectSheet(
    onDismiss: () -> Unit,
    onChoose: (NewProjectChoice) -> Unit,
) {
    TetherSheet(title = stringResource(R.string.home_new_project), onDismiss = onDismiss) {
        ListGroup(modifier = Modifier.padding(horizontal = TetherSpace.space4)) {
            SheetRow(
                icon = TetherIcons.GitBranch,
                title = R.string.home_new_project_clone,
                subtitle = R.string.home_new_project_clone_sub,
                onClick = { onChoose(NewProjectChoice.Clone) },
            )
            HorizontalDivider(thickness = TetherBorder.hairline, color = TetherTheme.colors.line)
            SheetRow(
                icon = TetherIcons.FolderOpen,
                title = R.string.home_new_project_empty,
                subtitle = R.string.home_new_project_empty_sub,
                onClick = { onChoose(NewProjectChoice.Empty) },
            )
            HorizontalDivider(thickness = TetherBorder.hairline, color = TetherTheme.colors.line)
            SheetRow(
                icon = TetherIcons.Package,
                title = R.string.home_new_project_import,
                subtitle = R.string.home_new_project_import_sub,
                onClick = { onChoose(NewProjectChoice.Import) },
            )
        }
    }
}

@Composable
private fun SheetRow(
    icon: ImageVector,
    @StringRes title: Int,
    @StringRes subtitle: Int,
    onClick: () -> Unit,
) {
    ListRow(
        title = stringResource(title),
        subtitle = stringResource(subtitle),
        leading = {
            TetherIcon(icon = icon, contentDescription = null, tint = TetherTheme.colors.text2)
        },
        onClick = onClick,
    )
}
