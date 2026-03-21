package org.radroots.app.android.security

import android.os.Build
import android.security.keystore.KeyInfo
import android.security.keystore.KeyProperties

internal enum class RadRootsAndroidKeySecurityLevel {
    STRONGBOX,
    TRUSTED_ENVIRONMENT,
    SOFTWARE_OR_UNKNOWN,
}

internal object RadRootsAndroidKeySecurityLevels {
    fun fromKeyInfo(keyInfo: KeyInfo): RadRootsAndroidKeySecurityLevel {
        return fromPlatformValues(
            sdkInt = Build.VERSION.SDK_INT,
            securityLevel = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                keyInfo.securityLevel
            } else {
                null
            },
            isInsideSecureHardware = isInsideSecureHardwareFallback(keyInfo),
        )
    }

    fun fromPlatformValues(
        sdkInt: Int,
        securityLevel: Int?,
        isInsideSecureHardware: Boolean,
    ): RadRootsAndroidKeySecurityLevel {
        if (sdkInt >= Build.VERSION_CODES.S && securityLevel != null) {
            return when (securityLevel) {
                KeyProperties.SECURITY_LEVEL_STRONGBOX -> RadRootsAndroidKeySecurityLevel.STRONGBOX
                KeyProperties.SECURITY_LEVEL_TRUSTED_ENVIRONMENT,
                KeyProperties.SECURITY_LEVEL_UNKNOWN_SECURE,
                -> RadRootsAndroidKeySecurityLevel.TRUSTED_ENVIRONMENT
                else -> RadRootsAndroidKeySecurityLevel.SOFTWARE_OR_UNKNOWN
            }
        }

        return if (isInsideSecureHardware) {
            RadRootsAndroidKeySecurityLevel.TRUSTED_ENVIRONMENT
        } else {
            RadRootsAndroidKeySecurityLevel.SOFTWARE_OR_UNKNOWN
        }
    }

    @Suppress("DEPRECATION")
    private fun isInsideSecureHardwareFallback(keyInfo: KeyInfo): Boolean {
        return keyInfo.isInsideSecureHardware
    }
}

internal fun shouldRequestStrongBox(
    policy: RadRootsAndroidSecretAccessPolicy,
    sdkInt: Int,
    hasStrongBoxFeature: Boolean,
): Boolean {
    return policy.preferStrongBox &&
        sdkInt >= Build.VERSION_CODES.P &&
        hasStrongBoxFeature
}

internal fun acceptsStrongBoxVerificationResult(
    sdkInt: Int,
    securityLevel: RadRootsAndroidKeySecurityLevel,
): Boolean {
    return when (securityLevel) {
        RadRootsAndroidKeySecurityLevel.STRONGBOX -> true
        RadRootsAndroidKeySecurityLevel.TRUSTED_ENVIRONMENT -> sdkInt < Build.VERSION_CODES.S
        RadRootsAndroidKeySecurityLevel.SOFTWARE_OR_UNKNOWN -> false
    }
}
