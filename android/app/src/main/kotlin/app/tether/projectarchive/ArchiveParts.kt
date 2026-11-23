package app.tether.projectarchive

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.PrimaryButton
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherSpace
import app.tether.home.ProjectNameProblem

/** The explanation of what an import does, shown on the empty import screen. */
@Composable
internal fun ImportExplanation() {
    Text(
        text = stringResource(R.string.archive_import_explain),
        style = MaterialTheme.typography.bodyLarge,
        color = TetherTheme.colors.text2,
    )
}

/** The accent button that opens the file picker. */
@Composable
internal fun PrimaryChooseFile(onChooseFile: () -> Unit) {
    PrimaryButton(
        label = stringResource(R.string.archive_choose_zip),
        onClick = onChooseFile,
        modifier = Modifier.fillMaxWidth(),
    )
}

/** The picked file's name and size, with a neutral icon. */
@Composable
internal fun FileSummary(state: ImportProjectUiState) {
    Column(
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space1),
        modifier = Modifier.fillMaxWidth(),
    ) {
        state.fileName?.let { name ->
            Text(
                text = name,
                style = MaterialTheme.typography.titleSmall,
                color = TetherTheme.colors.text,
            )
        }
        state.sizeBytes?.let { bytes ->
            Text(
                text = sizeText(formatArchiveSize(bytes)),
                style = MaterialTheme.typography.bodySmall,
                color = TetherTheme.colors.text3,
            )
        }
    }
}

/** A size as words: "512 KB" or "4.2 MB". */
@Composable
internal fun sizeText(size: ArchiveSize): String =
    when (size.unit) {
        ArchiveSizeUnit.Kilobytes -> stringResource(R.string.archive_size_kb, size.amount)
        ArchiveSizeUnit.Megabytes -> stringResource(R.string.archive_size_mb, size.amount)
    }

/** The project name field. A problem replaces the helper line, in the danger color and as text. */
@Composable
internal fun ProjectNameField(
    state: ImportProjectUiState,
    onNameChange: (String) -> Unit,
) {
    val problem = state.nameProblem
    val supporting =
        when (problem) {
            ProjectNameProblem.TooLong -> stringResource(R.string.archive_name_too_long)
            ProjectNameProblem.ControlCharacter -> stringResource(R.string.archive_name_control)
            null -> stringResource(R.string.archive_name_helper)
        }
    OutlinedTextField(
        value = state.name,
        onValueChange = onNameChange,
        label = { Text(stringResource(R.string.archive_project_name_label)) },
        supportingText = {
            Text(
                text = supporting,
                style = MaterialTheme.typography.bodySmall,
                color =
                    if (problem ==
                        null
                    ) {
                        TetherTheme.colors.text2
                    } else {
                        TetherTheme.colors.danger
                    },
            )
        },
        isError = problem != null,
        singleLine = true,
        modifier = Modifier.fillMaxWidth(),
    )
}

/** The accent Import button. It stays off until the file and the name are usable. */
@Composable
internal fun PrimaryImport(
    state: ImportProjectUiState,
    onImport: () -> Unit,
) {
    PrimaryButton(
        label = stringResource(R.string.archive_import_button),
        onClick = onImport,
        enabled = state.canImport,
        modifier = Modifier.fillMaxWidth(),
    )
}

/** A text button that lets the person pick a different file. */
@Composable
internal fun ChangeFile(onChooseFile: () -> Unit) {
    TetherTextButton(
        label = stringResource(R.string.archive_change_file),
        onClick = onChooseFile,
        icon = TetherIcons.Package,
    )
}

/** Cancel during an import. A text button, not the accent. */
@Composable
internal fun ImportCancel(onCancel: () -> Unit) {
    TetherTextButton(label = stringResource(R.string.archive_cancel), onClick = onCancel)
}

/** An indeterminate bar with one line of text. Progress keeps its motion under reduced motion. */
@Composable
internal fun ProgressWithText(text: String) {
    Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space3)) {
        LinearProgressIndicator(
            modifier = Modifier.fillMaxWidth(),
            color = TetherTheme.colors.text2,
            trackColor = TetherTheme.colors.surfaceSunken,
        )
        Text(
            text = text,
            style = MaterialTheme.typography.bodyMedium,
            color = TetherTheme.colors.text2,
        )
    }
}
