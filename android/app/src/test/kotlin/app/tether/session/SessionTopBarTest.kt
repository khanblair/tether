package app.tether.session

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class SessionTopBarTest {
    private val format = "%1\$s · %2\$s"

    @Test
    fun subtitleShowsProjectAndBranch() {
        assertEquals("acme-api · main", sessionSubtitle("acme-api", "main", format))
    }

    @Test
    fun subtitleShowsProjectAloneWhenBranchIsBlank() {
        assertEquals("acme-api", sessionSubtitle("acme-api", "", format))
    }

    @Test
    fun subtitleIsNullUntilTheProjectLoads() {
        assertNull(sessionSubtitle(null, "main", format))
    }
}
