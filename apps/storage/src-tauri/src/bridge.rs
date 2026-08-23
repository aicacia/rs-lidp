use std::{
    fs, io,
    net::Ipv4Addr,
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::{
    Router,
    extract::ws::{Message, WebSocket},
    extract::{State, WebSocketUpgrade},
    response::{Html, IntoResponse},
    routing::get,
};
use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, IsCa, Issuer, KeyPair, SanType,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use serde::{Deserialize, Serialize};
use storage_iroh::{StorageEvent, StorageIrohRuntime, StorageRequest, StorageResponse};
use storage_service::StorageService;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use crate::bridge_trust::install_ca_to_user_trust_store;

pub const STORAGE_BRIDGE_HOST: &str = "storage.localhost";

const TRUST_PAGE: &str = "<!DOCTYPE html>\
<html lang=\"en\">\
<head>\
<meta charset=\"utf-8\">\
<title>Storage Bridge Certificate</title>\
</head>\
<body>\
<h1>Storage bridge certificate</h1>\
<p>If your browser shows a certificate warning, accept it to allow local web apps to connect.</p>\
<p>You can close this tab after the page loads without warnings.</p>\
</body>\
</html>";

pub fn bridge_cert_path(data_dir: &Path) -> PathBuf {
    data_dir.join("storage-bridge-ca.pem")
}

fn ca_key_path(data_dir: &Path) -> PathBuf {
    data_dir.join("storage-bridge-ca.key")
}

pub fn bridge_url(host: &str, port: u16) -> String {
    format!("wss://{host}:{port}")
}

pub fn bridge_trust_url(wss_url: &str) -> Option<String> {
    wss_url
        .strip_prefix("wss://")
        .map(|rest| format!("https://{rest}/trust"))
}

fn host_file_suffix(host: &str) -> String {
    host.replace('.', "-")
}

fn server_cert_paths(data_dir: &Path, host: &str) -> (PathBuf, PathBuf, PathBuf) {
    let suffix = host_file_suffix(host);
    (
        data_dir.join(format!("storage-bridge-server-{suffix}.der")),
        data_dir.join(format!("storage-bridge-server-{suffix}.key")),
        data_dir.join(format!("storage-bridge-server-{suffix}.pem")),
    )
}

pub async fn ensure_storage_bridge_certificate(data_dir: &Path) -> io::Result<PathBuf> {
    let cert_path = bridge_cert_path(data_dir);
    let ca = load_or_create_ca(data_dir).await?;
    let ca_pem = ensure_ca_certificate_pem(data_dir, &ca).await?;
    fs::write(&cert_path, ca_pem.as_bytes())?;
    let _server_cert =
        load_or_create_server_cert(STORAGE_BRIDGE_HOST, data_dir, &ca).await?;
    let _ = install_ca_to_user_trust_store(data_dir, &cert_path);

    Ok(cert_path)
}

#[derive(Clone)]
struct CertificateFiles {
    cert_der: Vec<u8>,
    key_der: Vec<u8>,
    cert_pem: String,
}

async fn load_or_create_ca(data_dir: &Path) -> io::Result<KeyPair> {
    let key_path = ca_key_path(data_dir);

    if fs::exists(&key_path)? {
        log::debug!("Loading existing CA key from {:?}", key_path);
        let key = fs::read_to_string(&key_path)?;
        return Ok(KeyPair::from_pem(&key).map_err(io::Error::other)?);
    }

    let key = KeyPair::generate().map_err(io::Error::other)?;
    log::debug!("Generated new CA key and saving to {:?}", key_path);
    fs::write(&key_path, key.serialize_pem())?;

    Ok(key)
}

async fn ensure_ca_certificate_pem(data_dir: &Path, ca_key: &KeyPair) -> io::Result<String> {
    let ca_pem_path = bridge_cert_path(data_dir);
    let ca_der_path = ca_der_path(data_dir);

    if fs::exists(&ca_pem_path)? && fs::exists(&ca_der_path)? {
        return fs::read_to_string(&ca_pem_path);
    }

    let ca_cert = generate_ca_certificate(ca_key)?;
    let ca_pem = ca_cert.pem();
    let ca_der = ca_cert.der().to_vec();
    fs::write(&ca_pem_path, ca_pem.as_bytes())?;
    fs::write(&ca_der_path, &ca_der)?;
    Ok(ca_pem)
}

fn ca_der_path(data_dir: &Path) -> PathBuf {
    data_dir.join("storage-bridge-ca.der")
}

fn generate_ca_certificate(ca_key: &KeyPair) -> io::Result<rcgen::Certificate> {
    let mut ca_params = CertificateParams::new(vec![]).map_err(io::Error::other)?;
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params.distinguished_name = DistinguishedName::new();
    ca_params
        .distinguished_name
        .push(DnType::CommonName, "Local Storage Bridge CA");

    ca_params.self_signed(ca_key).map_err(io::Error::other)
}

async fn load_or_create_server_cert(
    host: &str,
    data_dir: &Path,
    ca_key: &KeyPair,
) -> io::Result<CertificateFiles> {
    let (cert_path, key_path, cert_pem_path) = server_cert_paths(data_dir, host);

    if fs::exists(&cert_path)? && fs::exists(&key_path)? && fs::exists(&cert_pem_path)? {
        log::debug!(
            "Loading existing server certificate and key from {:?} and {:?}",
            cert_path,
            key_path
        );
        return Ok(CertificateFiles {
            cert_der: fs::read(&cert_path)?,
            key_der: fs::read(&key_path)?,
            cert_pem: fs::read_to_string(&cert_pem_path)?,
        });
    }

    let mut params = CertificateParams::new(vec![host.to_string(), "localhost".to_string()])
        .map_err(io::Error::other)?;
    params
        .subject_alt_names
        .push(SanType::IpAddress(Ipv4Addr::LOCALHOST.into()));
    params.distinguished_name = DistinguishedName::new();
    params.distinguished_name.push(DnType::CommonName, host);
    params.is_ca = IsCa::NoCa;

    let key = KeyPair::generate().map_err(io::Error::other)?;

    let ca_params = {
        let mut params = CertificateParams::new(vec![]).map_err(io::Error::other)?;
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.distinguished_name = DistinguishedName::new();
        params
            .distinguished_name
            .push(DnType::CommonName, "Local Storage Bridge CA");
        params
    };

    let ca_issuer = Issuer::new(ca_params, ca_key);
    let cert = params
        .signed_by(&key, &ca_issuer)
        .map_err(io::Error::other)?;

    let cert_der = cert.der().to_vec();
    let key_der = key.serialize_der();
    let cert_pem = cert.pem();

    log::debug!(
        "Generated new server certificate and key, saving to {:?} and {:?}",
        cert_path,
        key_path
    );
    fs::write(&cert_path, &cert_der)?;
    fs::write(&key_path, &key_der)?;
    fs::write(cert_pem_path, cert_pem.as_bytes())?;

    Ok(CertificateFiles {
        cert_der,
        key_der,
        cert_pem,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

struct TlsListener {
    inner: TcpListener,
    acceptor: TlsAcceptor,
}

impl axum::serve::Listener for TlsListener {
    type Io = tokio_rustls::server::TlsStream<tokio::net::TcpStream>;
    type Addr = std::net::SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            match self.inner.accept().await {
                Ok((stream, addr)) => match self.acceptor.clone().accept(stream).await {
                    Ok(tls_stream) => return (tls_stream, addr),
                    Err(err) => {
                        log::warn!("storage bridge TLS accept failed: {err}");
                        continue;
                    }
                },
                Err(err) => {
                    log::warn!("storage bridge TCP accept failed: {err}");
                    continue;
                }
            }
        }
    }

    fn local_addr(&self) -> io::Result<Self::Addr> {
        self.inner.local_addr()
    }
}

#[derive(Debug, Clone)]
pub struct StorageBridge {
    runtime: StorageIrohRuntime,
    storage: Arc<StorageService>,
    url: Arc<tokio::sync::Mutex<String>>,
}

impl StorageBridge {
    pub fn new(files_dir: PathBuf) -> Self {
        Self {
            runtime: StorageIrohRuntime::new(),
            storage: Arc::new(StorageService::new(files_dir)),
            url: Arc::new(tokio::sync::Mutex::new(String::new())),
        }
    }

    pub async fn url(&self) -> String {
        self.url.lock().await.clone()
    }

    pub async fn handle_request(&self, request: StorageRequest) -> Result<StorageResponse, String> {
        match request {
            StorageRequest::ReadFile { path } => {
                let bytes = self
                    .storage
                    .read_file(&path)
                    .await
                    .map_err(|e| e.to_string())?;
                let content = String::from_utf8(bytes).map_err(|e| e.to_string())?;
                Ok(StorageResponse::success_payload(content))
            }
            StorageRequest::WriteFile { path, content } => {
                self.storage
                    .write_file(&path, content.as_bytes())
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(StorageResponse::success_empty())
            }
            other_request => {
                let event = match other_request {
                    StorageRequest::AddDevice { device_id } => {
                        self.runtime.add_device(&device_id).await?
                    }
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
                    _ => unreachable!(),
                };

                Ok(StorageResponse::success_event(event))
            }
        }
    }

    fn build_server_config(data_dir: &Path) -> Result<Arc<rustls::ServerConfig>, String> {
        let (cert_path, key_path, _) = server_cert_paths(data_dir, STORAGE_BRIDGE_HOST);
        let cert_der = fs::read(&cert_path).map_err(|err| err.to_string())?;
        let key_der = fs::read(&key_path).map_err(|err| err.to_string())?;
        let ca_der = fs::read(ca_der_path(data_dir)).map_err(|err| err.to_string())?;

        let server_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                vec![CertificateDer::from(cert_der), CertificateDer::from(ca_der)],
                PrivateKeyDer::from(PrivatePkcs8KeyDer::from(key_der)),
            )
            .map_err(|err| err.to_string())?;

        Ok(Arc::new(server_config))
    }

    pub async fn start_server(self, data_dir: &Path) -> Result<String, String> {
        let state = Arc::new(self.clone());

        let app = Router::new()
            .route("/", get(Self::bridge_handler))
            .route("/trust", get(Self::trust_page_handler))
            .with_state(state);

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|err| err.to_string())?;

        let port = listener.local_addr().map_err(|err| err.to_string())?.port();

        let bridge_url = bridge_url(STORAGE_BRIDGE_HOST, port);
        {
            let mut current = self.url.lock().await;
            *current = bridge_url.clone();
        }

        let tls_listener = TlsListener {
            inner: listener,
            acceptor: TlsAcceptor::from(Self::build_server_config(data_dir)?),
        };

        axum::serve(tls_listener, app)
            .await
            .map_err(|err| err.to_string())?;

        Ok(bridge_url)
    }

    async fn bridge_handler(
        ws: WebSocketUpgrade,
        State(bridge): State<Arc<Self>>,
    ) -> impl IntoResponse {
        ws.on_upgrade(|socket| Self::handle_bridge_socket(socket, bridge))
    }

    async fn trust_page_handler() -> Html<&'static str> {
        Html(TRUST_PAGE)
    }

    async fn handle_bridge_socket(mut socket: WebSocket, bridge: Arc<Self>) {
        let mut event_rx = bridge.runtime.subscribe_events();

        loop {
            tokio::select! {
                msg = socket.recv() => {
                    let Some(msg) = msg else {
                        break;
                    };

                    let Ok(Message::Text(raw)) = msg else {
                        continue;
                    };

                    if let Ok(bridge_msg) = serde_json::from_str::<BridgeMessage>(&raw) {
                        match bridge_msg {
                            BridgeMessage::Request { request, request_id } => {
                                let response = bridge.handle_request(request).await;
                                let response = match response {
                                    Ok(r) => r,
                                    Err(err) => StorageResponse::error(err),
                                };

                                let payload = match serde_json::to_string(&BridgeMessage::Response {
                                    response,
                                    request_id,
                                }) {
                                    Ok(payload) => payload,
                                    Err(_) => {
                                        let fallback = "{\"ok\":false,\"error\":\"serialization failed\"}".to_string();
                                        if socket
                                            .send(Message::Text(fallback.into()))
                                            .await
                                            .is_err()
                                        {
                                            break;
                                        }
                                        continue;
                                    }
                                };

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

pub fn bridge_trust_prompted_path(data_dir: &Path) -> PathBuf {
    data_dir.join("bridge-trust-prompted")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_url_uses_host_and_port() {
        assert_eq!(
            bridge_url(STORAGE_BRIDGE_HOST, 33233),
            "wss://storage.localhost:33233"
        );
    }

    #[test]
    fn bridge_trust_url_converts_wss_to_https() {
        assert_eq!(
            bridge_trust_url("wss://storage.localhost:33233"),
            Some("https://storage.localhost:33233/trust".to_string())
        );
    }

    #[test]
    fn server_cert_paths_include_host_suffix() {
        let dir = Path::new("/tmp/storage-data");
        let (cert, key, pem) = server_cert_paths(dir, STORAGE_BRIDGE_HOST);
        assert_eq!(
            cert,
            dir.join("storage-bridge-server-storage-localhost.der")
        );
        assert_eq!(
            key,
            dir.join("storage-bridge-server-storage-localhost.key")
        );
        assert_eq!(
            pem,
            dir.join("storage-bridge-server-storage-localhost.pem")
        );
    }
}
