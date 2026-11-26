package app.tether.preview

import androidx.compose.runtime.Composable
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.components.TetherTextButton

/** The small text button that opens a preview, from a tool card or an HTML code block. */
@Composable
fun PreviewButton(onClick: () -> Unit) {
    TetherTextButton(label = stringResource(R.string.preview_open_action), onClick = onClick)
}
