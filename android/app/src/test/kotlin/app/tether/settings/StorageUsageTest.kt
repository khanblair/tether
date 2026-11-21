package app.tether.settings

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.io.File

class StorageUsageTest {
    @get:Rule val temporary = TemporaryFolder()

    @Test
    fun a_folder_size_is_the_sum_of_every_file_under_it() {
        val folder = temporary.newFolder("workspaces")
        File(folder, "a.txt").writeBytes(ByteArray(10))
        File(folder.resolve("nested").apply { mkdirs() }, "b.txt").writeBytes(ByteArray(5))

        assertEquals(15L, directorySize(folder))
    }

    @Test
    fun a_missing_path_is_zero_bytes() {
        assertEquals(0L, directorySize(File(temporary.root, "missing")))
    }

    @Test
    fun the_database_size_counts_its_write_ahead_log_and_shared_memory() {
        val database = File(temporary.root, "tether.db").apply { writeBytes(ByteArray(100)) }
        File(temporary.root, "tether.db-wal").writeBytes(ByteArray(20))
        File(temporary.root, "tether.db-shm").writeBytes(ByteArray(3))
        val source = source(databaseFile = database)

        assertEquals(123L, source.usage().database)
    }

    @Test
    fun clearing_the_update_cache_removes_its_contents_and_keeps_the_folder() {
        val cache = temporary.newFolder("updates")
        File(cache, "app.apk").writeBytes(ByteArray(8))
        File(cache.resolve("partial").apply { mkdirs() }, "x").writeBytes(ByteArray(1))
        val source = source(updateCache = cache)

        val cleared = source.clearUpdateCache()

        assertTrue(cleared)
        assertTrue(cache.isDirectory)
        assertEquals(0, cache.listFiles().orEmpty().size)
        assertEquals(0L, source.usage().updateCache)
    }

    @Test
    fun the_app_data_total_leaves_out_the_update_cache() {
        val usage =
            StorageUsage(database = 1L, workspaces = 2L, checkpointBlobs = 4L, updateCache = 100L)

        assertEquals(7L, usage.appData)
        assertFalse(usage.appData == 107L)
    }

    private fun source(
        databaseFile: File = File(temporary.root, "tether.db"),
        updateCache: File = File(temporary.root, "updates"),
    ) = FileStorageSource(
        databaseFile = databaseFile,
        workspaces = File(temporary.root, "workspaces"),
        checkpointBlobs = File(temporary.root, "blobs"),
        updateCache = updateCache,
    )
}
