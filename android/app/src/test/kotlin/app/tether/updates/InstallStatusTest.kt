package app.tether.updates

import android.content.pm.PackageInstaller
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class InstallStatusTest {
    @Test
    fun successHasNoFailure() {
        assertNull(installFailureFor(PackageInstaller.STATUS_SUCCESS))
    }

    @Test
    fun abortedIsCancelled() {
        assertFailure(InstallFailure.Cancelled, PackageInstaller.STATUS_FAILURE_ABORTED)
    }

    @Test
    fun blockedIsBlocked() {
        assertFailure(InstallFailure.Blocked, PackageInstaller.STATUS_FAILURE_BLOCKED)
    }

    @Test
    fun conflictIsConflict() {
        assertFailure(InstallFailure.Conflict, PackageInstaller.STATUS_FAILURE_CONFLICT)
    }

    @Test
    fun incompatibleIsIncompatible() {
        assertFailure(InstallFailure.Incompatible, PackageInstaller.STATUS_FAILURE_INCOMPATIBLE)
    }

    @Test
    fun invalidIsInvalid() {
        assertFailure(InstallFailure.Invalid, PackageInstaller.STATUS_FAILURE_INVALID)
    }

    @Test
    fun storageIsStorage() {
        assertFailure(InstallFailure.Storage, PackageInstaller.STATUS_FAILURE_STORAGE)
    }

    @Test
    fun genericFailureIsOther() {
        assertFailure(InstallFailure.Other, PackageInstaller.STATUS_FAILURE)
    }

    @Test
    fun timeoutIsOther() {
        assertFailure(InstallFailure.Other, PackageInstaller.STATUS_FAILURE_TIMEOUT)
    }

    @Test
    fun pendingUserActionIsOther() {
        assertFailure(InstallFailure.Other, PackageInstaller.STATUS_PENDING_USER_ACTION)
    }

    @Test
    fun unknownStatusIsOther() {
        assertFailure(InstallFailure.Other, 99)
    }

    private fun assertFailure(
        expected: InstallFailure,
        status: Int,
    ) {
        assertEquals(expected, installFailureFor(status))
    }
}
