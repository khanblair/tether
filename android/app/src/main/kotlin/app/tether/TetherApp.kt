package app.tether

import android.app.Application
import android.os.SystemClock
import android.util.Log
import app.tether.core.generated.recoverInterruptedSessions
import app.tether.crash.CrashRecorder
import app.tether.di.AppModule
import app.tether.di.runQuietly
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
        CrashRecorder.install(this)
        recoverInterruptedTurns()
        module = AppModule(this)
        settings = SettingsDeps(this, module)
        appLock =
            AppLock(
                isEnabled = settings.store::appLockEnabled,
                clock = SystemClock::elapsedRealtime,
            )
        createTetherChannels(this)
        module.core.start()
        module.launchInBackground {
            runQuietly({
                module.linuxSetup.configureAtStart(this@TetherApp, module::configureExecutors)
            }) {
                Log.w("Tether", "Linux setup failed: ${it.javaClass.simpleName}")
            }
        }
        module.updates.restore()
        module.updates.checkIfDue()
    }

    /**
     * Marks turns that were running when the last process died as interrupted. It runs once per
     * process, before any session runtime exists, so a runtime built later never changes the
     * status of a turn that is running now.
     */
    private fun recoverInterruptedTurns() {
        runQuietly({ recoverInterruptedSessions(getDatabasePath("tether.db").absolutePath) }) {
            Log.w("Tether", "recovery failed: ${it.javaClass.simpleName}")
        }
    }
}
