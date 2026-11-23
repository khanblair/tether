package app.tether.projectarchive

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ArchiveRulesTest {
    @Test
    fun the_import_limit_is_200_mib() {
        assertEquals(200L * 1024 * 1024, ARCHIVE_IMPORT_LIMIT_BYTES)
    }

    @Test
    fun only_sizes_over_the_limit_are_refused_and_unknown_sizes_pass() {
        assertFalse(isOverImportLimit(null))
        assertFalse(isOverImportLimit(ARCHIVE_IMPORT_LIMIT_BYTES))
        assertTrue(isOverImportLimit(ARCHIVE_IMPORT_LIMIT_BYTES + 1))
    }

    @Test
    fun sizes_under_one_megabyte_round_up_to_whole_kilobytes() {
        assertEquals(ArchiveSize("1", ArchiveSizeUnit.Kilobytes), formatArchiveSize(0))
        assertEquals(ArchiveSize("1", ArchiveSizeUnit.Kilobytes), formatArchiveSize(1))
        assertEquals(ArchiveSize("512", ArchiveSizeUnit.Kilobytes), formatArchiveSize(512L * 1024))
        assertEquals(ArchiveSize("2", ArchiveSizeUnit.Kilobytes), formatArchiveSize(1025))
    }

    @Test
    fun sizes_from_one_megabyte_up_show_one_decimal() {
        assertEquals(ArchiveSize("1.0", ArchiveSizeUnit.Megabytes), formatArchiveSize(1024L * 1024))
        assertEquals(ArchiveSize("4.2", ArchiveSizeUnit.Megabytes), formatArchiveSize(4_404_019))
        assertEquals(
            ArchiveSize("200.0", ArchiveSizeUnit.Megabytes),
            formatArchiveSize(ARCHIVE_IMPORT_LIMIT_BYTES),
        )
    }

    @Test
    fun a_negative_size_formats_as_one_kilobyte() {
        assertEquals(ArchiveSize("1", ArchiveSizeUnit.Kilobytes), formatArchiveSize(-5))
    }

    @Test
    fun the_import_name_drops_only_the_last_extension() {
        assertEquals("api-server", importNameFromFile("api-server.zip"))
        assertEquals("a.b", importNameFromFile("a.b.zip"))
        assertEquals("archive", importNameFromFile("archive"))
        assertEquals("", importNameFromFile(null))
    }

    @Test
    fun the_export_file_name_is_the_project_name_with_zip_and_no_slashes() {
        assertEquals("acme-api.zip", exportFileName("acme-api"))
        assertEquals("acme-api.zip", exportFileName("acme/api"))
        assertEquals("a-b-c-d-e-f-g-h-i.zip", exportFileName("a\\b:c*d?e\"f<g>h|i"))
        assertEquals("project.zip", exportFileName("   "))
    }
}
