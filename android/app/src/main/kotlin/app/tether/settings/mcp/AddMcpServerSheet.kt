package app.tether.settings.mcp

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.PrimaryButton
import app.tether.design.components.SecretField
import app.tether.design.components.TetherIconButton
import app.tether.design.components.TetherSheet
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherSpace

/** The add sheet with its view model. It collects the state and passes each action down. */
@Composable
fun AddMcpServerRoute(
    viewModel: AddMcpServerViewModel,
    onDone: () -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    val callbacks =
        remember(viewModel) {
            AddMcpServerCallbacks(
                onNameChange = viewModel::onNameChange,
                onUrlChange = viewModel::onUrlChange,
                onHeaderNameChange = viewModel::onHeaderNameChange,
                onHeaderValueChange = viewModel::onHeaderValueChange,
                onToggleReveal = viewModel::toggleReveal,
                onAddHeader = viewModel::addHeader,
                onRemoveHeader = viewModel::removeHeader,
                onAdd = viewModel::add,
                onDismissProblem = viewModel::dismissProblem,
            )
        }
    AddMcpServerSheet(state = state, callbacks = callbacks, onDone = onDone)
}

/** The add sheet. It holds no state: [state] comes in, and each change goes out via [callbacks]. */
@Composable
fun AddMcpServerSheet(
    state: AddMcpServerUiState,
    callbacks: AddMcpServerCallbacks,
    onDone: () -> Unit,
) {
    TetherSheet(title = stringResource(R.string.mcp_add_title), onDismiss = onDone) {
        AddMcpServerForm(state = state, callbacks = callbacks, onDone = onDone)
    }
}

/** The form inside the sheet: name, address, headers, then Add or the result after a save. */
@Composable
internal fun AddMcpServerForm(
    state: AddMcpServerUiState,
    callbacks: AddMcpServerCallbacks,
    onDone: () -> Unit,
) {
    val editable = state.addedId == null && !state.adding
    Column(
        modifier = Modifier.padding(horizontal = TetherSpace.space4),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
    ) {
        if (state.problem != null) {
            InlineBanner(
                text = stringResource(state.problem.messageRes()),
                tone = BannerTone.Danger,
                actionLabel = stringResource(R.string.mcp_dismiss),
                onAction = callbacks.onDismissProblem,
            )
        }
        NameAndAddress(state = state, callbacks = callbacks, editable = editable)
        Text(
            text = stringResource(R.string.mcp_add_headers_title),
            style = MaterialTheme.typography.labelLarge,
            color = TetherTheme.colors.text,
        )
        Text(
            text = stringResource(R.string.mcp_add_headers_rule),
            style = MaterialTheme.typography.bodySmall,
            color = TetherTheme.colors.text2,
        )
        state.headers.forEachIndexed { index, draft ->
            HeaderFields(index = index, draft = draft, editable = editable, callbacks = callbacks)
        }
        if (editable && state.canAddHeader) {
            TetherTextButton(
                label = stringResource(R.string.mcp_add_header_add),
                onClick = callbacks.onAddHeader,
                icon = TetherIcons.Plus,
            )
        }
        if (state.addedId == null) {
            PrimaryButton(
                label = stringResource(R.string.mcp_add_server),
                onClick = callbacks.onAdd,
                modifier = Modifier.fillMaxWidth(),
                enabled = state.canAdd,
                loading = state.adding,
            )
        } else {
            AddedResult(state = state, onDone = onDone)
        }
    }
}

