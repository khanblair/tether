package app.tether.design.components

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.RowScope
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import app.tether.design.TetherTheme

/** Top app bar with a title, an optional subtitle, a navigation slot, and action slots. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TetherTopBar(
    title: String,
    modifier: Modifier = Modifier,
    subtitle: String? = null,
    navigationIcon: @Composable () -> Unit = {},
    actions: @Composable RowScope.() -> Unit = {},
) {
    TopAppBar(
        modifier = modifier,
        title = {
            Column {
                Text(text = title, style = MaterialTheme.typography.titleMedium)
                if (subtitle != null) {
                    Text(
                        text = subtitle,
                        style = MaterialTheme.typography.labelSmall,
                        color = TetherTheme.colors.text3,
                    )
                }
            }
        },
        navigationIcon = navigationIcon,
        actions = actions,
        colors =
            TopAppBarDefaults.topAppBarColors(
                containerColor = TetherTheme.colors.bg,
                titleContentColor = TetherTheme.colors.text,
                navigationIconContentColor = TetherTheme.colors.text2,
                actionIconContentColor = TetherTheme.colors.text2,
            ),
    )
}
