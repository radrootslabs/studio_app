package org.radroots.app.android.security

import android.content.Context

object RadRootsAndroidSecurityBridge {
    const val STATUS_SUCCESS = 0
    const val STATUS_NOT_FOUND = 1
    const val STATUS_INVALID_INPUT = 2
    const val STATUS_ERROR = 3

    @Volatile
    private var applicationContext: Context? = null

    @Volatile
    private var lastErrorMessage: String? = null

    @JvmStatic
    fun initialize(context: Context) {
        applicationContext = context.applicationContext
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
    fun resolveNostrStorageRoot(): String? {
        return try {
            val path = secretStore().resolveNostrStorageRoot().absolutePath
            clearError()
            path
        } catch (cause: Throwable) {
            captureError(cause)
            null
        }
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
