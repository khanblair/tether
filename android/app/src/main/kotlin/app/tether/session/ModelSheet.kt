package app.tether.session

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import app.tether.R
import app.tether.core.generated.ProviderView
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.BadgeTone
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.PrimaryButton
import app.tether.design.components.SectionTitle
import app.tether.design.components.StatusBadge
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherSheet
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.home.formatTokens

/**
 * The model picker. Models are grouped by provider, the session's model is marked, and a turn in
 * progress disables every row with the reason. A model that reasons also shows the thinking
 * section below the list.
 */
@Composable
fun ModelSheet(
    state: ModelSheetState,
    running: Boolean,
    onChoose: (ProviderView, ModelChoice) -> Unit,
    onOpenProviders: () -> Unit,
    onDismiss: () -> Unit,
    onChooseEffort: (ThinkingEffort?) -> Unit = {},
) {
    TetherSheet(title = stringResource(R.string.session_model_title), onDismiss = onDismiss) {
        if (running) {
            InlineBanner(
                text = stringResource(R.string.session_busy_reason),
                tone = BannerTone.Info,
                modifier = Modifier.padding(horizontal = TetherSpace.space4),
            )
        }
        if (state.failure != null) {
            InlineBanner(
                text = stringResource(R.string.session_error_settings),
                tone = BannerTone.Danger,
                modifier = Modifier.padding(horizontal = TetherSpace.space4),
            )
        }
        when {
            state.loading -> LoadingLine()
            state.groups.isEmpty() -> NoModels(onOpenProviders)
            else -> ModelBody(state, running, onChoose, onChooseEffort)
        }
    }
}

/** The model list and, for a reasoning model, the thinking section, in one scrolling column. */
@Composable
private fun ModelBody(
    state: ModelSheetState,
    running: Boolean,
    onChoose: (ProviderView, ModelChoice) -> Unit,
    onChooseEffort: (ThinkingEffort?) -> Unit,
) {
    val enabled = !running && !state.saving
    Column(
        modifier =
            Modifier
                .verticalScroll(rememberScrollState())
                .padding(bottom = TetherSpace.space2),
    ) {
        ModelGroups(state, enabled = enabled, onChoose = onChoose)
        if (state.showsEffort) {
            EffortSection(
                effort = state.effort,
                enabled = enabled,
                onChoose = onChooseEffort,
                modifier = Modifier.padding(top = TetherSpace.space3),
            )
        }
    }
}

@Composable
private fun LoadingLine() {
    Text(
        text = stringResource(R.string.session_model_loading),
        style = MaterialTheme.typography.bodyMedium,
        color = TetherTheme.colors.text2,
        modifier = Modifier.padding(TetherSpace.space4),
    )
}

@Composable
private fun NoModels(onOpenProviders: () -> Unit) {
    Column(
        modifier = Modifier.padding(TetherSpace.space4),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
    ) {
        Text(
            text = stringResource(R.string.session_model_none),
            style = MaterialTheme.typography.bodyMedium,
            color = TetherTheme.colors.text2,
        )
        PrimaryButton(
            label = stringResource(R.string.project_home_open_providers),
            onClick = onOpenProviders,
            modifier = Modifier.fillMaxWidth(),
        )
    }
}

@Composable
private fun ModelGroups(
    state: ModelSheetState,
    enabled: Boolean,
    onChoose: (ProviderView, ModelChoice) -> Unit,
) {
    Column {
        state.groups.forEach { group ->
            SectionTitle(text = stringResource(group.provider.nameRes()))
            group.models.forEach { model ->
                val selected =
                    state.current?.provider == group.provider && state.current.modelId == model.id
                ModelRow(
                    model = model,
                    selected = selected,
                    enabled = enabled && model.canRunAgents,
                    onClick = { onChoose(group.provider, model) },
                )
            }
        }
    }
}

/**
 * One model. The selected row shows a check icon and says so to TalkBack. A model that cannot run
 * agent sessions is shown disabled, with the reason under its name.
 */
@Composable
private fun ModelRow(
    model: ModelChoice,
    selected: Boolean,
    enabled: Boolean,
    onClick: () -> Unit,
) {
    val meta = modelMeta(model)
    val nameColor = if (enabled) TetherTheme.colors.text else TetherTheme.colors.textDisabled
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = TetherSize.touchMin)
                .clickable(enabled = enabled, role = Role.Button, onClick = onClick)
                .semantics { this.selected = selected }
                .padding(horizontal = TetherSpace.space4, vertical = TetherSpace.space3),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = model.displayName,
                style = MaterialTheme.typography.bodyLarge,
                color = nameColor,
            )
            if (!model.canRunAgents) {
                Text(
                    text = stringResource(R.string.session_model_no_tools),
                    style = MaterialTheme.typography.bodySmall,
                    color = TetherTheme.colors.text2,
                )
            }
            if (meta.isNotEmpty()) {
                Text(
                    text = meta,
                    style = MaterialTheme.typography.bodySmall,
                    color = TetherTheme.colors.text2,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            CapabilityBadges(model)
        }
        if (selected) {
            Spacer(Modifier.width(TetherSpace.space3))
            TetherIcon(
                icon = TetherIcons.CircleCheck,
                contentDescription = stringResource(R.string.session_selected),
                tint = TetherTheme.colors.accent,
            )
        }
    }
}

@Composable
private fun CapabilityBadges(model: ModelChoice) {
    Row(
        modifier = Modifier.padding(top = TetherSpace.space1),
        horizontalArrangement = Arrangement.spacedBy(TetherSpace.space1),
    ) {
        if (model.tools == true) {
            StatusBadge(stringResource(R.string.session_badge_tools), BadgeTone.Neutral)
        }
        if (model.images == true) {
            StatusBadge(stringResource(R.string.session_badge_vision), BadgeTone.Neutral)
        }
        if (model.reasoning == true) {
            StatusBadge(stringResource(R.string.session_badge_reasoning), BadgeTone.Neutral)
        }
    }
}

/** Context window and price as plain text, joined with a middle dot. Unknown parts are left out. */
@Composable
private fun modelMeta(model: ModelChoice): String {
    val context =
        model.contextWindow?.let {
            stringResource(R.string.session_model_context, formatTokens(it))
        }
    val input = model.inputMicrosPerMillion
    val output = model.outputMicrosPerMillion
    val price =
        if (input != null && output != null) {
            stringResource(
                R.string.session_model_price,
                formatPricePerMillion(input),
                formatPricePerMillion(output),
            )
        } else {
            null
        }
    return listOfNotNull(context, price).joinToString(separator = " · ")
}

/** The name string of a provider, as the provider screens show it. */
internal fun ProviderView.nameRes(): Int =
    when (this) {
        ProviderView.ANTHROPIC -> R.string.providers_name_anthropic
        ProviderView.OPEN_AI -> R.string.providers_name_openai
        ProviderView.GEMINI -> R.string.providers_name_gemini
        ProviderView.DEEP_SEEK -> R.string.providers_name_deepseek
        ProviderView.OPEN_ROUTER -> R.string.providers_name_openrouter
    }
