package org.radroots.app.android.security

import android.content.Context
import androidx.fragment.app.FragmentActivity

object RadRootsAndroidSecurityBridge {
    const val STATUS_SUCCESS = 0
    const val STATUS_NOT_FOUND = 1
    const val STATUS_INVALID_INPUT = 2
    const val STATUS_ERROR = 3

    const val USER_PRESENCE_RESULT_NONE = 0
    const val USER_PRESENCE_RESULT_SUCCESS = 1
    const val USER_PRESENCE_RESULT_ERROR = 2

    @Volatile
    private var applicationContext: Context? = null

    @Volatile
    private var currentActivity: FragmentActivity? = null

    @Volatile
    private var lastErrorMessage: String? = null

    @Volatile
    private var userPresenceVerificationPending: Boolean = false

    @Volatile
    private var userPresenceVerificationResult: Int = USER_PRESENCE_RESULT_NONE

    @JvmStatic
    fun initialize(context: Context) {
        applicationContext = context.applicationContext
        currentActivity = context as? FragmentActivity
        clearError()
    }

    @JvmStatic
    fun putSecret(
        servicePrefix: String,
        namespace: String,
        name: String,
        value: ByteArray,
        deviceLocalOnly: Boolean,
        userPresenceRequired: Boolean,
        preferStrongBox: Boolean,
    ): Int {
        return try {
            secretStore().putSecret(
                servicePrefix = servicePrefix,
                namespace = namespace,
                name = name,
                value = value,
                policy = RadRootsAndroidSecretAccessPolicy(
                    deviceLocalOnly = deviceLocalOnly,
                    userPresenceRequired = userPresenceRequired,
                    preferStrongBox = preferStrongBox,
                ),
            )
            clearError()
            STATUS_SUCCESS
        } catch (cause: Throwable) {
            captureError(cause)
        }
    }

    @JvmStatic
    fun getSecret(
        servicePrefix: String,
        namespace: String,
        name: String,
    ): ByteArray? {
        return try {
            val secret = secretStore().getSecret(servicePrefix, namespace, name)
            clearError()
            secret
        } catch (cause: Throwable) {
            captureError(cause)
            null
        }
    }

    @JvmStatic
    fun deleteSecret(
        servicePrefix: String,
        namespace: String,
        name: String,
    ): Int {
        return try {
            secretStore().deleteSecret(servicePrefix, namespace, name)
            clearError()
            STATUS_SUCCESS
        } catch (cause: Throwable) {
            captureError(cause)
        }
    }

    @JvmStatic
    fun deleteSecretNamespace(
        servicePrefix: String,
        namespace: String,
    ): Int {
        return try {
            secretStore().deleteNamespace(servicePrefix, namespace)
            clearError()
            STATUS_SUCCESS
        } catch (cause: Throwable) {
            captureError(cause)
        }
    }

    @JvmStatic
    fun resolveRadrootsBaseRoot(): String? {
        return try {
            val path = secretStore().resolveRadrootsBaseRoot().absolutePath
            clearError()
            path
        } catch (cause: Throwable) {
            captureError(cause)
            null
        }
    }

    @JvmStatic
    fun beginUserPresenceVerification(reason: String): Int {
        return try {
            if (reason.isBlank()) {
                throw RadRootsAndroidSecurityError.InvalidInput("verification reason must not be blank")
            }
            if (userPresenceVerificationPending) {
                throw RadRootsAndroidSecurityError.InvalidInput("device authentication is already in progress")
            }
            val activity = currentActivity
                ?: throw RadRootsAndroidSecurityError.InvalidInput("android security bridge has no active activity")

            clearError()
            userPresenceVerificationPending = true
            userPresenceVerificationResult = USER_PRESENCE_RESULT_NONE

            RadRootsAndroidUserPresenceVerifier(activity).beginVerification(
                reason = reason,
                onSuccess = {
                    clearError()
                    userPresenceVerificationPending = false
                    userPresenceVerificationResult = USER_PRESENCE_RESULT_SUCCESS
                },
                onFailure = { cause ->
                    lastErrorMessage = cause.message ?: cause.toString()
                    userPresenceVerificationPending = false
                    userPresenceVerificationResult = USER_PRESENCE_RESULT_ERROR
                },
            )

            STATUS_SUCCESS
        } catch (cause: Throwable) {
            userPresenceVerificationPending = false
            userPresenceVerificationResult = USER_PRESENCE_RESULT_NONE
            captureError(cause)
        }
    }

    @JvmStatic
    fun isUserPresenceVerificationPending(): Boolean = userPresenceVerificationPending

    @JvmStatic
    fun takeUserPresenceVerificationResult(): Int {
        val result = userPresenceVerificationResult
        userPresenceVerificationResult = USER_PRESENCE_RESULT_NONE
        return result
    }

    @JvmStatic
    fun takeLastErrorMessage(): String? {
        val message = lastErrorMessage
        lastErrorMessage = null
        return message
    }

    private fun secretStore(): RadRootsAndroidKeystoreSecretStore {
        val context = applicationContext
            ?: throw RadRootsAndroidSecurityError.InvalidInput("android security bridge is not initialized")
        return RadRootsAndroidKeystoreSecretStore(context)
    }

    private fun captureError(cause: Throwable): Int {
        lastErrorMessage = cause.message ?: cause.toString()
        return when (cause) {
            is RadRootsAndroidSecurityError.NotFound -> STATUS_NOT_FOUND
            is RadRootsAndroidSecurityError.InvalidInput -> STATUS_INVALID_INPUT
            else -> STATUS_ERROR
        }
    }

    private fun clearError() {
        lastErrorMessage = null
    }
}
