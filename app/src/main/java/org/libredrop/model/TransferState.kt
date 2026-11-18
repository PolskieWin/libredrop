package org.libredrop.model

import kotlinx.serialization.Serializable

@Serializable
enum class TransferDirection {
    INCOMING,
    OUTGOING
}

@Serializable
enum class TransferStatus {
    WAITING_APPROVAL,
    TRANSFERRING,
    COMPLETED,
    REJECTED,
    FAILED,
    CANCELLED
}

@Serializable
data class TransferState(
    val transferId: String,
    val direction: TransferDirection,
    val status: TransferStatus,
    val peerAlias: String,
    val peerAddress: String,
    val isTextMessage: Boolean = false,
    val textContent: String? = null,
    val files: List<TransferItem> = emptyList(),
    val currentFileIndex: Int = 0,
    val currentFileName: String = "",
    val bytesTransferred: Long = 0,
    val totalBytes: Long = 0,
    val speedBytesSec: Long = 0,
    val progressRatio: Float = 0f,
    val errorMessage: String? = null,
    val timestamp: Long = System.currentTimeMillis()
)
