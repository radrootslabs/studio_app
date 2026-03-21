package org.radroots.app.android

import android.content.Context
import java.io.File
import java.io.FileNotFoundException

object RadRootsAndroidAppBridge {
    private const val GEOCODER_ASSET_PATH = "geocoder/geonames.db"
    private const val GEOCODER_FILE_NAME = "geonames.db"

    @Volatile
    private var appContext: Context? = null

    @Volatile
    private var lastErrorMessage: String? = null

    @JvmStatic
    fun initialize(context: Context) {
        appContext = context.applicationContext
    }

    @JvmStatic
    @Synchronized
    fun stageOfflineGeocoderAsset(): String? {
        val context = appContext ?: return fail("android app bridge is not initialized")
        val targetDir = File(context.noBackupFilesDir, "RadRoots/app/android/geocoder")
        if (!targetDir.exists() && !targetDir.mkdirs()) {
            return fail("failed to create android geocoder directory: ${targetDir.absolutePath}")
        }

        val targetFile = File(targetDir, GEOCODER_FILE_NAME)
        return try {
            context.assets.open(GEOCODER_ASSET_PATH).use { input ->
                targetFile.outputStream().use { output ->
                    input.copyTo(output)
                }
            }
            lastErrorMessage = null
            targetFile.absolutePath
        } catch (_: FileNotFoundException) {
            fail("android bundled geocoder asset missing at assets/$GEOCODER_ASSET_PATH")
        } catch (source: Exception) {
            fail("failed to stage android geocoder asset: ${source.message ?: source.javaClass.simpleName}")
        }
    }

    @JvmStatic
    @Synchronized
    fun takeLastErrorMessage(): String? {
        val value = lastErrorMessage
        lastErrorMessage = null
        return value
    }

    @Synchronized
    private fun fail(message: String): String? {
        lastErrorMessage = message
        return null
    }
}
