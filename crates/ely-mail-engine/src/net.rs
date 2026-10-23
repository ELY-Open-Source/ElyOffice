use std::{future::Future, sync::Arc, time::Duration};

use rustls::{ClientConfig, crypto::ring, pki_types::ServerName};
use rustls_platform_verifier::BuilderVerifierExt;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_rustls::{TlsConnector, client::TlsStream};

use crate::{Error, Result};

pub(crate) type Stream = TlsStream<TcpStream>;

const WAIT: Duration = Duration::from_secs(60);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Server {
    pub host: String,
    pub port: u16,
    pub security: Security,
}

/// No plaintext: every mode ends in TLS.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Security {
    Tls,
    StartTls,
}

#[derive(Clone)]
pub struct Login {
    pub user: String,
    pub secret: Secret,
}

#[derive(Clone)]
pub enum Secret {
    Password(String),
    /// An OAuth access token, sent as XOAUTH2.
    Token(String),
}

impl Login {
    /// The SASL XOAUTH2 initial response, before base64.
    pub(crate) fn xoauth2(&self, token: &str) -> String {
        format!("user={}\x01auth=Bearer {token}\x01\x01", self.user)
    }
}

/// Verifies servers with the system's trust store.
pub fn system_tls() -> Result<TlsConnector> {
    let config = ClientConfig::builder_with_provider(Arc::new(ring::default_provider()))
        .with_safe_default_protocol_versions()
        .and_then(|builder| builder.with_platform_verifier())
        .map_err(|err| Error::Tls(err.to_string()))?
        .with_no_client_auth();
    Ok(TlsConnector::from(Arc::new(config)))
}

pub(crate) async fn within<T>(step: &'static str, work: impl Future<Output = T>) -> Result<T> {
    tokio::time::timeout(WAIT, work)
        .await
        .map_err(|_| Error::Timeout(step))
}

pub(crate) async fn tcp(server: &Server) -> Result<TcpStream> {
    let host = server.host.as_str();
    let stream = match crate::proxy::for_host(host)? {
        Some(route) => within("connecting through the proxy", crate::proxy::tunnel(&route, host, server.port)).await??,
        None => within("connecting", TcpStream::connect((host, server.port)))
            .await?
            .map_err(Error::Connect)?,
    };
    log::info!("mail: connected to {}:{}", server.host, server.port);
    Ok(stream)
}

pub(crate) async fn tls(tcp: TcpStream, server: &Server, tls: &TlsConnector) -> Result<Stream> {
    let name = ServerName::try_from(server.host.clone())
        .map_err(|err| Error::Tls(format!("{}: {err}", server.host)))?;
    let stream = within("starting TLS", tls.connect(name, tcp))
        .await?
        .map_err(|err| Error::Tls(err.to_string()))?;
    log::info!("mail: TLS with {}:{}", server.host, server.port);
    Ok(stream)
}
