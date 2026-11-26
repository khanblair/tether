package app.tether.crash

import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Before
import org.junit.Test
import java.io.File

class FileCrashStoreTest {
    private lateinit var dir: File

    @Before
    fun setUp() {
        dir = File(System.getProperty("java.io.tmpdir"), "crash-test-${System.nanoTime()}")
    }

    @After
    fun tearDown() {
        dir.deleteRecursively()
    }

    @Test
    fun a_written_report_is_read_back_and_then_cleared() {
        val store = FileCrashStore(dir)

        store.write("java.lang.Error at now")
        assertEquals("java.lang.Error at now", store.read())

        store.clear()
        assertNull(store.read())
    }

    @Test
    fun a_missing_folder_reads_as_no_report() {
        assertNull(FileCrashStore(dir).read())
    }
}
