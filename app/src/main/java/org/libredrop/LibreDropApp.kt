package org.libredrop

import android.app.Application
import android.os.Build
import org.libredrop.engine.DropConfig
import org.libredrop.engine.DropEngineInterface
import org.libredrop.engine.DropService
import org.libredrop.engine.KotlinFallbackEngine
import org.libredrop.model.DeviceType
import java.io.File
import java.util.UUID

class LibreDropApp : Application() {
    companion object {
        lateinit var instance: LibreDropApp
            private set

        lateinit var engine: DropEngineInterface
            private set
    }

    override fun onCreate() {
        super.onCreate()
        instance = this

        val downloadDir = File(getExternalFilesDir(null) ?: filesDir, "LibreDrop").apply {
            mkdirs()
        }.absolutePath

        val deviceName = "${Build.MANUFACTURER.replaceFirstChar { it.uppercase() }} ${Build.MODEL}"
        val deviceId = "droid-" + UUID.randomUUID().toString().take(8)

        val config = DropConfig(
            device_id = deviceId,
            alias = deviceName,
            device_model = "${Build.BRAND} ${Build.PRODUCT}",
            device_type = DeviceType.MOBILE,
            download_dir = downloadDir,
            auto_accept = false
        )

        engine = KotlinFallbackEngine()
        engine.init(config)
        engine.startServer()
        engine.startDiscovery()

        DropService.startService(this)
    }
}
