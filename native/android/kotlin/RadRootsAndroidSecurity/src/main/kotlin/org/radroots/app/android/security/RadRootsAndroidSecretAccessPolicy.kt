package org.radroots.app.android.security

data class RadRootsAndroidSecretAccessPolicy(
    val deviceLocalOnly: Boolean,
    val userPresenceRequired: Boolean,
    val preferStrongBox: Boolean,
) {
    companion object {
        val SECURE_LOCAL_SECRET = RadRootsAndroidSecretAccessPolicy(
            deviceLocalOnly = true,
            userPresenceRequired = false,
            preferStrongBox = true,
        )
    }
}
