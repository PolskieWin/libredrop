package org.libredrop.viewmodel

import android.content.Context
import android.net.Uri
import android.os.Build
import android.provider.OpenableColumns
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.libredrop.LibreDropApp
import org.libredrop.engine.DropConfig
import org.libredrop.model.DeviceType
import org.libredrop.model.Peer
import org.libredrop.model.TransferState
import java.io.File
import java.io.FileOutputStream
import java.util.UUID

class DropViewModel : ViewModel() {
    private val engine = LibreDropApp.engine

    val peers: StateFlow<List<Peer>> = engine.peers
    val activeTransfer: StateFlow<TransferState?> = engine.activeTransfer
    val transferHistory: StateFlow<List<TransferState>> = engine.transferHistory
    val isScanning: StateFlow<Boolean> = engine.isScanning
    val isWebDropActive: StateFlow<Boolean> = engine.isWebDropActive

    private val _selectedTargetPeer = MutableStateFlow<Peer?>(null)
    val selectedTargetPeer: StateFlow<Peer?> = _selectedTargetPeer.asStateFlow()

    private val _selectedUris = MutableStateFlow<List<Uri>>(emptyList())
    val selectedUris: StateFlow<List<Uri>> = _selectedUris.asStateFlow()

    private val _config = MutableStateFlow(
        DropConfig(
            device_id = "droid-" + UUID.randomUUID().toString().take(8),
            alias = "${Build.MANUFACTURER.replaceFirstChar { it.uppercase() }} ${Build.MODEL}",
            device_model = "${Build.BRAND} ${Build.PRODUCT}",
            device_type = DeviceType.MOBILE,
            download_dir = "/sdcard/Download/LibreDrop",
            auto_accept = false
        )
    )
    val config: StateFlow<DropConfig> = _config.asStateFlow()

    fun selectTargetPeer(peer: Peer?) {
        _selectedTargetPeer.value = peer
    }

    fun addUris(uris: List<Uri>) {
        _selectedUris.value = _selectedUris.value + uris
    }

    fun removeUri(uri: Uri) {
        _selectedUris.value = _selectedUris.value.filter { it != uri }
    }

    fun clearSelectedUris() {
        _selectedUris.value = emptyList()
    }

    fun refreshScan() {
        engine.stopDiscovery()
        engine.startDiscovery()
    }

    fun getLocalIp(): String = engine.getLocalIpAddress()

    fun sendSelectedFiles(context: Context) {
        val peer = _selectedTargetPeer.value ?: return
        val uris = _selectedUris.value
        if (uris.isEmpty()) return

        viewModelScope.launch {
            // Resolve URIs to temp cache files or direct paths for zero-downscale transfer
            val filePaths = mutableListOf<String>()
            for (uri in uris) {
                val path = resolveUriToFile(context, uri)
                if (path != null) {
                    filePaths.add(path)
                }
            }

            if (filePaths.isNotEmpty()) {
                engine.sendFiles(peer, filePaths)
                clearSelectedUris()
                selectTargetPeer(null)
            }
        }
    }

    fun sendMessage(peer: Peer, message: String) {
        engine.sendMessage(peer, message)
    }

    fun toggleWebDrop(enable: Boolean) {
        if (enable) {
            engine.startWebDrop(53319)
        } else {
            engine.stopWebDrop()
        }
    }

    fun saveConfig(newConfig: DropConfig) {
        _config.value = newConfig
        engine.init(newConfig)
    }

    fun cancelActiveTransfer() {
        engine.cancelActiveTransfer()
    }

    private fun resolveUriToFile(context: Context, uri: Uri): String? {
        return try {
            val returnCursor = context.contentResolver.query(uri, null, null, null, null)
            val nameIndex = returnCursor?.getColumnIndex(OpenableColumns.DISPLAY_NAME) ?: -1
            returnCursor?.moveToFirst()
            val fileName = if (nameIndex != -1) returnCursor?.getString(nameIndex) else "drop_file"
            returnCursor?.close()

            val tempFile = File(context.cacheDir, fileName ?: "drop_file")
            context.contentResolver.openInputStream(uri)?.use { input ->
                FileOutputStream(tempFile).use { output ->
                    input.copyTo(output)
                }
            }
            tempFile.absolutePath
        } catch (_: Throwable) {
            null
        }
    }
}
