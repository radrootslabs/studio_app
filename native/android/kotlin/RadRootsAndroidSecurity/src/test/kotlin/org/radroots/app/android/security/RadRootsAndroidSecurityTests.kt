package org.radroots.app.android.security

import android.os.Build
import android.security.keystore.KeyProperties
import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class RadRootsAndroidSecurityTests {
    @Test
    fun secureLocalSecretPolicyDefaultsAreStable() {
        val policy = RadRootsAndroidSecretAccessPolicy.SECURE_LOCAL_SECRET

        assertTrue(policy.deviceLocalOnly)
        assertFalse(policy.userPresenceRequired)
        assertTrue(policy.preferStrongBox)
    }

    @Test
    fun nostrRootUsesNoBackupLayout() {
        val baseDir = File("/data/user/0/org.radroots.app.android/no_backup")

        assertEquals(
            File("/data/user/0/org.radroots.app.android/no_backup/RadRoots/app/android/nostr"),
            RadRootsAndroidStoragePaths.nostrRoot(baseDir),
        )
        assertEquals(
            File("/data/user/0/org.radroots.app.android/no_backup/RadRoots/app/android/nostr/accounts.json"),
            RadRootsAndroidStoragePaths.accountsFile(baseDir),
        )
    }

    @Test
    fun secretFileIdIsDeterministic() {
        val first = RadRootsAndroidStoragePaths.secretFileId(
            servicePrefix = "org.radroots.app.nostr",
            namespace = "nostr",
            name = "account-1",
        )
        val second = RadRootsAndroidStoragePaths.secretFileId(
            servicePrefix = "org.radroots.app.nostr",
            namespace = "nostr",
            name = "account-1",
        )

        assertEquals(first, second)
        assertEquals(64, first.length)
    }

    @Test
    fun secretFileNamesCarryNamespacePrefix() {
        val baseDir = File("/data/user/0/org.radroots.app.android/no_backup")
        val path = RadRootsAndroidStoragePaths.secretFile(
            baseDir = baseDir,
            servicePrefix = "org.radroots.app.nostr",
            namespace = "remote-signer",
            name = "client-1",
        )

        assertTrue(path.name.endsWith(".bin"))
        assertTrue(
            path.name.startsWith(
                "${RadRootsAndroidStoragePaths.secretNamespaceId("org.radroots.app.nostr", "remote-signer")}.",
            ),
        )
    }

    @Test
    fun strongBoxIsRequestedOnlyWhenSupported() {
        val policy = RadRootsAndroidSecretAccessPolicy.SECURE_LOCAL_SECRET

        assertTrue(
            shouldRequestStrongBox(
                policy = policy,
                sdkInt = Build.VERSION_CODES.P,
                hasStrongBoxFeature = true,
            ),
        )
        assertFalse(
            shouldRequestStrongBox(
                policy = policy,
                sdkInt = Build.VERSION_CODES.O_MR1,
                hasStrongBoxFeature = true,
            ),
        )
        assertFalse(
            shouldRequestStrongBox(
                policy = policy.copy(preferStrongBox = false),
                sdkInt = Build.VERSION_CODES.P,
                hasStrongBoxFeature = true,
            ),
        )
        assertFalse(
            shouldRequestStrongBox(
                policy = policy,
                sdkInt = Build.VERSION_CODES.P,
                hasStrongBoxFeature = false,
            ),
        )
    }

    @Test
    fun securityLevelMappingPrefersVerifiedPlatformTier() {
        assertEquals(
            RadRootsAndroidKeySecurityLevel.STRONGBOX,
            RadRootsAndroidKeySecurityLevels.fromPlatformValues(
                sdkInt = Build.VERSION_CODES.S,
                securityLevel = KeyProperties.SECURITY_LEVEL_STRONGBOX,
                isInsideSecureHardware = true,
            ),
        )
        assertEquals(
            RadRootsAndroidKeySecurityLevel.TRUSTED_ENVIRONMENT,
            RadRootsAndroidKeySecurityLevels.fromPlatformValues(
                sdkInt = Build.VERSION_CODES.S,
                securityLevel = KeyProperties.SECURITY_LEVEL_TRUSTED_ENVIRONMENT,
                isInsideSecureHardware = true,
            ),
        )
        assertEquals(
            RadRootsAndroidKeySecurityLevel.SOFTWARE_OR_UNKNOWN,
            RadRootsAndroidKeySecurityLevels.fromPlatformValues(
                sdkInt = Build.VERSION_CODES.S,
                securityLevel = KeyProperties.SECURITY_LEVEL_SOFTWARE,
                isInsideSecureHardware = false,
            ),
        )
        assertEquals(
            RadRootsAndroidKeySecurityLevel.TRUSTED_ENVIRONMENT,
            RadRootsAndroidKeySecurityLevels.fromPlatformValues(
                sdkInt = Build.VERSION_CODES.R,
                securityLevel = null,
                isInsideSecureHardware = true,
            ),
        )
    }

    @Test
    fun strongBoxVerificationAcceptsOnlyBestAvailableTier() {
        assertTrue(
            acceptsStrongBoxVerificationResult(
                sdkInt = Build.VERSION_CODES.S,
                securityLevel = RadRootsAndroidKeySecurityLevel.STRONGBOX,
            ),
        )
        assertFalse(
            acceptsStrongBoxVerificationResult(
                sdkInt = Build.VERSION_CODES.S,
                securityLevel = RadRootsAndroidKeySecurityLevel.TRUSTED_ENVIRONMENT,
            ),
        )
        assertTrue(
            acceptsStrongBoxVerificationResult(
                sdkInt = Build.VERSION_CODES.R,
                securityLevel = RadRootsAndroidKeySecurityLevel.TRUSTED_ENVIRONMENT,
            ),
        )
        assertFalse(
            acceptsStrongBoxVerificationResult(
                sdkInt = Build.VERSION_CODES.R,
                securityLevel = RadRootsAndroidKeySecurityLevel.SOFTWARE_OR_UNKNOWN,
            ),
        )
    }
}
