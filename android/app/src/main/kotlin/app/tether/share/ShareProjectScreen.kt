package app.tether.share

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherSpace
import app.tether.home.ProjectSummary
import app.tether.shell.BackButton

/** Lets the user associate incoming text with a project before a session is created. */
@Composable
fun ShareProjectScreen(
    shared: SharedText?,
    projects: List<ProjectSummary>,
    onChoose: (String) -> Unit,
    onBack: () -> Unit,
) {
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = stringResource(R.string.share_project_title),
                navigationIcon = { BackButton(onBack) },
            )
        },
    ) { padding ->
        Column(modifier = Modifier.fillMaxSize().padding(padding).padding(TetherSpace.space4)) {
            Text(
                text = shared?.value.orEmpty(),
                color = TetherTheme.colors.text,
                maxLines = 6,
            )
            val selected = shared?.projectId
            if (projects.isEmpty()) {
                Text(
                    text = stringResource(R.string.share_no_projects),
                    color = TetherTheme.colors.text2,
                    modifier = Modifier.padding(top = TetherSpace.space4),
                )
            } else {
                LazyColumn(modifier = Modifier.fillMaxWidth()) {
                    items(projects, key = ProjectSummary::id) { project ->
                        TextButton(onClick = { onChoose(project.id) }) {
                            Text(
                                text =
                                    if (selected == project.id) {
                                        stringResource(
                                            R.string.share_selected_project,
                                            project.name,
                                        )
                                    } else {
                                        project.name
                                    },
                            )
                        }
                    }
                }
            }
        }
    }
}
