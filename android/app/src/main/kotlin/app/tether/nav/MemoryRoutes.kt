package app.tether.nav

import android.net.Uri
import androidx.compose.runtime.Composable
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.navigation.NavBackStackEntry
import androidx.navigation.NavGraphBuilder
import androidx.navigation.compose.composable
import app.tether.R
import app.tether.memory.MemoryAreaGateways
import app.tether.memory.MemoryScreen
import app.tether.memory.MemoryViewModel
import app.tether.memory.SampleSkillsScreen
import app.tether.memory.SampleSkillsViewModel
import app.tether.memory.SkillDetailScreen
import app.tether.memory.SkillDetailViewModel
import app.tether.memory.SkillsMemoryHubScreen
import app.tether.memory.SkillsScreen
import app.tether.memory.SkillsViewModel
import app.tether.memory.VaultEditScreen
import app.tether.memory.VaultEditViewModel
import app.tether.memory.VaultLintScreen
import app.tether.memory.VaultLintViewModel
import app.tether.memory.VaultListScreen
import app.tether.memory.VaultListViewModel
import app.tether.memory.VaultMapScreen
import app.tether.memory.VaultMapViewModel
import app.tether.memory.VaultNoteScreen
import app.tether.memory.VaultNoteViewModel
import app.tether.memory.VaultTab
import app.tether.shell.PlaceholderScreen

/** The Memory screen. */
const val MEMORY_ROUTE = "skills-memory/memory"

/** The Skills list. */
const val SKILLS_ROUTE = "skills-memory/skills"

/** One skill. The argument is the skill's folder name. */
const val SKILL_DETAIL_ROUTE = "skills-memory/skills/{folder}"

/** The sample skills gallery. Installing a sample copies it into the skills folder. */
const val SKILL_INSTALL_ROUTE = "skills-memory/install-skill"

/** The vault's Notes tab. */
const val VAULT_ROUTE = "skills-memory/vault"

/** One vault note. The argument is the note's relative path, encoded. */
const val VAULT_NOTE_ROUTE = "skills-memory/vault/note/{path}"

/** Editing an existing note. The argument is its path, encoded. */
const val VAULT_EDIT_ROUTE = "skills-memory/vault/edit/{path}"

/** Writing a new note. */
const val VAULT_NEW_ROUTE = "skills-memory/vault/new"

/** The vault map. The argument is the note the map starts from, encoded. */
const val VAULT_MAP_ROUTE = "skills-memory/vault/map/{start}"

/** The vault's Lint tab. */
const val VAULT_LINT_ROUTE = "skills-memory/vault/lint"

/** Vault sync and export. Not available until the core exposes sync. */
const val VAULT_SYNC_ROUTE = "skills-memory/vault-sync"

/** The note the map starts from when the vault's Map tab is opened. */
private const val CATALOG_NOTE = "index.md"

/** The path of the skill in [folder]. */
fun skillDetailPath(folder: String) = "skills-memory/skills/${Uri.encode(folder)}"

/** The path of the note at [path]. */
fun vaultNotePath(path: String) = "skills-memory/vault/note/${Uri.encode(path)}"

/** The path of the editor for the note at [path]. */
fun vaultEditPath(path: String) = "skills-memory/vault/edit/${Uri.encode(path)}"

/** The path of the map that starts from [start]. */
fun vaultMapPath(start: String) = "skills-memory/vault/map/${Uri.encode(start)}"

/** Registers the skills and memory area: the hub, Memory, Skills, and the vault. */
internal fun NavGraphBuilder.memoryRoutes(
    gateways: MemoryAreaGateways,
    nav: TetherNav,
) {
    hubAndMemoryRoutes(gateways, nav)
    skillRoutes(gateways, nav)
    vaultRoutes(gateways, nav)
    vaultToolRoutes(gateways, nav)
    vaultNoteRoutes(gateways, nav)
}

private fun NavGraphBuilder.hubAndMemoryRoutes(
    gateways: MemoryAreaGateways,
    nav: TetherNav,
) {
    composable(SKILLS_MEMORY_ROUTE) {
        SkillsMemoryHubScreen(
            onMemory = { nav.open(MEMORY_ROUTE) },
            onSkills = { nav.open(SKILLS_ROUTE) },
            onVault = { nav.open(VAULT_ROUTE) },
            onBack = nav::back,
        )
    }
    composable(MEMORY_ROUTE) {
        MemoryScreen(
            viewModel =
                viewModel(
                    factory = viewModelFactory { initializer { MemoryViewModel(gateways.memory) } },
                ),
            onBack = nav::back,
        )
    }
    composable(SKILL_INSTALL_ROUTE) {
        SampleSkillsScreen(
            viewModel =
                viewModel(
                    factory =
                        viewModelFactory {
                            initializer { SampleSkillsViewModel(gateways.skills) }
                        },
                ),
            onBack = nav::back,
        )
    }
}

