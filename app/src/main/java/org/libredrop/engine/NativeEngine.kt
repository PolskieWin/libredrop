package org.libredrop.engine

import kotlinx.serialization.json.Json
import org.libredrop.model.Peer

object NativeEngine {
    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
    }

    var isLoaded: Boolean = false
        private set

    init {
        try {
            System.loadLibrary("libredrop_engine")
            isLoaded = true
        } catch (t: Throwable) {
            isLoaded = false
        }
    }

    private external fun nativeGetVersion(): String
    private external fun nativeGetLocalIp(): String
    private external fun nativeInit(configJson: String): Boolean
    private external fun nativeStartDiscovery(): Boolean
    private external fun nativeStopDiscovery()
    private external fun nativeGetPeers(): String
    private external fun nativeStartServer(): Boolean
    private external fun nativeStopServer()
    private external fun nativeSendTransfer(
        targetIp: String,
        targetPort: Int,
        message: String?,
        files: Array<String>?
    ): Boolean
    private external fun nativeStartWebDrop(webPort: Int): Boolean
    private external fun nativeStopWebDrop()

    fun getVersion(): String = try {
        if (isLoaded) nativeGetVersion() else "LibreDrop Pure-Kotlin Fallback Engine"
    } catch (e: Throwable) {
        "LibreDrop Engine"
    }

    fun getLocalIp(): String = try {
        if (isLoaded) nativeGetLocalIp() else "127.0.0.1"
    } catch (e: Throwable) {
        "127.0.0.1"
    }

    fun init(config: DropConfig): Boolean = try {
        if (isLoaded) {
            val configStr = json.encodeToString(DropConfig.serializer(), config)
            nativeInit(configStr)
        } else false
    } catch (e: Throwable) {
        false
    }

    fun startDiscovery(): Boolean = try {
        if (isLoaded) nativeStartDiscovery() else false
    } catch (e: Throwable) {
        false
    }

    fun stopDiscovery() = try {
        if (isLoaded) nativeStopDiscovery() else Unit
    } catch (e: Throwable) {
        // ignore
    }

    fun getPeers(): List<Peer> = try {
        if (isLoaded) {
            val peersJson = nativeGetPeers()
            json.decodeFromString<List<Peer>>(peersJson)
        } else emptyList()
    } catch (e: Throwable) {
        emptyList()
    }

    fun startServer(): Boolean = try {
        if (isLoaded) nativeStartServer() else false
    } catch (e: Throwable) {
        false
    }

    fun stopServer() = try {
        if (isLoaded) nativeStopServer() else Unit
    } catch (e: Throwable) {
        // ignore
    }

    fun sendTransfer(
        targetIp: String,
        targetPort: Int,
        message: String?,
        files: List<String>
    ): Boolean = try {
        if (isLoaded) {
            nativeSendTransfer(targetIp, targetPort, message, files.toTypedArray())
        } else false
    } catch (e: Throwable) {
        false
    }

    fun startWebDrop(webPort: Int): Boolean = try {
        if (isLoaded) nativeStartWebDrop(webPort) else false
    } catch (e: Throwable) {
        false
    }

    fun stopWebDrop() = try {
        if (isLoaded) nativeStopWebDrop() else Unit
    } catch (e: Throwable) {
        // ignore
    }
}
