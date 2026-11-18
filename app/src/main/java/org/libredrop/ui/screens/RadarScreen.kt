package org.libredrop.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Wifi
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.libredrop.model.Peer
import org.libredrop.model.TransferState
import org.libredrop.ui.components.MessageDropDialog
import org.libredrop.ui.components.PeerCard
import org.libredrop.ui.components.RadarCanvas
import org.libredrop.ui.components.TransferCard
import org.libredrop.ui.theme.*

@Composable
fun RadarScreen(
    peers: List<Peer>,
    isScanning: Boolean,
    localIp: String,
    activeTransfer: TransferState?,
    onRefreshScan: () -> Unit,
    onSendFiles: (Peer) -> Unit,
    onSendMessage: (Peer, String) -> Unit,
    onCancelTransfer: () -> Unit,
    modifier: Modifier = Modifier
) {
    var messageDialogPeer by remember { mutableStateOf<Peer?>(null) }

    Column(
        modifier = modifier
            .fillMaxSize()
            .background(DarkBackground)
            .padding(horizontal = 16.dp)
    ) {
        Spacer(modifier = Modifier.height(16.dp))

        // Header Status Banner
        Card(
            modifier = Modifier
                .fillMaxWidth()
                .border(1.dp, BorderColor, RoundedCornerShape(16.dp)),
            shape = RoundedCornerShape(16.dp),
            colors = CardDefaults.cardColors(containerColor = DarkSurface)
        ) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(14.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(10.dp)
                ) {
                    Box(
                        modifier = Modifier
                            .size(36.dp)
                            .clip(CircleShape)
                            .background(SuccessGreen.copy(alpha = 0.15f)),
                        contentAlignment = Alignment.Center
                    ) {
                        Icon(
                            imageVector = Icons.Default.Wifi,
                            contentDescription = null,
                            tint = SuccessGreen,
                            modifier = Modifier.size(20.dp)
                        )
                    }

                    Column {
                        Text(
                            text = "LibreDrop Radar",
                            style = MaterialTheme.typography.titleMedium,
                            color = TextPrimary
                        )
                        Text(
                            text = "Local IP: $localIp • Zero Downscaling",
                            style = MaterialTheme.typography.bodyMedium,
                            color = TextSecondary,
                            fontSize = 11.sp
                        )
                    }
                }

                IconButton(
                    onClick = onRefreshScan,
                    modifier = Modifier
                        .size(36.dp)
                        .clip(CircleShape)
                        .background(DarkSurfaceElevated)
                ) {
                    Icon(
                        imageVector = Icons.Default.Refresh,
                        contentDescription = "Refresh Scan",
                        tint = CyanPrimary,
                        modifier = Modifier.size(18.dp)
                    )
                }
            }
        }

        Spacer(modifier = Modifier.height(12.dp))

        // Active Transfer Card
        if (activeTransfer != null) {
            TransferCard(
                transfer = activeTransfer,
                onCancel = onCancelTransfer,
                modifier = Modifier.padding(bottom = 12.dp)
            )
        }

        // Radar Visualizer & Peers List
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .weight(1f)
        ) {
            if (peers.isEmpty()) {
                // Centered Radar Canvas when waiting for peers
                Column(
                    modifier = Modifier.fillMaxSize(),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.Center
                ) {
                    Box(modifier = Modifier.size(220.dp)) {
                        RadarCanvas(isScanning = isScanning)
                    }
                    Spacer(modifier = Modifier.height(16.dp))
                    Text(
                        text = if (isScanning) "Scanning local Wi-Fi for peers..." else "Tap Refresh to scan",
                        style = MaterialTheme.typography.bodyLarge,
                        color = TextSecondary,
                        fontWeight = FontWeight.Medium
                    )
                    Spacer(modifier = Modifier.height(4.dp))
                    Text(
                        text = "Both devices must be on the same network or hotspot",
                        style = MaterialTheme.typography.bodyMedium,
                        color = TextMuted,
                        fontSize = 12.sp
                    )
                }
            } else {
                LazyColumn(
                    modifier = Modifier.fillMaxSize(),
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                    contentPadding = PaddingValues(vertical = 8.dp)
                ) {
                    item {
                        Text(
                            text = "Discovered Devices (${peers.size})",
                            style = MaterialTheme.typography.labelSmall,
                            color = CyanPrimary,
                            modifier = Modifier.padding(bottom = 4.dp)
                        )
                    }
                    items(peers, key = { it.id }) { peer ->
                        PeerCard(
                            peer = peer,
                            onSendFiles = { onSendFiles(peer) },
                            onSendMessage = { messageDialogPeer = peer }
                        )
                    }
                }
            }
        }
    }

    messageDialogPeer?.let { peer ->
        MessageDropDialog(
            peer = peer,
            onDismiss = { messageDialogPeer = null },
            onSend = { text -> onSendMessage(peer, text) }
        )
    }
}
