#![forbid(unsafe_code)]

use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{Mutex, broadcast};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StorageRequest {
    AddDevice { device_id: String },
    RemoveDevice { device_id: String },
    ConnectPeer { peer_id: String },
    SyncPeer { peer_id: String },
    SendMessage { peer_id: String, payload: String },
    CloseSession { peer_id: String },
    ReadFile { path: String },
    WriteFile { path: String, content: String },
    ListDir { path: String },
    CreateDir { path: String },
    DeletePath { path: String },
    RenamePath { from: String, to: String },
    ExistsPath { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StorageEvent {
    #[serde(rename_all = "camelCase")]
    Connected { peer_id: String },
    #[serde(rename_all = "camelCase")]
    MessageReceived { peer_id: String, payload: String },
    #[serde(rename_all = "camelCase")]
    Closed {
        peer_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<StorageEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl StorageResponse {
    /// Creates a successful response with optional event and/or payload.
    pub fn success(event: Option<StorageEvent>, payload: Option<Value>) -> Self {
        StorageResponse {
            ok: true,
            event,
            payload,
            error: None,
        }
    }

    /// Creates a successful response with only a payload.
    pub fn success_payload(payload: impl Into<Value>) -> Self {
        StorageResponse {
            ok: true,
            event: None,
            payload: Some(payload.into()),
            error: None,
        }
    }

    /// Creates a successful response with only an event.
    pub fn success_event(event: StorageEvent) -> Self {
        StorageResponse {
            ok: true,
            event: Some(event),
            payload: None,
            error: None,
        }
    }

    /// Creates a successful response with no additional data.
    pub fn success_empty() -> Self {
        StorageResponse {
            ok: true,
            event: None,
            payload: None,
            error: None,
        }
    }

    /// Creates an error response.
    pub fn error(error: String) -> Self {
        StorageResponse {
            ok: false,
            event: None,
            payload: None,
            error: Some(error),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_request_read_file_serde() {
        let request = StorageRequest::ReadFile {
            path: "example/hello.txt".to_string(),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"readFile\""));
        assert!(json.contains("\"path\""));

        let deserialized: StorageRequest = serde_json::from_str(&json).unwrap();
        match deserialized {
            StorageRequest::ReadFile { path } => assert_eq!(path, "example/hello.txt"),
            _ => panic!("Expected ReadFile"),
        }
    }

    #[test]
    fn test_storage_request_write_file_serde() {
        let request = StorageRequest::WriteFile {
            path: "example/hello.txt".to_string(),
            content: "Hello, World!".to_string(),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"writeFile\""));
        assert!(json.contains("\"path\""));
        assert!(json.contains("\"content\""));

        let deserialized: StorageRequest = serde_json::from_str(&json).unwrap();
        match deserialized {
            StorageRequest::WriteFile { path, content } => {
                assert_eq!(path, "example/hello.txt");
                assert_eq!(content, "Hello, World!");
            }
            _ => panic!("Expected WriteFile"),
        }
    }

    #[test]
    fn test_storage_request_list_dir_serde() {
        let request = StorageRequest::ListDir {
            path: "example".to_string(),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"listDir\""));
        assert!(json.contains("\"path\""));

        let deserialized: StorageRequest = serde_json::from_str(&json).unwrap();
        match deserialized {
            StorageRequest::ListDir { path } => assert_eq!(path, "example"),
            _ => panic!("Expected ListDir"),
        }
    }

    #[test]
    fn test_storage_response_success_payload() {
        let response = StorageResponse::success_payload("Hello, World!".to_string());
        let json = serde_json::to_string(&response).unwrap();

        // Verify the JSON structure matches TS format: { ok: true, payload: "..." }
        assert!(json.contains("\"ok\":true"));
        assert!(json.contains("\"payload\""));
        // error and event should not be present when using skip_serializing_if
        assert!(!json.contains("\"error\""));
        assert!(!json.contains("\"event\""));

        let deserialized: StorageResponse = serde_json::from_str(&json).unwrap();
        assert!(deserialized.ok);
        assert_eq!(
            deserialized.payload,
            Some(serde_json::json!("Hello, World!"))
        );
        assert!(deserialized.event.is_none());
        assert!(deserialized.error.is_none());
    }

    #[test]
    fn test_storage_response_error() {
        let response = StorageResponse::error("File not found".to_string());
        let json = serde_json::to_string(&response).unwrap();

        // Verify the JSON structure matches TS format: { ok: false, error: "..." }
        assert!(json.contains("\"ok\":false"));
        assert!(json.contains("\"error\""));
        // payload and event should not be present
        assert!(!json.contains("\"payload\""));
        assert!(!json.contains("\"event\""));

        let deserialized: StorageResponse = serde_json::from_str(&json).unwrap();
        assert!(!deserialized.ok);
        assert_eq!(deserialized.error, Some("File not found".to_string()));
        assert!(deserialized.payload.is_none());
        assert!(deserialized.event.is_none());
    }

    #[test]
    fn test_storage_response_success_event() {
        let event = StorageEvent::Connected {
            peer_id: "peer123".to_string(),
        };
        let response = StorageResponse::success_event(event);
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"ok\":true"));
        assert!(json.contains("\"event\""));
        assert!(json.contains("\"type\":\"connected\""));
        assert!(json.contains("\"peerId\""));

        let deserialized: StorageResponse = serde_json::from_str(&json).unwrap();
        assert!(deserialized.ok);
        assert!(deserialized.event.is_some());
        assert!(deserialized.payload.is_none());
    }

    #[test]
    fn test_storage_event_serde_camel_case() {
        let event = StorageEvent::MessageReceived {
            peer_id: "peer123".to_string(),
            payload: "Hello".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();

        // Properties should be in camelCase
        assert!(json.contains("\"peerId\""));
        assert!(!json.contains("\"peer_id\""));
        assert!(json.contains("\"payload\""));
        assert!(json.contains("\"type\":\"messageReceived\""));

        let deserialized: StorageEvent = serde_json::from_str(&json).unwrap();
        match deserialized {
            StorageEvent::MessageReceived { peer_id, payload } => {
                assert_eq!(peer_id, "peer123");
                assert_eq!(payload, "Hello");
            }
            _ => panic!("Expected MessageReceived"),
        }
    }
}
