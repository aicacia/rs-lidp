use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::{
    Router,
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
};
use storage_iroh::{StorageIrohRuntime, StorageRequest, StorageResponse};

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
            .route("/request", get(Self::request_handler))
            .route("/events", get(Self::event_handler))
            .with_state(state);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:3042")
            .await
            .map_err(|err| err.to_string())?;

        axum::serve(listener, app)
            .await
            .map_err(|err| err.to_string())?;

        Ok(())
    }

    async fn request_handler(
        ws: WebSocketUpgrade,
        State(bridge): State<Arc<Self>>,
    ) -> impl IntoResponse {
        ws.on_upgrade(|socket| Self::handle_request_socket(socket, bridge))
    }

    async fn event_handler(
        ws: WebSocketUpgrade,
        State(bridge): State<Arc<Self>>,
    ) -> impl IntoResponse {
        ws.on_upgrade(|socket| Self::handle_event_socket(socket, bridge))
    }

    async fn handle_request_socket(mut socket: WebSocket, bridge: Arc<Self>) {
        while let Some(msg) = socket.recv().await {
            let Ok(Message::Text(raw)) = msg else {
                continue;
            };

            let request = match serde_json::from_str::<StorageRequest>(&raw) {
                Ok(request) => request,
                Err(err) => {
                    let payload = serde_json::to_string(&StorageResponse::Err {
                        error: format!("invalid request: {err}"),
                    })
                    .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"invalid request\"}".to_string());

                    let _ = socket.send(Message::Text(payload.into())).await;
                    continue;
                }
            };

            let response = bridge.handle_request(request).await;
            let payload = match response {
                Ok(response) => serde_json::to_string(&response).unwrap_or_else(|_| {
                    "{\"ok\":false,\"error\":\"response serialization failed\"}".to_string()
                }),
                Err(err) => serde_json::to_string(&StorageResponse::Err { error: err })
                    .unwrap_or_else(|_| {
                        "{\"ok\":false,\"error\":\"response serialization failed\"}".to_string()
                    }),
            };

            if socket.send(Message::Text(payload.into())).await.is_err() {
                break;
            }
        }
    }

    async fn handle_event_socket(mut socket: WebSocket, bridge: Arc<Self>) {
        let mut rx = bridge.runtime.subscribe_events();

        loop {
            let Ok(event) = rx.recv().await else {
                break;
            };

            let payload = match serde_json::to_string(&event) {
                Ok(value) => value,
                Err(_) => continue,
            };

            if socket.send(Message::Text(payload.into())).await.is_err() {
                break;
            }
        }
    }
}

pub type StorageBridgeState = tokio::sync::Mutex<StorageBridge>;
