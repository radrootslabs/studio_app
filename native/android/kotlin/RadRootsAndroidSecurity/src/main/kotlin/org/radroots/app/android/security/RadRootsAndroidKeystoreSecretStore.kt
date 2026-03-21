package org.radroots.app.android.security

import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyInfo
import android.security.keystore.KeyProperties
import android.security.keystore.StrongBoxUnavailableException
import java.io.File
import java.nio.ByteBuffer
import java.nio.file.AtomicMoveNotSupportedException
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.SecretKeyFactory
import javax.crypto.spec.GCMParameterSpec

class RadRootsAndroidKeystoreSecretStore(
    private val context: Context,
) {
    fun putSecret(
        servicePrefix: String,
        namespace: String,
        name: String,
        value: ByteArray,
        policy: RadRootsAndroidSecretAccessPolicy,
    ) {
        validateIdentifiers(servicePrefix, namespace, name)
        requireSupportedPolicy(policy)
        val key = getOrCreateKey(masterKeyAlias(servicePrefix, namespace), policy)
        val cipher = Cipher.getInstance(cipherTransformation)
        cipher.init(Cipher.ENCRYPT_MODE, key)
        val iv = cipher.iv
        val ciphertext = cipher.doFinal(value)
        val target = RadRootsAndroidStoragePaths.secretFile(context, servicePrefix, namespace, name)
        writeSecretFile(target, encodeSecretBlob(iv, ciphertext))
    }

    fun getSecret(
        servicePrefix: String,
        namespace: String,
        name: String,
    ): ByteArray? {
        validateIdentifiers(servicePrefix, namespace, name)
        val target = RadRootsAndroidStoragePaths.secretFile(context, servicePrefix, namespace, name)
        if (!target.exists()) {
            return null
        }
        val secretBlob = readSecretFile(target)
        val (iv, ciphertext) = decodeSecretBlob(secretBlob)
        val cipher = Cipher.getInstance(cipherTransformation)
        cipher.init(
            Cipher.DECRYPT_MODE,
            getOrCreateKey(
                masterKeyAlias(servicePrefix, namespace),
                RadRootsAndroidSecretAccessPolicy.SECURE_LOCAL_SECRET,
            ),
            GCMParameterSpec(gcmTagBits, iv),
        )
        return try {
            cipher.doFinal(ciphertext)
        } catch (cause: Throwable) {
            throw RadRootsAndroidSecurityError.KeystoreFailure(
                "failed to decrypt secret",
                cause,
            )
        }
    }

    fun deleteSecret(
        servicePrefix: String,
        namespace: String,
        name: String,
    ) {
        validateIdentifiers(servicePrefix, namespace, name)
        val target = RadRootsAndroidStoragePaths.secretFile(context, servicePrefix, namespace, name)
        if (!target.exists()) {
            return
        }
        if (!target.delete()) {
            throw RadRootsAndroidSecurityError.StorageFailure("failed to delete encrypted secret file")
        }
    }

    fun resolveNostrStorageRoot(): File = RadRootsAndroidStoragePaths.nostrRoot(context)

    private fun validateIdentifiers(servicePrefix: String, namespace: String, name: String) {
        if (servicePrefix.isBlank()) {
            throw RadRootsAndroidSecurityError.InvalidInput("service prefix must not be blank")
        }
        if (namespace.isBlank()) {
            throw RadRootsAndroidSecurityError.InvalidInput("namespace must not be blank")
        }
        if (name.isBlank()) {
            throw RadRootsAndroidSecurityError.InvalidInput("name must not be blank")
        }
    }

    private fun requireSupportedPolicy(policy: RadRootsAndroidSecretAccessPolicy) {
        if (!policy.deviceLocalOnly) {
            throw RadRootsAndroidSecurityError.InvalidInput(
                "android security store supports only device-local secrets",
            )
        }
    }

    private fun masterKeyAlias(servicePrefix: String, namespace: String): String =
        "org.radroots.app.android.security.v1.${RadRootsAndroidStoragePaths.secretFileId(servicePrefix, namespace, "master")}"

    private fun getOrCreateKey(
        alias: String,
        policy: RadRootsAndroidSecretAccessPolicy,
    ): SecretKey {
        val keyStore = KeyStore.getInstance(androidKeystoreProvider).apply { load(null) }
        val existing = keyStore.getKey(alias, null)
        if (existing is SecretKey) {
            return existing
        }
        return createKey(alias, policy)
    }

    private fun createKey(
        alias: String,
        policy: RadRootsAndroidSecretAccessPolicy,
    ): SecretKey {
        val requestStrongBox = shouldRequestStrongBox(
            policy = policy,
            sdkInt = Build.VERSION.SDK_INT,
            hasStrongBoxFeature = canRequestStrongBox(),
        )

        return try {
            val generated = generateKey(alias, policy, requestStrongBox = requestStrongBox)
            if (requestStrongBox && !isAcceptableStrongBoxResult(generated.securityLevel)) {
                deleteKey(alias)
                return generateKey(alias, policy, requestStrongBox = false).key
            }
            generated.key
        } catch (cause: StrongBoxUnavailableException) {
            if (!requestStrongBox) {
                throw keystoreFailure(cause)
            }
            deleteKey(alias)
            generateKey(alias, policy, requestStrongBox = false).key
        } catch (cause: Throwable) {
            throw keystoreFailure(cause)
        }
    }

    private fun generateKey(
        alias: String,
        policy: RadRootsAndroidSecretAccessPolicy,
        requestStrongBox: Boolean,
    ): AndroidKeyCreationResult {
        val builder = KeyGenParameterSpec.Builder(
            alias,
            KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
        )
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setKeySize(256)
            .setRandomizedEncryptionRequired(true)

        if (policy.userPresenceRequired) {
            builder.setUserAuthenticationRequired(true)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                builder.setUserAuthenticationParameters(
                    0,
                    KeyProperties.AUTH_BIOMETRIC_STRONG or KeyProperties.AUTH_DEVICE_CREDENTIAL,
                )
            }
        }

        if (requestStrongBox && Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            builder.setIsStrongBoxBacked(true)
        }

        val keyGenerator = KeyGenerator.getInstance(
            KeyProperties.KEY_ALGORITHM_AES,
            androidKeystoreProvider,
        )
        keyGenerator.init(builder.build())
        val key = keyGenerator.generateKey()
        return AndroidKeyCreationResult(
            key = key,
            securityLevel = resolveKeySecurityLevel(key),
        )
    }

    private fun writeSecretFile(target: File, encoded: ByteArray) {
        val parent = target.parentFile
            ?: throw RadRootsAndroidSecurityError.StorageFailure("secret file has no parent directory")
        if (!parent.exists() && !parent.mkdirs()) {
            throw RadRootsAndroidSecurityError.StorageFailure("failed to create secret directory")
        }
        val temp = File(parent, "${target.name}.tmp")
        try {
            temp.writeBytes(encoded)
            try {
                Files.move(
                    temp.toPath(),
                    target.toPath(),
                    StandardCopyOption.ATOMIC_MOVE,
                    StandardCopyOption.REPLACE_EXISTING,
                )
            } catch (_: AtomicMoveNotSupportedException) {
                Files.move(
                    temp.toPath(),
                    target.toPath(),
                    StandardCopyOption.REPLACE_EXISTING,
                )
            }
        } catch (cause: Throwable) {
            temp.delete()
            throw RadRootsAndroidSecurityError.StorageFailure(
                "failed to write encrypted secret file",
                cause,
            )
        }
    }

    private fun readSecretFile(target: File): ByteArray {
        return try {
            target.readBytes()
        } catch (cause: Throwable) {
            throw RadRootsAndroidSecurityError.StorageFailure(
                "failed to read encrypted secret file",
                cause,
            )
        }
    }

    private fun encodeSecretBlob(iv: ByteArray, ciphertext: ByteArray): ByteArray {
        val buffer = ByteBuffer.allocate(1 + Int.SIZE_BYTES + iv.size + ciphertext.size)
        buffer.put(secretBlobVersion)
        buffer.putInt(iv.size)
        buffer.put(iv)
        buffer.put(ciphertext)
        return buffer.array()
    }

    private fun decodeSecretBlob(blob: ByteArray): Pair<ByteArray, ByteArray> {
        try {
            val buffer = ByteBuffer.wrap(blob)
            val version = buffer.get()
            if (version != secretBlobVersion) {
                throw RadRootsAndroidSecurityError.StorageFailure("unsupported encrypted secret version")
            }
            val ivLength = buffer.int
            if (ivLength <= 0 || ivLength > buffer.remaining()) {
                throw RadRootsAndroidSecurityError.StorageFailure("invalid encrypted secret iv length")
            }
            val iv = ByteArray(ivLength)
            buffer.get(iv)
            val ciphertext = ByteArray(buffer.remaining())
            buffer.get(ciphertext)
            return iv to ciphertext
        } catch (error: RadRootsAndroidSecurityError.StorageFailure) {
            throw error
        } catch (cause: Throwable) {
            throw RadRootsAndroidSecurityError.StorageFailure(
                "failed to decode encrypted secret file",
                cause,
            )
        }
    }

    private fun canRequestStrongBox(): Boolean {
        return context.packageManager.hasSystemFeature(PackageManager.FEATURE_STRONGBOX_KEYSTORE)
    }

    private fun isAcceptableStrongBoxResult(
        securityLevel: RadRootsAndroidKeySecurityLevel,
    ): Boolean {
        return acceptsStrongBoxVerificationResult(
            sdkInt = Build.VERSION.SDK_INT,
            securityLevel = securityLevel,
        )
    }

    private fun resolveKeySecurityLevel(key: SecretKey): RadRootsAndroidKeySecurityLevel {
        val keyFactory = SecretKeyFactory.getInstance(key.algorithm, androidKeystoreProvider)
        val keyInfo = keyFactory.getKeySpec(key, KeyInfo::class.java) as KeyInfo
        return RadRootsAndroidKeySecurityLevels.fromKeyInfo(keyInfo)
    }

    private fun deleteKey(alias: String) {
        val keyStore = KeyStore.getInstance(androidKeystoreProvider).apply { load(null) }
        if (keyStore.containsAlias(alias)) {
            keyStore.deleteEntry(alias)
        }
    }

    private fun keystoreFailure(cause: Throwable): RadRootsAndroidSecurityError.KeystoreFailure {
        return when (cause) {
            is RadRootsAndroidSecurityError.KeystoreFailure -> cause
            else -> RadRootsAndroidSecurityError.KeystoreFailure(
                "failed to create keystore secret key",
                cause,
            )
        }
    }

    private companion object {
        const val androidKeystoreProvider = "AndroidKeyStore"
        const val cipherTransformation = "AES/GCM/NoPadding"
        const val gcmTagBits = 128
        const val secretBlobVersion: Byte = 1
    }
}

private data class AndroidKeyCreationResult(
    val key: SecretKey,
    val securityLevel: RadRootsAndroidKeySecurityLevel,
)
