package org.libredrop.ui

import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.viewModels
import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.History
import androidx.compose.material.icons.filled.Language
import androidx.compose.material.icons.filled.Radar
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import org.libredrop.ui.screens.*
import org.libredrop.ui.theme.CyanPrimary
import org.libredrop.ui.theme.DarkBackground
import org.libredrop.ui.theme.DarkSurface
import org.libredrop.ui.theme.LibreDropTheme
import org.libredrop.ui.theme.TextMuted
import org.libredrop.ui.theme.TextSecondary
import org.libredrop.viewmodel.DropViewModel

sealed class Screen(val title: String, val icon: ImageVector) {
    object Radar : Screen("Radar", Icons.Default.Radar)
    object Transfers : Screen("Transfers", Icons.Default.History)
    object WebDrop : Screen("Web Drop", Icons.Default.Language)
    object Settings : Screen("Settings", Icons.Default.Settings)
}

class MainActivity : ComponentActivity() {
    private val viewModel: DropViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        handleShareIntent(intent)

        setContent {
            LibreDropTheme {
                MainAppScaffold(viewModel = viewModel)
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        handleShareIntent(intent)
    }

    private fun handleShareIntent(intent: Intent?) {
        if (intent == null) return
        when (intent.action) {
            Intent.ACTION_SEND -> {
                val uri = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                    intent.getParcelableExtra(Intent.EXTRA_STREAM, Uri::class.java)
                } else {
                    @Suppress("DEPRECATION")
                    intent.getParcelableExtra(Intent.EXTRA_STREAM)
                }
                if (uri != null) {
                    viewModel.addUris(listOf(uri))
                }
            }
            Intent.ACTION_SEND_MULTIPLE -> {
                val uris = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                    intent.getParcelableArrayListExtra(Intent.EXTRA_STREAM, Uri::class.java)
                } else {
                    @Suppress("DEPRECATION")
                    intent.getParcelableArrayListExtra(Intent.EXTRA_STREAM)
                }
                if (uris != null) {
                    viewModel.addUris(uris)
                }
            }
        }
    }
}

@Composable
fun MainAppScaffold(viewModel: DropViewModel) {
    val context = LocalContext.current
    var currentScreen by remember { mutableStateOf<Screen>(Screen.Radar) }

    val peers by viewModel.peers.collectAsState()
    val isScanning by viewModel.isScanning.collectAsState()
    val activeTransfer by viewModel.activeTransfer.collectAsState()
    val transferHistory by viewModel.transferHistory.collectAsState()
    val isWebDropActive by viewModel.isWebDropActive.collectAsState()
    val selectedTargetPeer by viewModel.selectedTargetPeer.collectAsState()
    val selectedUris by viewModel.selectedUris.collectAsState()
    val config by viewModel.config.collectAsState()

    val localIp = remember(peers) { viewModel.getLocalIp() }

    Scaffold(
        bottomBar = {
            NavigationBar(
                containerColor = DarkSurface,
                tonalElevation = 8.dp
            ) {
                val items = listOf(Screen.Radar, Screen.Transfers, Screen.WebDrop, Screen.Settings)
                items.forEach { screen ->
                    val isSelected = currentScreen == screen
                    NavigationBarItem(
                        selected = isSelected,
                        onClick = { currentScreen = screen },
                        icon = { Icon(screen.icon, contentDescription = screen.title) },
                        label = { Text(screen.title) },
                        colors = NavigationBarItemDefaults.colors(
                            selectedIconColor = DarkBackground,
                            selectedTextColor = CyanPrimary,
                            indicatorColor = CyanPrimary,
                            unselectedIconColor = TextSecondary,
                            unselectedTextColor = TextMuted
                        )
                    )
                }
            }
        },
        containerColor = DarkBackground
    ) { paddingValues ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
        ) {
            when {
                selectedTargetPeer != null || selectedUris.isNotEmpty() -> {
                    SendSheet(
                        targetPeer = selectedTargetPeer,
                        selectedUris = selectedUris,
                        onAddUris = { viewModel.addUris(it) },
                        onRemoveUri = { viewModel.removeUri(it) },
                        onConfirmSend = { viewModel.sendSelectedFiles(context) },
                        onCancel = {
                            viewModel.clearSelectedUris()
                            viewModel.selectTargetPeer(null)
                        }
                    )
                }
                currentScreen == Screen.Radar -> {
                    RadarScreen(
                        peers = peers,
                        isScanning = isScanning,
                        localIp = localIp,
                        activeTransfer = activeTransfer,
                        onRefreshScan = { viewModel.refreshScan() },
                        onSendFiles = { peer -> viewModel.selectTargetPeer(peer) },
                        onSendMessage = { peer, msg -> viewModel.sendMessage(peer, msg) },
                        onCancelTransfer = { viewModel.cancelActiveTransfer() }
                    )
                }
                currentScreen == Screen.Transfers -> {
                    TransferScreen(
                        activeTransfer = activeTransfer,
                        history = transferHistory,
                        onCancelTransfer = { viewModel.cancelActiveTransfer() }
                    )
                }
                currentScreen == Screen.WebDrop -> {
                    WebDropScreen(
                        isActive = isWebDropActive,
                        localIp = localIp,
                        port = 53319,
                        onToggleActive = { viewModel.toggleWebDrop(it) }
                    )
                }
                currentScreen == Screen.Settings -> {
                    SettingsScreen(
                        config = config,
                        onSaveConfig = { viewModel.saveConfig(it) }
                    )
                }
            }
        }
    }
}
