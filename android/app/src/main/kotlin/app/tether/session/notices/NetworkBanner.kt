package app.tether.session.notices

import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.tooling.preview.Preview
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.tokens.TetherSpace

/**
 * The notice under the status line while the network is down. It has no buttons. The turn waits
 * and continues on its own, and the polite live region of [InlineBanner] announces it.
 */
@Composable
fun NetworkBanner(modifier: Modifier = Modifier) {
    InlineBanner(
        text = stringResource(R.string.notices_network_offline),
        tone = BannerTone.Info,
        modifier = modifier.padding(horizontal = TetherSpace.space4),
    )
}

@Preview(name = "Network paused, light")
@Composable
internal fun NetworkBannerPreview() {
    TetherTheme(mode = ThemeMode.Light) { NetworkBanner() }
}

@Preview(name = "Network paused, dark")
@Composable
internal fun NetworkBannerDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) { NetworkBanner() }
}
