package app.tether.linux

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class LinuxSetupTest {
    @Test
    fun empty_resource_values_become_null() {
        assertNull(resourceValueOrNull(""))
        assertNull(resourceValueOrNull("  "))
        assertEquals(
            "https://example.test/rootfs.json",
            resourceValueOrNull(" https://example.test/rootfs.json "),
        )
    }

    @Test
    fun executor_paths_use_the_current_native_library_and_files_directories() {
        val paths =
            executorPaths(
                "/data/app/lib",
                "/data/user/0/app/files",
                "pack",
                "rootfs",
            )

        assertEquals("/data/user/0/app/files/runtimes", paths.packsRoot)
        assertEquals("/data/user/0/app/files/linux", paths.linuxRoot)
        assertEquals("/data/app/lib/libproot.so", paths.prootPath)
        assertEquals("/data/app/lib/libproot-loader.so", paths.prootLoaderPath)
        assertEquals("pack", paths.packVerifyingKeyHex)
        assertEquals("rootfs", paths.rootfsVerifyingKeyHex)
    }

    @Test
    fun missing_proot_skips_configuration() {
        val paths = executorPaths("/native", "/files", null, null)
        var called = false

        val configured =
            configureLinuxIfAvailable(
                paths = paths,
                fileExists = { it.endsWith("libproot-loader.so") },
                configureExecutors = { called = true },
            )

        assertFalse(configured)
        assertFalse(called)
    }

    @Test
    fun present_proot_and_loader_configure_the_core() {
        val paths = executorPaths("/native", "/files", null, null)
        var configuredPaths = false

        val configured =
            configureLinuxIfAvailable(
                paths = paths,
                fileExists = { true },
                configureExecutors = { configuredPaths = it == paths },
            )

        assertTrue(configured)
        assertTrue(configuredPaths)
    }
}
