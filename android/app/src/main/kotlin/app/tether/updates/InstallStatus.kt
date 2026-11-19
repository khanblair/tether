package app.tether.updates

import android.content.pm.PackageInstaller

/**
 * Maps a `PackageInstaller.STATUS_*` code to a failure. Returns null for `STATUS_SUCCESS`.
 *
 * `STATUS_PENDING_USER_ACTION` is not a failure. The caller must handle it before calling this
 * function. If it is passed in anyway, this returns [InstallFailure.Other].
 */
fun installFailureFor(status: Int): InstallFailure? =
    when (status) {
        PackageInstaller.STATUS_SUCCESS -> null
        PackageInstaller.STATUS_FAILURE_ABORTED -> InstallFailure.Cancelled
        PackageInstaller.STATUS_FAILURE_BLOCKED -> InstallFailure.Blocked
        PackageInstaller.STATUS_FAILURE_CONFLICT -> InstallFailure.Conflict
        PackageInstaller.STATUS_FAILURE_INCOMPATIBLE -> InstallFailure.Incompatible
        PackageInstaller.STATUS_FAILURE_INVALID -> InstallFailure.Invalid
        PackageInstaller.STATUS_FAILURE_STORAGE -> InstallFailure.Storage
        else -> InstallFailure.Other
    }
