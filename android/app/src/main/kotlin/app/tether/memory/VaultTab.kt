package app.tether.memory

import androidx.annotation.StringRes
import androidx.compose.runtime.Composable
import androidx.compose.ui.res.stringResource
import app.tether.R

/** The three views of the vault. Each is its own screen; the tabs move between them. */
enum class VaultTab(
    @StringRes val label: Int,
) {
    /** The note list and search. */
    Notes(R.string.vault_tab_notes),

    /** The map of linked notes as text. */
    Map(R.string.vault_tab_map),

    /** Notes with no links into them. */
    Lint(R.string.vault_tab_lint),
}

/** The Notes, Map, and Lint tabs. Choosing the tab already shown does nothing. */
@Composable
internal fun VaultTabs(
    selected: VaultTab,
    onSelect: (VaultTab) -> Unit,
) {
    val tabs = VaultTab.entries
    AreaTabs(
        labels = tabs.map { stringResource(it.label) },
        selected = tabs.indexOf(selected),
        onSelect = { index -> if (tabs[index] != selected) onSelect(tabs[index]) },
    )
}
