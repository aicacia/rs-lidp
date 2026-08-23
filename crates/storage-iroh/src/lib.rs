#![forbid(unsafe_code)]

use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, broadcast};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StorageRequest {
    AddDevice { device_id: String },
    RemoveDevice { device_id: String },
    ConnectPeer { peer_id: String },
    SyncPeer { peer_id: String },
    SendMessage { peer_id: String, payload: String },
    CloseSession { peer_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StorageEvent {
    Connected {
        peer_id: String,
    },
    MessageReceived {
        peer_id: String,
        payload: String,
    },
    Closed {
        peer_id: String,
        reason: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StorageResponse {
    Ok {
        event: Option<StorageEvent>,
        payload: Option<String>,
    },
    Err {
        error: String,
    },
}

#[derive(Debug, Clone)]
pub struct StorageIrohRuntime {
    peers: Arc<Mutex<HashMap<String, bool>>>,
    devices: Arc<Mutex<HashMap<String, bool>>>,
    events: Arc<broadcast::Sender<StorageEvent>>,
}

impl StorageIrohRuntime {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(128);
        Self {
            peers: Arc::new(Mutex::new(HashMap::new())),
            devices: Arc::new(Mutex::new(HashMap::new())),
            events: Arc::new(events),
        }
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<StorageEvent> {
        self.events.subscribe()
    }

    pub async fn connect_peer(&self, peer_id: &str) -> Result<StorageEvent, String> {
        if peer_id.trim().is_empty() {
            return Err("peer id cannot be empty".to_string());
        }

        let mut peers = self.peers.lock().await;
        peers.insert(peer_id.to_string(), true);
        drop(peers);

        let event = StorageEvent::Connected {
            peer_id: peer_id.to_string(),
        };
        let _ = self.events.send(event.clone());
        Ok(event)
    }

    pub async fn sync_peer(&self, peer_id: &str) -> Result<StorageEvent, String> {
        if peer_id.trim().is_empty() {
            return Err("peer id cannot be empty".to_string());
        }

        let peers = self.peers.lock().await;
        if !peers.contains_key(peer_id) {
            return Err(format!("peer '{peer_id}' is not connected"));
        }
        drop(peers);

        let event = StorageEvent::Connected {
            peer_id: peer_id.to_string(),
        };
        let _ = self.events.send(event.clone());
        Ok(event)
    }

    pub async fn send_message(&self, peer_id: &str, payload: &str) -> Result<StorageEvent, String> {
        if peer_id.trim().is_empty() {
            return Err("peer id cannot be empty".to_string());
        }

        let peers = self.peers.lock().await;
        if !peers.contains_key(peer_id) {
            return Err(format!("peer '{peer_id}' is not connected"));
        }
        drop(peers);

        let event = StorageEvent::MessageReceived {
            peer_id: peer_id.to_string(),
            payload: payload.to_string(),
        };
        let _ = self.events.send(event.clone());
        Ok(event)
    }

    pub async fn close_session(&self, peer_id: &str) -> Result<StorageEvent, String> {
        if peer_id.trim().is_empty() {
            return Err("peer id cannot be empty".to_string());
        }

        let mut peers = self.peers.lock().await;
        peers.remove(peer_id);
        drop(peers);

        let event = StorageEvent::Closed {
            peer_id: peer_id.to_string(),
            reason: Some("session closed".to_string()),
        };
        let _ = self.events.send(event.clone());
        Ok(event)
    }

    pub async fn add_device(&self, device_id: &str) -> Result<StorageEvent, String> {
        if device_id.trim().is_empty() {
            return Err("device id cannot be empty".to_string());
        }

        let mut devices = self.devices.lock().await;
        devices.insert(device_id.to_string(), true);
        drop(devices);

        let event = StorageEvent::Connected {
            peer_id: device_id.to_string(),
        };
        let _ = self.events.send(event.clone());
        Ok(event)
    }

    pub async fn remove_device(&self, device_id: &str) -> Result<StorageEvent, String> {
        if device_id.trim().is_empty() {
            return Err("device id cannot be empty".to_string());
        }

        let mut devices = self.devices.lock().await;
        devices.remove(device_id);
        drop(devices);

        let event = StorageEvent::Closed {
            peer_id: device_id.to_string(),
            reason: Some("device removed".to_string()),
        };
        let _ = self.events.send(event.clone());
        Ok(event)
    }
}
