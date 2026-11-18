package app.tether.e2e

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.tether.core.CoreClient
import app.tether.core.generated.CoreEventView
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith

/** Proves libtether_core.so loads on the device and an event crosses from Rust to Kotlin. */
@RunWith(AndroidJUnit4::class)
class NativeLibraryTest {
    @Test
    fun core_version_equals_app_version() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val appVersion = context.packageManager.getPackageInfo(context.packageName, 0).versionName

        assertEquals(appVersion, CoreClient().coreVersion())
    }

    @Test
    fun test_event_reaches_kotlin() =
        runBlocking {
            val client = CoreClient()
            client.start()
            client.emitTestEvent("ping")

            val event = client.events.first { it is CoreEventView.Test }

            assertEquals(CoreEventView.Test(message = "ping"), event)
        }
}
