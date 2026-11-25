// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::{Context as _, Result};
use aptos_config::config::ApiConfig;
use aptos_logger::error;
use futures::{future, ready, FutureExt};
use hyper::{
    server::{
        accept::Accept,
        conn::{AddrIncoming, AddrStream},
    },
    service::{make_service_fn, service_fn, Service},
    Server as HyperServer,
};
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
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_rustls::TlsAcceptor;
use warp::{Filter, Reply};

tokio::task_local! {
    static TLS_REMOTE_ADDR: Option<SocketAddr>;
}

pub(crate) fn tls_remote_addr() -> Option<SocketAddr> {
    TLS_REMOTE_ADDR.try_with(|addr| *addr).ok().flatten()
}

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
                let addr_incoming =
                    AddrIncoming::bind(&self.address).expect("failed to bind TLS TCP listener");
                serve_tls(addr_incoming, tls_config, routes).await;
            },
        }
    }
}

async fn serve_tls<F>(incoming: AddrIncoming, tls_config: ServerConfig, routes: F)
where
    F: Filter<Error = Infallible> + Clone + Sync + Send + 'static,
    F::Extract: Reply,
{
    let inner = warp::service(routes);
    let make_svc = make_service_fn(move |conn: &LazyTlsStream| {
        let remote_addr = conn.remote_addr;
        let inner = inner.clone();
        future::ok::<_, Infallible>(service_fn(move |req| {
            let mut svc = inner.clone();
            TLS_REMOTE_ADDR.scope(remote_addr, async move { svc.call(req).await })
        }))
    });
    if let Err(err) = HyperServer::builder(TlsAccept::new(incoming, tls_config))
        .serve(make_svc)
        .await
    {
        error!("TLS webserver error: {}", err);
    }
}

struct TlsAccept {
    incoming: AddrIncoming,
    acceptor: TlsAcceptor,
}

impl TlsAccept {
    fn new(mut incoming: AddrIncoming, tls_config: ServerConfig) -> Self {
        incoming.set_nodelay(true);
        Self {
            incoming,
            acceptor: TlsAcceptor::from(Arc::new(tls_config)),
        }
    }
}

impl Accept for TlsAccept {
    type Conn = LazyTlsStream;
    type Error = io::Error;

    fn poll_accept(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<io::Result<Self::Conn>>> {
        let this = self.get_mut();
        match ready!(Pin::new(&mut this.incoming).poll_accept(cx)) {
            Some(Ok(sock)) => {
                let remote_addr = sock.remote_addr();
                Poll::Ready(Some(Ok(LazyTlsStream::new(
                    sock,
                    &this.acceptor,
                    Some(remote_addr),
                ))))
            },
            Some(Err(err)) => Poll::Ready(Some(Err(err))),
            None => Poll::Ready(None),
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

enum TlsState<S> {
    Handshaking(tokio_rustls::Accept<S>),
    Streaming(tokio_rustls::server::TlsStream<S>),
    Failed,
}

struct LazyTlsStream<S = AddrStream> {
    state: TlsState<S>,
    remote_addr: Option<SocketAddr>,
}

impl<S: AsyncRead + AsyncWrite + Unpin> LazyTlsStream<S> {
    fn new(stream: S, acceptor: &TlsAcceptor, remote_addr: Option<SocketAddr>) -> Self {
        Self {
            state: TlsState::Handshaking(acceptor.accept(stream)),
            remote_addr,
        }
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncRead for LazyTlsStream<S> {
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
                Err(err) => {
                    this.state = TlsState::Failed;
                    Poll::Ready(Err(err))
                },
            },
            TlsState::Streaming(stream) => Pin::new(stream).poll_read(cx, buf),
            TlsState::Failed => Poll::Ready(Err(io::Error::from(io::ErrorKind::BrokenPipe))),
        }
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncWrite for LazyTlsStream<S> {
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
                Err(err) => {
                    this.state = TlsState::Failed;
                    Poll::Ready(Err(err))
                },
            },
            TlsState::Streaming(stream) => Pin::new(stream).poll_write(cx, buf),
            TlsState::Failed => Poll::Ready(Err(io::Error::from(io::ErrorKind::BrokenPipe))),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut self.get_mut().state {
            TlsState::Handshaking(_) | TlsState::Failed => Poll::Ready(Ok(())),
            TlsState::Streaming(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut self.get_mut().state {
            TlsState::Handshaking(_) | TlsState::Failed => Poll::Ready(Ok(())),
            TlsState::Streaming(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustls::{pki_types::ServerName, ClientConfig, RootCertStore};
    use std::sync::Mutex;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
    };
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
        let addr_incoming = AddrIncoming::from_listener(listener).expect("addr_incoming");

        let logged_remote_addr: Arc<Mutex<Option<SocketAddr>>> = Arc::new(Mutex::new(None));
        let logged_remote_addr_clone = logged_remote_addr.clone();
        let routes = warp::path("ping")
            .map(|| "pong")
            .recover(|_: warp::Rejection| async {
                Ok::<_, Infallible>(warp::http::StatusCode::NOT_FOUND)
            })
            .with(crate::logger())
            .with(warp::log::custom(move |info| {
                *logged_remote_addr_clone.lock().expect("lock") =
                    info.remote_addr().or_else(tls_remote_addr);
            }));
        let server_handle = tokio::spawn(serve_tls(addr_incoming, tls_config, routes));

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
        let client_addr = tcp.local_addr().expect("client local_addr");
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
        assert_eq!(*logged_remote_addr.lock().expect("lock"), Some(client_addr));

        server_handle.abort();
    }

    #[tokio::test]
    async fn test_tls_handshake_error_repoll_does_not_panic() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cert_path = dir.path().join("cert.pem");
        let key_path = dir.path().join("key.pem");
        std::fs::write(&cert_path, TEST_CERT_PEM).expect("write cert");
        std::fs::write(&key_path, TEST_KEY_PEM).expect("write key");

        let tls_config = build_tls_server_config(&cert_path, &key_path).expect("server tls config");
        let acceptor = TlsAcceptor::from(Arc::new(tls_config));
        let (mut client_io, server_io) = tokio::io::duplex(1024);
        let mut lazy_tls = LazyTlsStream::new(server_io, &acceptor, None);

        client_io
            .write_all(b"GET /plain HTTP/1.1\r\n\r\n")
            .await
            .expect("write plaintext");
        drop(client_io);

        let mut buf = [0u8; 32];
        assert!(lazy_tls.read(&mut buf).await.is_err());
        assert_eq!(
            lazy_tls.read(&mut buf).await.unwrap_err().kind(),
            io::ErrorKind::BrokenPipe
        );
        assert_eq!(
            lazy_tls.write(b"x").await.unwrap_err().kind(),
            io::ErrorKind::BrokenPipe
        );
        lazy_tls.flush().await.expect("flush after error");
        lazy_tls.shutdown().await.expect("shutdown after error");
    }
}
