package org.radroots.app.android.security

import android.app.KeyguardManager
import android.os.Build
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt
import androidx.fragment.app.FragmentActivity
import androidx.core.content.ContextCompat

class RadRootsAndroidUserPresenceVerifier(
    private val activity: FragmentActivity,
) {
    fun beginVerification(
        reason: String,
        onSuccess: () -> Unit,
        onFailure: (RadRootsAndroidSecurityError) -> Unit,
    ) {
        if (reason.isBlank()) {
            onFailure(RadRootsAndroidSecurityError.InvalidInput("verification reason must not be blank"))
            return
        }

        val promptInfo = try {
            buildPromptInfo(reason)
        } catch (error: RadRootsAndroidSecurityError) {
            onFailure(error)
            return
        }

        val executor = ContextCompat.getMainExecutor(activity)
        val prompt = BiometricPrompt(
            activity,
            executor,
            object : BiometricPrompt.AuthenticationCallback() {
                override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                    onSuccess()
                }

                override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                    onFailure(mapAuthenticationError(errorCode, errString))
                }

                override fun onAuthenticationFailed() {
                    onFailure(
                        RadRootsAndroidSecurityError.UserPresenceFailure(
                            "device authentication failed",
                        ),
                    )
                }
            },
        )

        activity.runOnUiThread {
            prompt.authenticate(promptInfo)
        }
    }

    private fun buildPromptInfo(reason: String): BiometricPrompt.PromptInfo {
        ensureAuthenticationAvailable()

        val builder = BiometricPrompt.PromptInfo.Builder()
            .setTitle("Rad Roots")
            .setSubtitle("Authenticate to $reason")

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            builder.setAllowedAuthenticators(
                BiometricManager.Authenticators.BIOMETRIC_STRONG or
                    BiometricManager.Authenticators.DEVICE_CREDENTIAL,
            )
        } else if (deviceCredentialAvailable()) {
            builder.setDeviceCredentialAllowed(true)
        } else {
            builder.setNegativeButtonText("Cancel")
        }

        return builder.build()
    }

    private fun ensureAuthenticationAvailable() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            when (
                BiometricManager.from(activity).canAuthenticate(
                    BiometricManager.Authenticators.BIOMETRIC_STRONG or
                        BiometricManager.Authenticators.DEVICE_CREDENTIAL,
                )
            ) {
                BiometricManager.BIOMETRIC_SUCCESS -> return
                BiometricManager.BIOMETRIC_ERROR_NONE_ENROLLED ->
                    throw RadRootsAndroidSecurityError.UserPresenceUnavailable(
                        "no device authentication method is enrolled",
                    )
                BiometricManager.BIOMETRIC_ERROR_NO_HARDWARE,
                BiometricManager.BIOMETRIC_ERROR_HW_UNAVAILABLE ->
                    throw RadRootsAndroidSecurityError.UserPresenceUnavailable(
                        "device authentication is unavailable",
                    )
                else ->
                    throw RadRootsAndroidSecurityError.UserPresenceFailure(
                        "failed to prepare device authentication",
                    )
            }
        }

        val biometricStatus = BiometricManager.from(activity).canAuthenticate()
        if (biometricStatus == BiometricManager.BIOMETRIC_SUCCESS || deviceCredentialAvailable()) {
            return
        }

        throw when (biometricStatus) {
            BiometricManager.BIOMETRIC_ERROR_NONE_ENROLLED ->
                RadRootsAndroidSecurityError.UserPresenceUnavailable(
                    "no biometric or device credential is available",
                )
            BiometricManager.BIOMETRIC_ERROR_NO_HARDWARE,
            BiometricManager.BIOMETRIC_ERROR_HW_UNAVAILABLE ->
                RadRootsAndroidSecurityError.UserPresenceUnavailable(
                    "device authentication is unavailable",
                )
            else ->
                RadRootsAndroidSecurityError.UserPresenceFailure(
                    "failed to prepare device authentication",
                )
        }
    }

    private fun deviceCredentialAvailable(): Boolean {
        val keyguardManager = activity.getSystemService(KeyguardManager::class.java)
        return keyguardManager?.isDeviceSecure == true
    }

    private fun mapAuthenticationError(
        errorCode: Int,
        errString: CharSequence,
    ): RadRootsAndroidSecurityError {
        val message = errString.toString()
        return when (errorCode) {
            BiometricPrompt.ERROR_NEGATIVE_BUTTON,
            BiometricPrompt.ERROR_USER_CANCELED,
            BiometricPrompt.ERROR_CANCELED ->
                RadRootsAndroidSecurityError.UserCancelled(message)
            BiometricPrompt.ERROR_HW_NOT_PRESENT,
            BiometricPrompt.ERROR_HW_UNAVAILABLE,
            BiometricPrompt.ERROR_NO_BIOMETRICS,
            BiometricPrompt.ERROR_NO_DEVICE_CREDENTIAL ->
                RadRootsAndroidSecurityError.UserPresenceUnavailable(message)
            else -> RadRootsAndroidSecurityError.UserPresenceFailure(message)
        }
    }
}
