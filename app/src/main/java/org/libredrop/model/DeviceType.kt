package org.libredrop.model

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
enum class DeviceType {
    @SerialName("mobile")
    MOBILE,

    @SerialName("desktop")
    DESKTOP,

    @SerialName("tablet")
    TABLET,

    @SerialName("server")
    SERVER,

    @SerialName("cli")
    CLI
}
