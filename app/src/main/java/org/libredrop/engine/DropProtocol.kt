package org.libredrop.engine

import kotlinx.serialization.Serializable
import org.libredrop.model.DeviceType

@Serializable
data class DropConfig(
    val device_id: String,
    val alias: String,
    val device_model: String,
    val device_type: DeviceType = DeviceType.MOBILE,
    val port: Int = 53317,
    val discovery_port: Int = 53318,
    val download_dir: String,
    val auto_accept: Boolean = false
)

object DropProtocol {
    const val DEFAULT_TRANSFER_PORT = 53317
    const val DEFAULT_DISCOVERY_PORT = 53318
    const val DEFAULT_WEB_PORT = 53319
    const val MAGIC_HEADER = "LDRP"
    const val PROTOCOL_VERSION = 1
}
