package org.radroots.app.android

import android.content.Context
import java.io.File
import java.io.FileNotFoundException

object RadRootsAndroidAppBridge {
    private const val GEOCODER_ASSET_PATH = "geocoder/geonames.db"
    private const val GEOCODER_REVISION_ASSET_PATH = "geocoder/geonames.revision"
    private const val GEOCODER_FILE_NAME = "geonames.db"
    private const val GEOCODER_ERROR_KIND_MISSING_BUILD_ASSET = 1
    private const val GEOCODER_ERROR_KIND_INITIALIZATION_FAILED = 2
    private const val GEOCODER_ERROR_KIND_INTERNAL_ERROR = 3

    @Volatile
    private var appContext: Context? = null

    @Volatile
    private var lastErrorMessage: String? = null

    @Volatile
    private var lastErrorKind: Int = 0

    @JvmStatic
    fun initialize(context: Context) {
        appContext = context.applicationContext
    }

    @JvmStatic
    @Synchronized
    fun stageOfflineGeocoderAsset(): String? {
        val context = appContext
            ?: return fail(
                GEOCODER_ERROR_KIND_INTERNAL_ERROR,
                "android app bridge is not initialized",
            )
        val revision = loadGeocoderRevision(context) ?: return null
        val targetDir = stagedGeocoderDirectory(context, revision)
        if (!targetDir.exists() && !targetDir.mkdirs()) {
            return fail(
                GEOCODER_ERROR_KIND_INITIALIZATION_FAILED,
                "failed to create android geocoder directory: ${targetDir.absolutePath}",
            )
        }

        val targetFile = File(targetDir, GEOCODER_FILE_NAME)
        if (targetFile.isFile) {
            lastErrorMessage = null
            lastErrorKind = 0
            return targetFile.absolutePath
        }
        return try {
            context.assets.open(GEOCODER_ASSET_PATH).use { input ->
                targetFile.outputStream().use { output ->
                    input.copyTo(output)
                }
            }
            lastErrorMessage = null
            lastErrorKind = 0
            targetFile.absolutePath
        } catch (_: FileNotFoundException) {
            fail(
                GEOCODER_ERROR_KIND_MISSING_BUILD_ASSET,
                "android bundled geocoder asset missing at assets/$GEOCODER_ASSET_PATH",
            )
        } catch (source: Exception) {
            fail(
                GEOCODER_ERROR_KIND_INITIALIZATION_FAILED,
                "failed to stage android geocoder asset: ${source.message ?: source.javaClass.simpleName}",
            )
        }
    }

    private fun loadGeocoderRevision(context: Context): String? {
        val revision = try {
            context.assets.open(GEOCODER_REVISION_ASSET_PATH).bufferedReader().use { it.readText() }
        } catch (_: FileNotFoundException) {
            return fail(
                GEOCODER_ERROR_KIND_MISSING_BUILD_ASSET,
                "android bundled geocoder revision asset missing at assets/$GEOCODER_REVISION_ASSET_PATH",
            )
        } catch (source: Exception) {
            return fail(
                GEOCODER_ERROR_KIND_MISSING_BUILD_ASSET,
                "failed to read android geocoder revision asset at assets/$GEOCODER_REVISION_ASSET_PATH: ${source.message ?: source.javaClass.simpleName}",
            )
        }.trim()

        if (!isValidRevision(revision)) {
            return fail(
                GEOCODER_ERROR_KIND_MISSING_BUILD_ASSET,
                "android bundled geocoder revision asset invalid at assets/$GEOCODER_REVISION_ASSET_PATH",
            )
        }

        return revision
    }

    private fun isValidRevision(revision: String): Boolean {
        return revision.length == 64 && revision.all { it.isDigit() || it.lowercaseChar() in 'a'..'f' }
    }

    private fun stagedGeocoderDirectory(context: Context, revision: String): File {
        return File(context.noBackupFilesDir, "RadRoots/app/android/geocoder/$revision")
    }

    @JvmStatic
    @Synchronized
    fun takeLastErrorKind(): Int {
        val value = lastErrorKind
        lastErrorKind = 0
        return value
    }

    @JvmStatic
    @Synchronized
    fun takeLastErrorMessage(): String? {
        val value = lastErrorMessage
        lastErrorMessage = null
        return value
    }

    @Synchronized
    private fun fail(kind: Int, message: String): String? {
        lastErrorKind = kind
        lastErrorMessage = message
        return null
    }
}
