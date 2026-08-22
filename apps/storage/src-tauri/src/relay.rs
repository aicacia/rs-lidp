use std::{
    fs, io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::Path,
};

use iroh_relay::server::{CertConfig, RelayConfig, Server, ServerConfig, TlsConfig};
use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, IsCa, Issuer, KeyPair,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

pub struct LocalRelay {
    server: Server,
}

impl LocalRelay {
    pub async fn start(host: &str, data_dir: impl AsRef<Path>) -> io::Result<Self> {
        rustls::crypto::ring::default_provider()
            .install_default()
            .ok();

        let data_dir = data_dir.as_ref();
        fs::create_dir_all(data_dir)?;

        let ca = load_or_create_ca(data_dir).await?;
        let server_cert = load_or_create_server_cert(host, data_dir, &ca).await?;

        let server_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                vec![CertificateDer::from(server_cert.cert_der)],
                PrivateKeyDer::from(PrivatePkcs8KeyDer::from(server_cert.key_der)),
            )
            .map_err(io::Error::other)?;

        let mut relay = RelayConfig::new(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0));

        relay.tls = Some(TlsConfig::new(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
            CertConfig::Manual { server_config },
        ));

        let mut config = ServerConfig::default();
        config.relay = Some(relay);
        config.quic = None;

        let server = Server::spawn(config).await.map_err(io::Error::other)?;

        Ok(Self { server })
    }

    pub fn url(&self) -> io::Result<String> {
        let relay_addr = self
            .server
            .https_addr()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Relay address not available"))?;

        Ok(format!("https://{}", relay_addr))
    }

    pub async fn shutdown(self) -> io::Result<()> {
        self.server.shutdown().await.map_err(io::Error::other)?;
        Ok(())
    }
}

#[derive(Clone)]
struct CertificateFiles {
    cert_der: Vec<u8>,
    key_der: Vec<u8>,
}

async fn load_or_create_ca(data_dir: &Path) -> io::Result<KeyPair> {
    let key_path = data_dir.join("relay-ca.key");

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

async fn load_or_create_server_cert(
    host: &str,
    data_dir: &Path,
    ca_key: &KeyPair,
) -> io::Result<CertificateFiles> {
    let cert_path = data_dir.join("relay-server.der");
    let key_path = data_dir.join("relay-server.key");

    if fs::exists(&cert_path)? && fs::exists(&key_path)? {
        log::debug!(
            "Loading existing server certificate and key from {:?} and {:?}",
            cert_path,
            key_path
        );
        return Ok(CertificateFiles {
            cert_der: fs::read(cert_path)?,
            key_der: fs::read(key_path)?,
        });
    }

    let mut params = CertificateParams::new(vec![host.to_string()]).map_err(io::Error::other)?;

    params
        .subject_alt_names
        .push(rcgen::SanType::IpAddress(Ipv4Addr::LOCALHOST.into()));

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
            .push(DnType::CommonName, "Local Relay CA");
        params
    };

    let ca_issuer = Issuer::new(ca_params, ca_key);
    let cert = params
        .signed_by(&key, &ca_issuer)
        .map_err(io::Error::other)?;

    let cert_der = cert.der().to_vec();
    let key_der = key.serialize_der();

    log::debug!(
        "Generated new server certificate and key, saving to {:?} and {:?}",
        cert_path,
        key_path
    );

    fs::write(cert_path, &cert_der)?;
    fs::write(key_path, &key_der)?;

    Ok(CertificateFiles { cert_der, key_der })
}
