package org.libredrop.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.libredrop.model.TransferDirection
import org.libredrop.model.TransferState
import org.libredrop.model.TransferStatus
import org.libredrop.ui.theme.*

@Composable
fun TransferCard(
    transfer: TransferState,
    onCancel: () -> Unit,
    modifier: Modifier = Modifier
) {
    Card(
        modifier = modifier
            .fillMaxWidth()
            .border(1.dp, CyanPrimary.copy(alpha = 0.5f), RoundedCornerShape(16.dp)),
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(containerColor = DarkSurfaceElevated)
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Column {
                    val directionText = if (transfer.direction == TransferDirection.OUTGOING) "Sending to" else "Receiving from"
                    Text(
                        text = "$directionText ${transfer.peerAlias}",
                        style = MaterialTheme.typography.labelSmall,
                        color = CyanPrimary
                    )
                    Spacer(modifier = Modifier.height(2.dp))
                    Text(
                        text = if (transfer.isTextMessage) "Text Message" else transfer.currentFileName.ifEmpty { "Files" },
                        style = MaterialTheme.typography.titleMedium,
                        color = TextPrimary
                    )
                }

                if (transfer.status == TransferStatus.TRANSFERRING) {
                    IconButton(
                        onClick = onCancel,
                        modifier = Modifier.size(32.dp)
                    ) {
                        Icon(
                            imageVector = Icons.Default.Close,
                            contentDescription = "Cancel",
                            tint = TextSecondary,
                            modifier = Modifier.size(18.dp)
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(12.dp))

            // Progress bar
            LinearProgressIndicator(
                progress = { transfer.progressRatio.coerceIn(0f, 1f) },
                modifier = Modifier
                    .fillMaxWidth()
                    .height(6.dp)
                    .clip(RoundedCornerShape(3.dp)),
                color = CyanPrimary,
                trackColor = DarkSurface
            )

            Spacer(modifier = Modifier.height(8.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                val percent = (transfer.progressRatio * 100f).toInt()
                Text(
                    text = "$percent%",
                    style = MaterialTheme.typography.bodyMedium,
                    color = TextPrimary,
                    fontSize = 12.sp
                )

                if (transfer.speedBytesSec > 0) {
                    Text(
                        text = "${formatBytes(transfer.speedBytesSec)}/s",
                        style = MaterialTheme.typography.bodyMedium,
                        color = SuccessGreen,
                        fontSize = 12.sp
                    )
                }

                Text(
                    text = "${formatBytes(transfer.bytesTransferred)} / ${formatBytes(transfer.totalBytes)}",
                    style = MaterialTheme.typography.bodyMedium,
                    color = TextSecondary,
                    fontSize = 12.sp
                )
            }
        }
    }
}

private fun formatBytes(bytes: Long): String {
    val kb = 1024L
    val mb = kb * 1024L
    val gb = mb * 1024L
    return when {
        bytes >= gb -> String.format("%.2f GB", bytes.toDouble() / gb)
        bytes >= mb -> String.format("%.2f MB", bytes.toDouble() / mb)
        bytes >= kb -> String.format("%.2f KB", bytes.toDouble() / kb)
        else -> "$bytes B"
    }
}