/** The name and address fields. Each shows its rule, and turns to an error when the rule breaks. */
@Composable
private fun NameAndAddress(
    state: AddMcpServerUiState,
    callbacks: AddMcpServerCallbacks,
    editable: Boolean,
) {
    OutlinedTextField(
        value = state.name,
        onValueChange = callbacks.onNameChange,
        modifier = Modifier.fillMaxWidth(),
        label = { Text(stringResource(R.string.mcp_add_name_label)) },
        supportingText = { Text(stringResource(R.string.mcp_add_name_rule)) },
        isError = state.name.isNotEmpty() && !state.nameValid,
        enabled = editable,
        singleLine = true,
        textStyle = TetherTheme.mono,
        keyboardOptions =
            KeyboardOptions(
                capitalization = KeyboardCapitalization.None,
                keyboardType = KeyboardType.Ascii,
            ),
    )
    OutlinedTextField(
        value = state.url,
        onValueChange = callbacks.onUrlChange,
        modifier = Modifier.fillMaxWidth(),
        label = { Text(stringResource(R.string.mcp_add_url_label)) },
        supportingText = { Text(stringResource(R.string.mcp_add_url_rule)) },
        isError = state.url.isNotEmpty() && !state.urlValid,
        enabled = editable,
        singleLine = true,
        textStyle = TetherTheme.mono,
        keyboardOptions =
            KeyboardOptions(
                capitalization = KeyboardCapitalization.None,
                keyboardType = KeyboardType.Uri,
            ),
    )
}

/** One header row: its name, its value in a [SecretField], and a remove button. */
@Composable
private fun HeaderFields(
    index: Int,
    draft: HeaderDraft,
    editable: Boolean,
    callbacks: AddMcpServerCallbacks,
) {
    val nameBroken = draft.name.isNotEmpty() && !isValidHeaderName(draft.name)
    val valueBroken = draft.value.isNotEmpty() && !isValidHeaderValue(draft.value)
    val toggle = { callbacks.onToggleReveal(index) }
    Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space2)) {
        OutlinedTextField(
            value = draft.name,
            onValueChange = { callbacks.onHeaderNameChange(index, it) },
            modifier = Modifier.fillMaxWidth(),
            label = { Text(stringResource(R.string.mcp_add_header_name_label)) },
            supportingText = { Text(stringResource(R.string.mcp_add_header_name_rule)) },
            isError = nameBroken,
            enabled = editable,
            singleLine = true,
            textStyle = TetherTheme.mono,
            keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.None),
        )
        SecretField(
            value = draft.value,
            onValueChange = { callbacks.onHeaderValueChange(index, it) },
            label = stringResource(R.string.mcp_add_header_value_label),
            revealed = draft.revealed,
            onToggleReveal = if (editable) toggle else null,
            revealLabel = stringResource(R.string.mcp_add_reveal),
            hideLabel = stringResource(R.string.mcp_add_hide),
            pasteLabel = stringResource(R.string.mcp_add_paste),
            hint = if (valueBroken) null else stringResource(R.string.mcp_add_header_value_rule),
            error = if (valueBroken) stringResource(R.string.mcp_add_header_value_rule) else null,
            enabled = editable,
        )
        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
            TetherIconButton(
                icon = TetherIcons.X,
                contentDescription = stringResource(R.string.mcp_add_header_remove),
                onClick = { callbacks.onRemoveHeader(index) },
                enabled = editable,
            )
        }
    }
}

/** After a save: the saved notice, the test result, and Done. The form stays read-only. */
@Composable
private fun AddedResult(
    state: AddMcpServerUiState,
    onDone: () -> Unit,
) {
    InlineBanner(text = stringResource(R.string.mcp_add_saved), tone = BannerTone.Success)
    when (val result = state.testResult) {
        null ->
            Text(
                text = stringResource(R.string.mcp_testing),
                style = MaterialTheme.typography.bodyMedium,
                color = TetherTheme.colors.text2,
            )
        is McpTestResult.Connected ->
            InlineBanner(text = testPassedText(result.toolCount), tone = BannerTone.Success)
        McpTestResult.Failed ->
            InlineBanner(
                text = stringResource(R.string.mcp_add_saved_test_failed),
                tone = BannerTone.Warning,
            )
    }
    PrimaryButton(
        label = stringResource(R.string.mcp_add_done),
        onClick = onDone,
        modifier = Modifier.fillMaxWidth(),
        enabled = !state.testing,
    )
}
