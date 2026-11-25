// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::{Context as _, Result};
use aptos_config::config::ApiConfig;
use futures::{ready, stream, FutureExt};
use rustls::{
    pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer},
    ServerConfig,
};
use std::{
    convert::Infallible,
    io,
    net::SocketAddr,
    path::Path,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::{TcpListener, TcpStream},
};
use tokio_rustls::TlsAcceptor;
use warp::{Filter, Reply};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebServer {
    pub address: SocketAddr,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
}

impl From<ApiConfig> for WebServer {
    fn from(cfg: ApiConfig) -> Self {
        Self::new(cfg.address, cfg.tls_cert_path, cfg.tls_key_path)
    }
}

impl WebServer {
    pub fn new(
        address: SocketAddr,
        tls_cert_path: Option<String>,
        tls_key_path: Option<String>,
    ) -> Self {
        Self {
            address,
            tls_cert_path,
            tls_key_path,
        }
    }

    pub async fn serve<F>(&self, routes: F)
    where
        F: Filter<Error = Infallible> + Clone + Sync + Send + 'static,
        F::Extract: Reply,
    {
        match &self.tls_cert_path {
            None => warp::serve(routes).bind(self.address).await,
            Some(cert_path) => {
                let key_path = self
                    .tls_key_path
                    .as_deref()
                    .expect("tls_key_path is required when tls_cert_path is set");
                let tls_config = build_tls_server_config(cert_path, key_path)
                    .expect("failed to build TLS server config");
                let listener = TcpListener::bind(self.address)
                    .await
                    .expect("failed to bind TLS TCP listener");
                let acceptor = TlsAcceptor::from(Arc::new(tls_config));
                let incoming = stream::unfold((listener, acceptor), |(listener, acceptor)| async {
                    let res = listener
                        .accept()
                        .await
                        .map(|(stream, _addr)| LazyTlsStream::new(stream, &acceptor));
                    Some((res, (listener, acceptor)))
                });
                warp::serve(routes).serve_incoming(incoming).await;
            },
        }
    }
}

fn build_tls_server_config(
    cert_path: impl AsRef<Path>,
    key_path: impl AsRef<Path>,
) -> Result<ServerConfig> {
    let certs: Vec<CertificateDer<'static>> = CertificateDer::pem_file_iter(cert_path.as_ref())
        .with_context(|| format!("failed to open TLS certificate {:?}", cert_path.as_ref()))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to parse TLS certificate PEM")?;
    let key = PrivateKeyDer::from_pem_file(key_path.as_ref())
        .with_context(|| format!("failed to parse TLS private key {:?}", key_path.as_ref()))?;

    let mut config =
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .context("failed to configure TLS protocol versions")?
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .context("invalid TLS certificate or private key")?;
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(config)
}

enum TlsState {
    Handshaking(tokio_rustls::Accept<TcpStream>),
    Streaming(tokio_rustls::server::TlsStream<TcpStream>),
}

struct LazyTlsStream {
    state: TlsState,
}

impl LazyTlsStream {
    fn new(stream: TcpStream, acceptor: &TlsAcceptor) -> Self {
        Self {
            state: TlsState::Handshaking(acceptor.accept(stream)),
        }
    }
}

