package org.libredrop.engine

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.serialization.json.Json
import org.libredrop.model.*
import java.io.*
import java.net.*
import java.security.MessageDigest

class KotlinFallbackEngine : DropEngineInterface {
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private val json = Json { ignoreUnknownKeys = true; isLenient = true }

    private val _peers = MutableStateFlow<List<Peer>>(emptyList())
    override val peers: StateFlow<List<Peer>> = _peers.asStateFlow()

    private val _activeTransfer = MutableStateFlow<TransferState?>(null)
    override val activeTransfer: StateFlow<TransferState?> = _activeTransfer.asStateFlow()

    private val _transferHistory = MutableStateFlow<List<TransferState>>(emptyList())
    override val transferHistory: StateFlow<List<TransferState>> = _transferHistory.asStateFlow()

    private val _isScanning = MutableStateFlow(false)
    override val isScanning: StateFlow<Boolean> = _isScanning.asStateFlow()

    private val _isServerRunning = MutableStateFlow(false)
    override val isServerRunning: StateFlow<Boolean> = _isServerRunning.asStateFlow()

    private val _isWebDropActive = MutableStateFlow(false)
    override val isWebDropActive: StateFlow<Boolean> = _isWebDropActive.asStateFlow()

    private var currentConfig: DropConfig? = null
    private var serverSocket: ServerSocket? = null
    private var discoveryJob: Job? = null
    private var peerPollingJob: Job? = null

    override fun init(config: DropConfig): Boolean {
        currentConfig = config
        if (NativeEngine.isLoaded) {
            NativeEngine.init(config)
        }
        return true
    }

    override fun startDiscovery(): Boolean {
        _isScanning.value = true
        if (NativeEngine.isLoaded) {
            NativeEngine.startDiscovery()
            peerPollingJob?.cancel()
            peerPollingJob = scope.launch {
                while (isActive && _isScanning.value) {
                    val nativePeers = NativeEngine.getPeers()
                    _peers.value = nativePeers
                    delay(1500)
                }
            }
            return true
        }

        // Pure Kotlin UDP discovery
        val config = currentConfig ?: return false
        discoveryJob?.cancel()
        discoveryJob = scope.launch {
            val peerMap = mutableMapOf<String, Peer>()

            // Broadcaster
            val broadcaster = launch {
                try {
                    val socket = DatagramSocket()
                    socket.broadcast = true
                    while (isActive) {
                        val packetData = """{"magic":"LIBREDROP_BEACON_V1","device_id":"${config.device_id}","alias":"${config.alias}","device_model":"${config.device_model}","device_type":"mobile","port":${config.port},"version":"1.0.0"}"""
                        val bytes = packetData.toByteArray(Charsets.UTF_8)
                        val packet = DatagramPacket(
                            bytes,
                            bytes.size,
                            InetAddress.getByName("255.255.255.255"),
                            config.discovery_port
                        )
                        socket.send(packet)
                        delay(2500)
                    }
                } catch (_: Throwable) {}
            }

            // Listener
            val listener = launch {
                try {
                    val socket = DatagramSocket(config.discovery_port)
                    val buffer = ByteArray(2048)
                    while (isActive) {
                        val packet = DatagramPacket(buffer, buffer.size)
                        socket.receive(packet)
                        val text = String(packet.data, 0, packet.length, Charsets.UTF_8)
                        if (text.contains("LIBREDROP_BEACON_V1") && !text.contains(config.device_id)) {
                            val ip = packet.address.hostAddress ?: ""
                            val peer = Peer(
                                id = "peer-$ip",
                                alias = "LibreDrop Node ($ip)",
                                deviceModel = "Android / LAN",
                                deviceType = DeviceType.MOBILE,
                                address = ip,
                                port = config.port
                            )
                            peerMap[ip] = peer
                            _peers.value = peerMap.values.toList()
                        }
                    }
                } catch (_: Throwable) {}
            }

            broadcaster.join()
            listener.join()
        }

        return true
    }

    override fun stopDiscovery() {
        _isScanning.value = false
        if (NativeEngine.isLoaded) {
            NativeEngine.stopDiscovery()
        }
        discoveryJob?.cancel()
        peerPollingJob?.cancel()
    }

    override fun startServer(): Boolean {
        _isServerRunning.value = true
        if (NativeEngine.isLoaded) {
            return NativeEngine.startServer()
        }

        val config = currentConfig ?: return false
        scope.launch {
            try {
                serverSocket = ServerSocket(config.port)
                while (isActive) {
                    val client = serverSocket?.accept() ?: break
                    launch { handleIncomingSocket(client) }
                }
            } catch (_: Throwable) {}
        }
        return true
    }

    override fun stopServer() {
        _isServerRunning.value = false
        if (NativeEngine.isLoaded) {
            NativeEngine.stopServer()
        }
        try {
            serverSocket?.close()
        } catch (_: Throwable) {}
    }

    override fun startWebDrop(port: Int): Boolean {
        _isWebDropActive.value = true
        return if (NativeEngine.isLoaded) {
            NativeEngine.startWebDrop(port)
        } else {
            false
        }
    }

    override fun stopWebDrop() {
        _isWebDropActive.value = false
        if (NativeEngine.isLoaded) {
            NativeEngine.stopWebDrop()
        }
    }

