package app.tether.crash

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class CrashViewModelTest {
    @Test
    fun the_saved_report_is_shown() {
        val model = CrashViewModel(FakeCrashStore("java.lang.Error at now"))

        assertEquals("java.lang.Error at now", model.report.value)
    }

    @Test
    fun no_saved_report_means_nothing_to_show() {
        val model = CrashViewModel(FakeCrashStore(null))

        assertNull(model.report.value)
    }

    @Test
    fun clear_removes_the_report_from_the_store_and_the_screen() {
        val store = FakeCrashStore("java.lang.Error at now")
        val model = CrashViewModel(store)

        model.clear()

        assertNull(model.report.value)
        assertTrue(store.cleared)
    }

    private class FakeCrashStore(
        private val report: String?,
    ) : CrashStore {
        var cleared = false

        override fun read(): String? = report

        override fun clear() {
            cleared = true
        }
    }
}