impl AsyncRead for LazyTlsStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        match &mut this.state {
            TlsState::Handshaking(accept) => match ready!(accept.poll_unpin(cx)) {
                Ok(mut stream) => {
                    let poll = Pin::new(&mut stream).poll_read(cx, buf);
                    this.state = TlsState::Streaming(stream);
                    poll
                },
                Err(err) => Poll::Ready(Err(err)),
            },
            TlsState::Streaming(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for LazyTlsStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        match &mut this.state {
            TlsState::Handshaking(accept) => match ready!(accept.poll_unpin(cx)) {
                Ok(mut stream) => {
                    let poll = Pin::new(&mut stream).poll_write(cx, buf);
                    this.state = TlsState::Streaming(stream);
                    poll
                },
                Err(err) => Poll::Ready(Err(err)),
            },
            TlsState::Streaming(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut self.get_mut().state {
            TlsState::Handshaking(_) => Poll::Ready(Ok(())),
            TlsState::Streaming(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut self.get_mut().state {
            TlsState::Handshaking(_) => Poll::Ready(Ok(())),
            TlsState::Streaming(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustls::{pki_types::ServerName, ClientConfig, RootCertStore};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio_rustls::TlsConnector;

    const TEST_CERT_PEM: &str = "-----BEGIN CERTIFICATE-----\n\
MIIBMDCB16ADAgECAgEBMAoGCCqGSM49BAMCMBQxEjAQBgNVBAMMCWxvY2FsaG9z\n\
dDAgFw0yMDAxMDEwMDAwMDBaGA8yMTIwMDEwMTAwMDAwMFowFDESMBAGA1UEAwwJ\n\
bG9jYWxob3N0MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAE+YMhYAY1WKXhZKOi\n\
aPgYx2DIWTdufeZy4YPJru05/Rup1ydx0860Z+ePIxiRvAN04CdTVYzDkq9NlUvz\n\
Qb17zKMYMBYwFAYDVR0RBA0wC4IJbG9jYWxob3N0MAoGCCqGSM49BAMCA0gAMEUC\n\
IG0Ix+sNHqgRuU7UzzlJgWp6ob67M4l9m8VlAZ+0L5xxAiEA+U1G1GAOQ2L0WN/x\n\
FWthD3mCFqMXDKpSou8V8lGKyQQ=\n\
-----END CERTIFICATE-----\n";

    const TEST_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----\n\
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgTLdI94joKIg5H/wd\n\
d7NgMrdPo7NfQNJ+Xp9bGD61OyehRANCAAT5gyFgBjVYpeFko6Jo+BjHYMhZN259\n\
5nLhg8mu7Tn9G6nXJ3HTzrRn548jGJG8A3TgJ1NVjMOSr02VS/NBvXvM\n\
-----END PRIVATE KEY-----\n";

    #[tokio::test]
    async fn test_tls_webserver_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cert_path = dir.path().join("cert.pem");
        let key_path = dir.path().join("key.pem");
        std::fs::write(&cert_path, TEST_CERT_PEM).expect("write cert");
        std::fs::write(&key_path, TEST_KEY_PEM).expect("write key");

        let tls_config = build_tls_server_config(&cert_path, &key_path).expect("server tls config");
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local_addr");
        let acceptor = TlsAcceptor::from(Arc::new(tls_config));
        let incoming = stream::unfold((listener, acceptor), |(listener, acceptor)| async {
            let res = listener
                .accept()
                .await
                .map(|(stream, _addr)| LazyTlsStream::new(stream, &acceptor));
            Some((res, (listener, acceptor)))
        });

        let routes = warp::path("ping").map(|| "pong");
        let server_handle = tokio::spawn(warp::serve(routes).serve_incoming(incoming));

        let mut root_store = RootCertStore::empty();
        for cert in CertificateDer::pem_slice_iter(TEST_CERT_PEM.as_bytes()) {
            root_store
                .add(cert.expect("parse root cert"))
                .expect("add root cert");
        }
        let client_config =
            ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_safe_default_protocol_versions()
                .expect("protocol versions")
                .with_root_certificates(root_store)
                .with_no_client_auth();

        let connector = TlsConnector::from(Arc::new(client_config));
        let tcp = TcpStream::connect(addr).await.expect("connect");
        let server_name = ServerName::try_from("localhost").expect("server name");
        let mut tls_stream = connector
            .connect(server_name, tcp)
            .await
            .expect("tls handshake");
        tls_stream
            .write_all(b"GET /ping HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .expect("write request");
        let mut response = String::new();
        tls_stream
            .read_to_string(&mut response)
            .await
            .expect("read response");
        assert!(
            response.contains("200 OK"),
            "unexpected response: {response}"
        );
        assert!(
            response.ends_with("pong"),
            "unexpected response body: {response}"
        );

        server_handle.abort();
    }
}
