package org.radroots.app.android.security

import android.content.Context
import java.io.File
import java.security.MessageDigest

object RadRootsAndroidStoragePaths {
    private const val rootDirName = "RadRoots"
    private const val configDirName = "config"
    private const val dataDirName = "data"
    private const val secretsRootDirName = "secrets"
    private const val appsDirName = "apps"
    private const val appRuntimeDirName = "app"
    private const val nostrDirName = "nostr"
    private const val accountsFileName = "accounts.json"

    fun baseRoot(context: Context): File = baseRoot(context.noBackupFilesDir)

    fun baseRoot(baseDir: File): File = File(baseDir, rootDirName)

    fun appDataRoot(context: Context): File = appDataRoot(context.noBackupFilesDir)

    fun appDataRoot(baseDir: File): File =
        File(
            File(
                File(baseRoot(baseDir), dataDirName),
                appsDirName,
            ),
            appRuntimeDirName,
        )

    fun nostrRoot(context: Context): File = nostrRoot(context.noBackupFilesDir)

    fun nostrRoot(baseDir: File): File = File(appDataRoot(baseDir), nostrDirName)

    fun secretsDir(context: Context): File = secretsDir(context.noBackupFilesDir)

    fun secretsDir(baseDir: File): File =
        File(
            File(
                File(baseRoot(baseDir), secretsRootDirName),
                appsDirName,
            ),
            appRuntimeDirName,
        )

    fun accountsFile(context: Context): File = accountsFile(context.noBackupFilesDir)

    fun accountsFile(baseDir: File): File = File(nostrRoot(baseDir), accountsFileName)

    fun secretFile(
        context: Context,
        servicePrefix: String,
        namespace: String,
        name: String,
    ): File = secretFile(context.noBackupFilesDir, servicePrefix, namespace, name)

    fun secretFile(
        baseDir: File,
        servicePrefix: String,
        namespace: String,
        name: String,
    ): File = File(
        secretsDir(baseDir),
        "${secretNamespaceId(servicePrefix, namespace)}.${secretFileId(servicePrefix, namespace, name)}.bin",
    )

    fun legacySecretFile(
        baseDir: File,
        servicePrefix: String,
        namespace: String,
        name: String,
    ): File = File(secretsDir(baseDir), "${secretFileId(servicePrefix, namespace, name)}.bin")

    fun secretNamespaceId(servicePrefix: String, namespace: String): String {
        val digest = MessageDigest.getInstance("SHA-256")
        val encoded = buildString {
            append(servicePrefix)
            append('\u0000')
            append(namespace)
        }.toByteArray(Charsets.UTF_8)
        return digest.digest(encoded).joinToString("") { "%02x".format(it) }
    }

    fun namespaceFilePrefix(servicePrefix: String, namespace: String): String =
        "${secretNamespaceId(servicePrefix, namespace)}."

    fun secretFileId(servicePrefix: String, namespace: String, name: String): String {
        val digest = MessageDigest.getInstance("SHA-256")
        val encoded = buildString {
            append(servicePrefix)
            append('\u0000')
            append(namespace)
            append('\u0000')
            append(name)
        }.toByteArray(Charsets.UTF_8)
        return digest.digest(encoded).joinToString("") { "%02x".format(it) }
    }
}
