package app.tether.providers

import android.text.format.DateUtils
import androidx.annotation.PluralsRes
import androidx.annotation.StringRes
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Scaffold
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.core.generated.ConnectionStatusView
import app.tether.core.generated.ProviderView
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.ProviderRow
import app.tether.design.components.ProviderRowStatus
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.shell.BackButton

/** A subtitle or banner text: a plain string, or a plural with its quantity and format values. */
internal sealed interface ProviderText {
    data class Plain(
        @StringRes val resId: Int,
    ) : ProviderText

    data class Plural(
        @PluralsRes val resId: Int,
        val count: Int,
        val args: List<Any>,
    ) : ProviderText
}

/** One provider row as the list draws it. Pure data, so a JUnit test can check it. */
internal data class ProviderRowModel(
    val provider: ProviderView,
    @StringRes val nameRes: Int,
    val subtitle: ProviderText,
    val status: ProviderRowStatus,
    @StringRes val statusDescription: Int,
)

/**
 * Maps one provider's saved state to its row. [relativeTime] is the formatted test time, such as
 * "2 minutes ago", or null when the provider was never tested. The caller formats it.
 */
internal fun providerRowFor(
    summary: ProviderSummary,
    relativeTime: String?,
): ProviderRowModel {
    val (subtitle, status, description) = rowParts(summary, relativeTime)
    return ProviderRowModel(
        provider = summary.provider,
        nameRes = providerNameRes(summary.provider),
        subtitle = subtitle,
        status = status,
        statusDescription = description,
    )
}

private fun rowParts(
    summary: ProviderSummary,
    relativeTime: String?,
): Triple<ProviderText, ProviderRowStatus, Int> =
    if (!summary.hasKey) {
        Triple(
            ProviderText.Plain(R.string.providers_subtitle_no_key),
            ProviderRowStatus.Unset,
            R.string.providers_status_no_key,
        )
    } else {
        when (summary.status) {
            ConnectionStatusView.OK ->
                Triple(
                    okSubtitle(summary, relativeTime),
                    ProviderRowStatus.Ok,
                    R.string.providers_status_connected,
                )
            ConnectionStatusView.BAD_KEY ->
                Triple(
                    ProviderText.Plain(R.string.providers_subtitle_bad_key),
                    ProviderRowStatus.BadKey,
                    R.string.providers_status_key_rejected,
                )
            ConnectionStatusView.NETWORK_ERROR ->
                Triple(
                    ProviderText.Plain(R.string.providers_subtitle_network),
                    ProviderRowStatus.NetworkError,
                    R.string.providers_status_network_error,
                )
            ConnectionStatusView.BLOCKED ->
                Triple(
                    ProviderText.Plain(R.string.providers_subtitle_blocked),
                    ProviderRowStatus.Blocked,
                    R.string.providers_status_blocked,
                )
            ConnectionStatusView.UNSET, ConnectionStatusView.TESTING ->
                Triple(
                    ProviderText.Plain(R.string.providers_subtitle_key_not_tested),
                    ProviderRowStatus.Unset,
                    R.string.providers_status_not_tested,
                )
        }
    }

private fun okSubtitle(
    summary: ProviderSummary,
    relativeTime: String?,
): ProviderText {
    val count = summary.modelCount
    return if (summary.testedAtMs != null && relativeTime != null) {
        ProviderText.Plural(R.plurals.providers_subtitle_ok, count, listOf(count, relativeTime))
    } else {
        ProviderText.Plural(R.plurals.providers_models, count, listOf(count))
    }
}

/** The provider's display name. Shared with the detail screen. */
@StringRes
internal fun providerNameRes(provider: ProviderView): Int =
    when (provider) {
        ProviderView.ANTHROPIC -> R.string.providers_name_anthropic
        ProviderView.OPEN_AI -> R.string.providers_name_openai
        ProviderView.GEMINI -> R.string.providers_name_gemini
        ProviderView.DEEP_SEEK -> R.string.providers_name_deepseek
        ProviderView.OPEN_ROUTER -> R.string.providers_name_openrouter
    }

/** Shows the saved provider list and reads it again each time the screen resumes. */
@Composable
fun ProvidersScreen(
    viewModel: ProvidersViewModel,
    onOpen: (ProviderView) -> Unit,
    onBack: () -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    val lifecycleOwner = LocalLifecycleOwner.current
    DisposableEffect(lifecycleOwner, viewModel) {
        val observer =
            LifecycleEventObserver { _, event ->
                if (event == Lifecycle.Event.ON_RESUME) viewModel.refresh()
            }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
    }
    ProvidersContent(state = state, onOpen = onOpen, onBack = onBack)
}

/** The stateless list. [ProvidersScreen] supplies the state. */
@Composable
fun ProvidersContent(
    state: ProvidersUiState,
    onOpen: (ProviderView) -> Unit,
    onBack: () -> Unit,
) {
    val now = System.currentTimeMillis()
    val rows = state.items.map { providerRowFor(it, relativeTime(it.testedAtMs, now)) }
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = stringResource(R.string.providers_title),
                navigationIcon = { BackButton(onBack) },
            )
        },
    ) { padding ->
        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .verticalScroll(rememberScrollState())
                    .padding(TetherSpace.space4),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Column(
                modifier = Modifier.widthIn(max = TetherSize.transcriptMax).fillMaxWidth(),
                verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
            ) {
                if (state.failed) {
                    InlineBanner(
                        text = stringResource(R.string.providers_load_failed),
                        tone = BannerTone.Danger,
                    )
                }
                ProviderList(rows = rows, onOpen = onOpen)
            }
        }
    }
}

@Composable
private fun ProviderList(
    rows: List<ProviderRowModel>,
    onOpen: (ProviderView) -> Unit,
) {
    val shape = RoundedCornerShape(TetherRadius.md)
    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .clip(shape)
                .background(TetherTheme.colors.surface)
                .border(TetherBorder.hairline, TetherTheme.colors.line, shape),
    ) {
        rows.forEachIndexed { index, row ->
            if (index > 0) {
                HorizontalDivider(
                    color = TetherTheme.colors.line,
                    thickness = TetherBorder.hairline,
                )
            }
            ProviderRow(
                name = stringResource(row.nameRes),
                subtitle = row.subtitle.resolve(),
                status = row.status,
                statusDescription = stringResource(row.statusDescription),
                onClick = { onOpen(row.provider) },
            )
        }
    }
}

/** The text for this subtitle or banner, with the plural rule applied. */
@Suppress("SpreadOperator")
@Composable
internal fun ProviderText.resolve(): String =
    when (this) {
        is ProviderText.Plain -> stringResource(resId)
        is ProviderText.Plural -> pluralStringResource(resId, count, *args.toTypedArray())
    }

private fun relativeTime(
    testedAtMs: Long?,
    now: Long,
): String? =
    testedAtMs?.let {
        DateUtils.getRelativeTimeSpanString(it, now, DateUtils.MINUTE_IN_MILLIS).toString()
    }
