package app.tether

import android.app.Application
import app.tether.di.AppModule

/** Application class. Starts the Rust core once per process. */
class TetherApp : Application() {
    lateinit var module: AppModule
        private set

    override fun onCreate() {
        super.onCreate()
        module = AppModule()
        module.core.start()
    }
}