    override fun sendFiles(targetPeer: Peer, filePaths: List<String>): Boolean {
        val transferId = "tx-${System.currentTimeMillis()}"
        val items = filePaths.mapIndexed { idx, p ->
            val f = File(p)
            TransferItem(
                id = "item-$idx",
                name = f.name,
                size = f.length(),
                mimeType = "application/octet-stream"
            )
        }
        val totalSize = items.sumOf { it.size }

        val transfer = TransferState(
            transferId = transferId,
            direction = TransferDirection.OUTGOING,
            status = TransferStatus.TRANSFERRING,
            peerAlias = targetPeer.alias,
            peerAddress = targetPeer.address,
            isTextMessage = false,
            files = items,
            totalBytes = totalSize
        )
        _activeTransfer.value = transfer

        if (NativeEngine.isLoaded) {
            scope.launch {
                NativeEngine.sendTransfer(targetPeer.address, targetPeer.port, null, filePaths)
            }
            return true
        }

        // Pure Kotlin file stream
        scope.launch {
            try {
                val socket = Socket(targetPeer.address, targetPeer.port)
                val out = DataOutputStream(socket.getOutputStream())
                val buffer = ByteArray(256 * 1024)
                var bytesSent = 0L

                for ((idx, path) in filePaths.withIndex()) {
                    val file = File(path)
                    val input = FileInputStream(file)
                    var read: Int
                    while (input.read(buffer).also { read = it } != -1) {
                        out.write(buffer, 0, read)
                        bytesSent += read
                        val ratio = if (totalSize > 0) bytesSent.toFloat() / totalSize else 1f
                        _activeTransfer.value = transfer.copy(
                            currentFileIndex = idx,
                            currentFileName = file.name,
                            bytesTransferred = bytesSent,
                            progressRatio = ratio
                        )
                    }
                    input.close()
                }
                out.flush()
                socket.close()

                val completedState = transfer.copy(
                    status = TransferStatus.COMPLETED,
                    bytesTransferred = totalSize,
                    progressRatio = 1f
                )
                _activeTransfer.value = null
                _transferHistory.value = listOf(completedState) + _transferHistory.value
            } catch (e: Exception) {
                _activeTransfer.value = transfer.copy(
                    status = TransferStatus.FAILED,
                    errorMessage = e.message
                )
            }
        }
        return true
    }

    override fun sendMessage(targetPeer: Peer, message: String): Boolean {
        val transferId = "msg-${System.currentTimeMillis()}"
        val transfer = TransferState(
            transferId = transferId,
            direction = TransferDirection.OUTGOING,
            status = TransferStatus.TRANSFERRING,
            peerAlias = targetPeer.alias,
            peerAddress = targetPeer.address,
            isTextMessage = true,
            textContent = message,
            totalBytes = message.length.toLong()
        )
        _activeTransfer.value = transfer

        if (NativeEngine.isLoaded) {
            scope.launch {
                NativeEngine.sendTransfer(targetPeer.address, targetPeer.port, message, emptyList())
            }
            return true
        }

        scope.launch {
            try {
                val socket = Socket(targetPeer.address, targetPeer.port)
                val writer = PrintWriter(socket.getOutputStream(), true)
                writer.println(message)
                socket.close()

                val completed = transfer.copy(status = TransferStatus.COMPLETED, progressRatio = 1f)
                _activeTransfer.value = null
                _transferHistory.value = listOf(completed) + _transferHistory.value
            } catch (e: Exception) {
                _activeTransfer.value = transfer.copy(
                    status = TransferStatus.FAILED,
                    errorMessage = e.message
                )
            }
        }
        return true
    }

    override fun acceptIncomingTransfer(transferId: String) {
        val current = _activeTransfer.value
        if (current?.transferId == transferId) {
            _activeTransfer.value = current.copy(status = TransferStatus.TRANSFERRING)
        }
    }

    override fun declineIncomingTransfer(transferId: String) {
        val current = _activeTransfer.value
        if (current?.transferId == transferId) {
            _activeTransfer.value = current.copy(status = TransferStatus.REJECTED)
        }
    }

    override fun cancelActiveTransfer() {
        val current = _activeTransfer.value
        if (current != null) {
            _activeTransfer.value = current.copy(status = TransferStatus.CANCELLED)
        }
    }

    override fun getLocalIpAddress(): String {
        return if (NativeEngine.isLoaded) {
            NativeEngine.getLocalIp()
        } else {
            try {
                val interfaces = NetworkInterface.getNetworkInterfaces()
                for (intf in interfaces) {
                    val addrs = intf.inetAddresses
                    for (addr in addrs) {
                        if (!addr.isLoopbackAddress && addr is Inet4Address) {
                            return addr.hostAddress ?: "127.0.0.1"
                        }
                    }
                }
                "127.0.0.1"
            } catch (_: Throwable) {
                "127.0.0.1"
            }
        }
    }

    private fun handleIncomingSocket(socket: Socket) {
        try {
            val reader = BufferedReader(InputStreamReader(socket.getInputStream()))
            val firstLine = reader.readLine() ?: return
            val senderIp = socket.inetAddress.hostAddress ?: "Unknown"

            val transfer = TransferState(
                transferId = "in-${System.currentTimeMillis()}",
                direction = TransferDirection.INCOMING,
                status = TransferStatus.COMPLETED,
                peerAlias = "Peer ($senderIp)",
                peerAddress = senderIp,
                isTextMessage = true,
                textContent = firstLine,
                progressRatio = 1f
            )
            _transferHistory.value = listOf(transfer) + _transferHistory.value
            socket.close()
        } catch (_: Throwable) {}
    }
}
