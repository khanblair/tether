package app.tether.design.components

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.exclude
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSpace

/**
 * A modal bottom sheet with a [title] and [content]. The sheet opens fully expanded, so a field in
 * it is not hidden behind the keyboard. The content rises above the keyboard and keeps clear of the
 * navigation bar. The bar's space is counted once: the keyboard covers it while the keyboard is up.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TetherSheet(
    title: String,
    onDismiss: () -> Unit,
    modifier: Modifier = Modifier,
    content: @Composable ColumnScope.() -> Unit,
) {
    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true),
        modifier = modifier,
        containerColor = TetherTheme.colors.surface,
        shape = RoundedCornerShape(topStart = TetherRadius.lg, topEnd = TetherRadius.lg),
    ) {
        Column(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .imePadding()
                    .windowInsetsPadding(WindowInsets.navigationBars.exclude(WindowInsets.ime))
                    .padding(bottom = TetherSpace.space4),
        ) {
            Text(
                text = title,
                style = MaterialTheme.typography.titleMedium,
                color = TetherTheme.colors.text,
                modifier =
                    Modifier
                        .padding(horizontal = TetherSpace.space4, vertical = TetherSpace.space4)
                        .semantics { heading() },
            )
            content()
        }
    }
}
