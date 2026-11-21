package app.tether.session

import app.tether.providers.FakeProviderGateway

/** The dependencies a sheet view model gets in a test, with a count of the reopen calls. */
class SheetFixture(
    val settings: FakeSessionSettingsGateway = FakeSessionSettingsGateway(),
    val providers: FakeProviderGateway = FakeProviderGateway(),
    val rewinder: FakeSessionRewinder = FakeSessionRewinder(),
) {
    var running = false
    var reopenCount = 0

    val deps: SessionSheetDeps
        get() =
            SessionSheetDeps(
                sessionId = "session-1",
                settings = settings,
                providers = providers,
                rewinder = rewinder,
                isRunning = { running },
                reopen = { reopenCount++ },
            )
}
