use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::{
    Router,
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
};
use serde::{Deserialize, Serialize};
use storage_iroh::{StorageEvent, StorageIrohRuntime, StorageRequest, StorageResponse};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum BridgeMessage {
    Request {
        #[serde(flatten)]
        request: StorageRequest,
        request_id: u64,
    },
    Response {
        #[serde(flatten)]
        response: StorageResponse,
        request_id: u64,
    },
    Event(StorageEvent),
}

#[derive(Debug, Clone)]
pub struct StorageBridge {
    runtime: StorageIrohRuntime,
}

impl StorageBridge {
    pub fn new() -> Self {
        Self {
            runtime: StorageIrohRuntime::new(),
        }
    }

    pub async fn handle_request(&self, request: StorageRequest) -> Result<StorageResponse, String> {
        let event = match request {
            StorageRequest::AddDevice { device_id } => self.runtime.add_device(&device_id).await?,
            StorageRequest::RemoveDevice { device_id } => {
                self.runtime.remove_device(&device_id).await?
            }
            StorageRequest::ConnectPeer { peer_id } => self.runtime.connect_peer(&peer_id).await?,
            StorageRequest::SyncPeer { peer_id } => self.runtime.sync_peer(&peer_id).await?,
            StorageRequest::SendMessage { peer_id, payload } => {
                self.runtime.send_message(&peer_id, &payload).await?
            }
            StorageRequest::CloseSession { peer_id } => {
                self.runtime.close_session(&peer_id).await?
            }
        };

        Ok(StorageResponse::Ok {
            event: Some(event),
            payload: None,
        })
    }

    pub async fn start_server(self) -> Result<(), String> {
        let state = Arc::new(self);

        let app = Router::new()
            .route("/bridge", get(Self::bridge_handler))
            .with_state(state);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:3042")
            .await
            .map_err(|err| err.to_string())?;

        axum::serve(listener, app)
            .await
            .map_err(|err| err.to_string())?;

        Ok(())
    }

    async fn bridge_handler(
        ws: WebSocketUpgrade,
        State(bridge): State<Arc<Self>>,
    ) -> impl IntoResponse {
        ws.on_upgrade(|socket| Self::handle_bridge_socket(socket, bridge))
    }

    async fn handle_bridge_socket(mut socket: WebSocket, bridge: Arc<Self>) {
        let mut event_rx = bridge.runtime.subscribe_events();

        loop {
            tokio::select! {
                // Handle incoming WebSocket messages
                msg = socket.recv() => {
                    let Some(msg) = msg else {
                        break;
                    };

                    let Ok(Message::Text(raw)) = msg else {
                        continue;
                    };

                    // Try parsing as a request first
                    if let Ok(bridge_msg) = serde_json::from_str::<BridgeMessage>(&raw) {
                        match bridge_msg {
                            BridgeMessage::Request { request, request_id } => {
                                let response = bridge.handle_request(request).await;
                                let response = match response {
                                    Ok(r) => r,
                                    Err(err) => StorageResponse::Err { error: err },
                                };

                                let payload = serde_json::to_string(&BridgeMessage::Response {
                                    response,
                                    request_id,
                                })
                                .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"serialization failed\"}".to_string());

                                if socket.send(Message::Text(payload.into())).await.is_err() {
                                    break;
                                }
                            }
                            BridgeMessage::Response { .. } => {
                                // Unexpected response from client, ignore
                            }
                            BridgeMessage::Event(_) => {
                                // Client should not send events, ignore
                            }
                        }
                    }
                }
                // Stream events to client
                Ok(event) = event_rx.recv() => {
                    let payload = match serde_json::to_string(&BridgeMessage::Event(event)) {
                        Ok(p) => p,
                        Err(_) => continue,
                    };

                    if socket.send(Message::Text(payload.into())).await.is_err() {
                        break;
                    }
                }
            }
        }
    }
}

pub type StorageBridgeState = tokio::sync::Mutex<StorageBridge>;
