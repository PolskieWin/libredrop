use crate::models::{DeviceType, DropConfig, PeerInfo};
use crate::utils::get_epoch_ms;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::{broadcast, RwLock};
use tokio::time::{sleep, Duration};

const BEACON_MAGIC: &str = "LIBREDROP_BEACON_V1";
const PEER_TIMEOUT_MS: u64 = 8000;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BeaconPacket {
    magic: String,
    device_id: String,
    alias: String,
    device_model: String,
    device_type: DeviceType,
    port: u16,
    version: String,
}

#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    PeerFound(PeerInfo),
    PeerUpdated(PeerInfo),
    PeerLost(String),
}

pub struct DiscoveryManager {
    config: DropConfig,
    peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
    event_sender: broadcast::Sender<DiscoveryEvent>,
    is_running: Arc<RwLock<bool>>,
}

impl DiscoveryManager {
    pub fn new(config: DropConfig) -> Self {
        let (tx, _) = broadcast::channel(64);
        Self {
            config,
            peers: Arc::new(RwLock::new(HashMap::new())),
            event_sender: tx,
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DiscoveryEvent> {
        self.event_sender.subscribe()
    }

    pub async fn get_peers(&self) -> Vec<PeerInfo> {
        let peers = self.peers.read().await;
        peers.values().cloned().collect()
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut running = self.is_running.write().await;
        if *running {
            return Ok(());
        }
        *running = true;
        drop(running);

        let discovery_port = self.config.discovery_port;
        let my_id = self.config.device_id.clone();
        let config_clone = self.config.clone();
        let peers_clone = self.peers.clone();
        let event_tx = self.event_sender.clone();
        let is_running_rx = self.is_running.clone();

        // 1. Start Listener task
        tokio::spawn(async move {
            let bind_addr = format!("0.0.0.0:{}", discovery_port);
            let socket = match UdpSocket::bind(&bind_addr).await {
                Ok(s) => {
                    let _ = s.set_broadcast(true);
                    s
                }
                Err(e) => {
                    eprintln!("Discovery listener bind failed on {}: {}", bind_addr, e);
                    return;
                }
            };

            let mut buf = [0u8; 2048];
            while *is_running_rx.read().await {
                tokio::select! {
                    res = socket.recv_from(&mut buf) => {
                        match res {
                            Ok((size, sender_addr)) => {
                                if let Ok(beacon) = serde_json::from_slice::<BeaconPacket>(&buf[..size]) {
                                    if beacon.magic == BEACON_MAGIC && beacon.device_id != my_id {
                                        let ip = sender_addr.ip().to_string();
                                        let peer = PeerInfo {
                                            id: beacon.device_id.clone(),
                                            alias: beacon.alias,
                                            device_model: beacon.device_model,
                                            device_type: beacon.device_type,
                                            address: ip,
                                            port: beacon.port,
                                            last_seen_epoch_ms: get_epoch_ms(),
                                        };

                                        let mut p_map = peers_clone.write().await;
                                        if let Some(existing) = p_map.get_mut(&beacon.device_id) {
                                            *existing = peer.clone();
                                            let _ = event_tx.send(DiscoveryEvent::PeerUpdated(peer));
                                        } else {
                                            p_map.insert(beacon.device_id.clone(), peer.clone());
                                            let _ = event_tx.send(DiscoveryEvent::PeerFound(peer));
                                        }
                                    }
                                }
                            }
                            Err(_) => {
                                sleep(Duration::from_millis(500)).await;
                            }
                        }
                    }
                    _ = sleep(Duration::from_millis(500)) => {}
                }
            }
        });

        // 2. Start Broadcaster task
        let is_running_tx = self.is_running.clone();
        let my_config = config_clone.clone();
        tokio::spawn(async move {
            let socket = match UdpSocket::bind("0.0.0.0:0").await {
                Ok(s) => {
                    let _ = s.set_broadcast(true);
                    s
                }
                Err(e) => {
                    eprintln!("Discovery broadcaster bind error: {}", e);
                    return;
                }
            };

            let broadcast_target: SocketAddr = format!("255.255.255.255:{}", my_config.discovery_port)
                .parse()
                .unwrap();

            while *is_running_tx.read().await {
                let packet = BeaconPacket {
                    magic: BEACON_MAGIC.to_string(),
                    device_id: my_config.device_id.clone(),
                    alias: my_config.alias.clone(),
                    device_model: my_config.device_model.clone(),
                    device_type: my_config.device_type.clone(),
                    port: my_config.port,
                    version: "1.0.0".to_string(),
                };

                if let Ok(bytes) = serde_json::to_vec(&packet) {
                    let _ = socket.send_to(&bytes, broadcast_target).await;
                }

                sleep(Duration::from_millis(2500)).await;
            }
        });

        // 3. Start Cleaner task for dead peers
        let peers_cleaner = self.peers.clone();
        let is_running_cleaner = self.is_running.clone();
        let event_tx_cleaner = self.event_sender.clone();
        tokio::spawn(async move {
            while *is_running_cleaner.read().await {
                sleep(Duration::from_secs(3)).await;
                let now = get_epoch_ms();
                let mut p_map = peers_cleaner.write().await;
                let mut dead_ids = Vec::new();

                for (id, peer) in p_map.iter() {
                    if now.saturating_sub(peer.last_seen_epoch_ms) > PEER_TIMEOUT_MS {
                        dead_ids.push(id.clone());
                    }
                }

                for dead in dead_ids {
                    p_map.remove(&dead);
                    let _ = event_tx_cleaner.send(DiscoveryEvent::PeerLost(dead));
                }
            }
        });

        Ok(())
    }

    pub async fn stop(&self) {
        let mut running = self.is_running.write().await;
        *running = false;
    }
}
