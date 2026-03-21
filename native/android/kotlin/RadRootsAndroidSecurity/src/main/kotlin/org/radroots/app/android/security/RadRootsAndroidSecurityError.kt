package org.radroots.app.android.security

sealed class RadRootsAndroidSecurityError(
    message: String,
    cause: Throwable? = null,
) : Exception(message, cause) {
    class InvalidInput(message: String) : RadRootsAndroidSecurityError(message)

    class NotFound(message: String) : RadRootsAndroidSecurityError(message)

    class KeystoreFailure(message: String, cause: Throwable? = null) :
        RadRootsAndroidSecurityError(message, cause)

    class StorageFailure(message: String, cause: Throwable? = null) :
        RadRootsAndroidSecurityError(message, cause)
}
