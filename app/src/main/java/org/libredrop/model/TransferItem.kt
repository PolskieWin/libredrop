package org.libredrop.model

import kotlinx.serialization.Serializable

@Serializable
data class TransferItem(
    val id: String,
    val name: String,
    val size: Long,
    val mimeType: String,
    val uriString: String? = null,
    val sha256: String? = null
)
