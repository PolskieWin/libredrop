package org.libredrop.engine

import kotlinx.coroutines.flow.StateFlow
import org.libredrop.model.Peer
import org.libredrop.model.TransferState

interface DropEngineInterface {
    val peers: StateFlow<List<Peer>>
    val activeTransfer: StateFlow<TransferState?>
    val transferHistory: StateFlow<List<TransferState>>
    val isScanning: StateFlow<Boolean>
    val isServerRunning: StateFlow<Boolean>
    val isWebDropActive: StateFlow<Boolean>

    fun init(config: DropConfig): Boolean
    fun startDiscovery(): Boolean
    fun stopDiscovery()
    fun startServer(): Boolean
    fun stopServer()
    fun startWebDrop(port: Int): Boolean
    fun stopWebDrop()
    fun sendFiles(targetPeer: Peer, filePaths: List<String>): Boolean
    fun sendMessage(targetPeer: Peer, message: String): Boolean
    fun acceptIncomingTransfer(transferId: String)
    fun declineIncomingTransfer(transferId: String)
    fun cancelActiveTransfer()
    fun getLocalIpAddress(): String
}
