package org.libredrop.model

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class Peer(
    val id: String,
    val alias: String,
    @SerialName("device_model")
    val deviceModel: String,
    @SerialName("device_type")
    val deviceType: DeviceType = DeviceType.MOBILE,
    val address: String,
    val port: Int = 53317,
    @SerialName("last_seen_epoch_ms")
    val lastSeenEpochMs: Long = System.currentTimeMillis()
)
