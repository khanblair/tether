package app.tether

import android.app.Application
import android.os.SystemClock
import app.tether.di.AppModule
import app.tether.settings.AppLock
import app.tether.settings.SettingsDeps
import app.tether.settings.createTetherChannels

/** Application class. Starts the Rust core once per process. */
class TetherApp : Application() {
    lateinit var module: AppModule
        private set

    /** The settings screens' data sources, created once for the process. */
    lateinit var settings: SettingsDeps
        private set

    /** Decides when the lock screen shows. It lives here, so a rotation does not lock the app. */
    lateinit var appLock: AppLock
        private set

    override fun onCreate() {
        super.onCreate()
        module = AppModule(this)
        settings = SettingsDeps(this, module)
        appLock =
            AppLock(
                isEnabled = settings.store::appLockEnabled,
                clock = SystemClock::elapsedRealtime,
            )
        createTetherChannels(this)
        module.core.start()
        module.updates.restore()
        module.updates.checkIfDue()
    }
}
