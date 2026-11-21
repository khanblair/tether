package app.tether.memory

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Scaffold
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.ListGroup
import app.tether.design.components.ListRow
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.shell.BackButton

/** The entry point from Settings and the drawer: links to Memory, Skills, and Vault. */
@Composable
fun SkillsMemoryHubScreen(
    onMemory: () -> Unit,
    onSkills: () -> Unit,
    onVault: () -> Unit,
    onBack: () -> Unit,
) {
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = stringResource(R.string.settings_area_skills_title),
                navigationIcon = { BackButton(onBack) },
            )
        },
    ) { padding ->
        Column(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(padding)
                    .verticalScroll(rememberScrollState())
                    .padding(TetherSpace.space4),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Column(
                modifier = Modifier.widthIn(max = TetherSize.transcriptMax).fillMaxWidth(),
                verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
            ) {
                ListGroup {
                    ListRow(
                        title = stringResource(R.string.hub_memory_title),
                        subtitle = stringResource(R.string.hub_memory_body),
                        onClick = onMemory,
                    )
                    HorizontalDivider(
                        color = TetherTheme.colors.line,
                        thickness = TetherBorder.hairline,
                    )
                    ListRow(
                        title = stringResource(R.string.hub_skills_title),
                        subtitle = stringResource(R.string.hub_skills_body),
                        onClick = onSkills,
                    )
                    HorizontalDivider(
                        color = TetherTheme.colors.line,
                        thickness = TetherBorder.hairline,
                    )
                    ListRow(
                        title = stringResource(R.string.hub_vault_title),
                        subtitle = stringResource(R.string.hub_vault_body),
                        onClick = onVault,
                    )
                }
            }
        }
    }
}
