package app.tether.projectarchive

import app.tether.core.generated.ProjectRuntimeException
import org.junit.Assert.assertEquals
import org.junit.Test

class ArchiveReasonTest {
    @Test
    fun every_known_archive_detail_maps_to_its_reason() {
        val expected =
            mapOf(
                "invalid archive entry" to ArchiveReason.InvalidEntry,
                "archive has too many entries" to ArchiveReason.TooManyEntries,
                "archive is too large" to ArchiveReason.TooLarge,
                "archive compression ratio is unsafe" to ArchiveReason.UnsafeRatio,
                "archive operation cancelled" to ArchiveReason.Cancelled,
                "archive destination is invalid" to ArchiveReason.InvalidDestination,
                "file is not a ZIP archive" to ArchiveReason.NotZip,
                "archive storage is unavailable" to ArchiveReason.StorageUnavailable,
            )
        expected.forEach { (detail, reason) ->
            assertEquals(detail, reason, archiveReasonForDetail(detail))
        }
    }

    @Test
    fun an_unknown_archive_detail_maps_to_the_unknown_reason() {
        assertEquals(ArchiveReason.UnknownDetail, archiveReasonForDetail("something new"))
    }

    @Test
    fun every_core_error_variant_maps_to_a_reason() {
        assertEquals(
            ArchiveReason.InvalidName,
            ProjectRuntimeException.InvalidName().toArchiveReason(),
        )
        assertEquals(
            ArchiveReason.Unexpected,
            ProjectRuntimeException.InvalidDefaults().toArchiveReason(),
        )
        assertEquals(
            ArchiveReason.Unexpected,
            ProjectRuntimeException.BypassNeedsConfirmation().toArchiveReason(),
        )
        assertEquals(
            ArchiveReason.TooLarge,
            ProjectRuntimeException.Archive(detail = "archive is too large").toArchiveReason(),
        )
        assertEquals(ArchiveReason.Cancelled, ProjectRuntimeException.Cancelled().toArchiveReason())
        assertEquals(
            ArchiveReason.ProjectNotFound,
            ProjectRuntimeException.NotFound().toArchiveReason(),
        )
        assertEquals(
            ArchiveReason.Unavailable,
            ProjectRuntimeException.Unavailable().toArchiveReason(),
        )
    }

    @Test
    fun an_archive_error_with_an_unknown_detail_keeps_the_unknown_reason() {
        val error = ProjectRuntimeException.Archive(detail = "new detail")
        assertEquals(ArchiveReason.UnknownDetail, error.toArchiveReason())
    }
}