private fun NavGraphBuilder.skillRoutes(
    gateways: MemoryAreaGateways,
    nav: TetherNav,
) {
    composable(SKILLS_ROUTE) {
        SkillsScreen(
            viewModel =
                viewModel(
                    factory = viewModelFactory { initializer { SkillsViewModel(gateways.skills) } },
                ),
            onOpen = { nav.open(skillDetailPath(it)) },
            onInstall = { nav.open(SKILL_INSTALL_ROUTE) },
            onBack = nav::back,
        )
    }
    composable(SKILL_DETAIL_ROUTE) { entry ->
        val folder = entry.argument("folder")
        SkillDetailScreen(
            viewModel =
                viewModel(
                    factory =
                        viewModelFactory {
                            initializer { SkillDetailViewModel(gateways.skills, folder) }
                        },
                ),
            onBack = nav::back,
        )
    }
}

private fun NavGraphBuilder.vaultRoutes(
    gateways: MemoryAreaGateways,
    nav: TetherNav,
) {
    composable(VAULT_ROUTE) {
        VaultListScreen(
            viewModel =
                viewModel(
                    factory =
                        viewModelFactory {
                            initializer {
                                VaultListViewModel(
                                    gateways.vault,
                                )
                            }
                        },
                ),
            onOpen = { nav.open(vaultNotePath(it)) },
            onNew = { nav.open(VAULT_NEW_ROUTE) },
            onOpenTab = { openVaultTab(it, nav) },
            onSync = { nav.open(VAULT_SYNC_ROUTE) },
            onBack = nav::back,
        )
    }
    composable(VAULT_SYNC_ROUTE) {
        NotAvailable(nav, R.string.vault_sync_na_title, R.string.vault_sync_na_body)
    }
    composable(VAULT_NEW_ROUTE) {
        VaultEditScreen(
            viewModel =
                viewModel(
                    factory =
                        viewModelFactory {
                            initializer {
                                VaultEditViewModel(
                                    gateways.vault,
                                    null,
                                )
                            }
                        },
                ),
            onSaved = { saved ->
                nav.back()
                nav.open(vaultNotePath(saved))
            },
            onBack = nav::back,
        )
    }
}

private fun NavGraphBuilder.vaultToolRoutes(
    gateways: MemoryAreaGateways,
    nav: TetherNav,
) {
    composable(VAULT_LINT_ROUTE) {
        VaultLintScreen(
            viewModel =
                viewModel(
                    factory =
                        viewModelFactory {
                            initializer {
                                VaultLintViewModel(
                                    gateways.vault,
                                )
                            }
                        },
                ),
            onOpen = { nav.open(vaultNotePath(it)) },
            onOpenTab = { openVaultTab(it, nav) },
            onBack = nav::back,
        )
    }
    composable(VAULT_MAP_ROUTE) { entry ->
        val start = entry.argument("start")
        VaultMapScreen(
            viewModel =
                viewModel(
                    factory =
                        viewModelFactory {
                            initializer {
                                VaultMapViewModel(
                                    gateways.vault,
                                    start,
                                )
                            }
                        },
                ),
            onOpenTab = { openVaultTab(it, nav) },
            onBack = nav::back,
        )
    }
}

private fun NavGraphBuilder.vaultNoteRoutes(
    gateways: MemoryAreaGateways,
    nav: TetherNav,
) {
    composable(VAULT_NOTE_ROUTE) { entry ->
        val path = entry.argument("path")
        VaultNoteScreen(
            viewModel =
                viewModel(
                    factory =
                        viewModelFactory {
                            initializer {
                                VaultNoteViewModel(
                                    gateways.vault,
                                    path,
                                )
                            }
                        },
                ),
            onOpenNote = { nav.open(vaultNotePath(it)) },
            onEdit = { nav.open(vaultEditPath(path)) },
            onMap = { nav.open(vaultMapPath(path)) },
            onBack = nav::back,
        )
    }
    composable(VAULT_EDIT_ROUTE) { entry ->
        val path = entry.argument("path")
        VaultEditScreen(
            viewModel =
                viewModel(
                    factory =
                        viewModelFactory {
                            initializer {
                                VaultEditViewModel(
                                    gateways.vault,
                                    path,
                                )
                            }
                        },
                ),
            onSaved = { nav.back() },
            onBack = nav::back,
        )
    }
}

/** Opens the vault tab [tab]. The map starts from the catalog note. */
private fun openVaultTab(
    tab: VaultTab,
    nav: TetherNav,
) {
    when (tab) {
        VaultTab.Notes -> nav.open(VAULT_ROUTE)
        VaultTab.Map -> nav.open(vaultMapPath(CATALOG_NOTE))
        VaultTab.Lint -> nav.open(VAULT_LINT_ROUTE)
    }
}

private fun NavBackStackEntry.argument(name: String): String = arguments?.getString(name).orEmpty()

@Composable
private fun NotAvailable(
    nav: TetherNav,
    titleRes: Int,
    explanationRes: Int,
) {
    PlaceholderScreen(
        title = stringResource(titleRes),
        explanation = stringResource(explanationRes),
        onBack = nav::back,
    )
}
