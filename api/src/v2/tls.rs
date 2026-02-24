// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! TLS support for the v2 API server.
//!
//! When TLS is configured (cert + key paths set), the v2 server uses
//! `tokio-rustls` to terminate TLS and `hyper-util` auto-builder to
//! serve HTTP/1.1 or HTTP/2 based on ALPN negotiation.

use anyhow::{Context, Result};
use aptos_logger::{info, warn};
use axum::Router;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder as AutoBuilder,
};
use rustls::{Certificate, PrivateKey, ServerConfig};
use std::{
    io::BufReader,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{net::TcpListener, sync::watch};
use tokio_rustls::TlsAcceptor;
use tower::Service;

/// Build a `TlsAcceptor` from PEM-encoded certificate and key files.
///
/// The certificate file may contain a chain (leaf + intermediates).
/// ALPN is configured to prefer `h2` then fall back to `http/1.1`.
pub fn build_tls_acceptor(cert_path: &str, key_path: &str) -> Result<TlsAcceptor> {
    // ---- Load certificates ----
    let cert_file = std::fs::File::open(cert_path)
        .with_context(|| format!("Failed to open TLS cert file: {}", cert_path))?;
    let mut cert_reader = BufReader::new(cert_file);
    let certs: Vec<Certificate> = rustls_pemfile::certs(&mut cert_reader)
        .with_context(|| format!("Failed to parse PEM certs from: {}", cert_path))?
        .into_iter()
        .map(Certificate)
        .collect();

    if certs.is_empty() {
        anyhow::bail!("No certificates found in {}", cert_path);
    }

    // ---- Load private key ----
    // Try PKCS8 first, then RSA, then EC.
    let key = read_private_key(key_path)?;

    // ---- Build ServerConfig ----
    let mut config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .context("Failed to build TLS ServerConfig")?;

    // ALPN: prefer h2 for multiplexed HTTP/2, fall back to HTTP/1.1.
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    info!(
        "TLS configured for v2 API (cert: {}, key: {})",
        cert_path, key_path
    );

    Ok(TlsAcceptor::from(Arc::new(config)))
}

/// Try to read a private key from a PEM file. Supports PKCS8, RSA, and EC keys.
fn read_private_key(path: &str) -> Result<PrivateKey> {
    // Read all PEM items and find the first key.
    let mut keys_pkcs8 = Vec::new();
    let mut keys_rsa = Vec::new();
    let mut keys_ec = Vec::new();

    // We need to re-read the file for each key type since rustls-pemfile 1.x
    // consumes the reader. Clone the content first.
    let key_data =
        std::fs::read(path).with_context(|| format!("Failed to read key file: {}", path))?;

    // Try PKCS8
    let mut reader = BufReader::new(key_data.as_slice());
    if let Ok(keys) = rustls_pemfile::pkcs8_private_keys(&mut reader) {
        keys_pkcs8 = keys;
    }

    // Try RSA
    let mut reader = BufReader::new(key_data.as_slice());
    if let Ok(keys) = rustls_pemfile::rsa_private_keys(&mut reader) {
        keys_rsa = keys;
    }

    // Try EC
    let mut reader = BufReader::new(key_data.as_slice());
    if let Ok(keys) = rustls_pemfile::ec_private_keys(&mut reader) {
        keys_ec = keys;
    }

    // Prefer PKCS8 > RSA > EC
    if let Some(key) = keys_pkcs8.into_iter().next() {
        return Ok(PrivateKey(key));
    }
    if let Some(key) = keys_rsa.into_iter().next() {
        return Ok(PrivateKey(key));
    }
    if let Some(key) = keys_ec.into_iter().next() {
        return Ok(PrivateKey(key));
    }

    anyhow::bail!(
        "No private key found in {} (tried PKCS8, RSA, EC formats)",
        path
    )
}

/// Serve an Axum router over TLS using the given TcpListener and TlsAcceptor.
///
/// This is a replacement for `axum::serve` that wraps each accepted TCP stream
/// in a TLS layer before handing it to `hyper_util::server::conn::auto::Builder`,
/// which auto-negotiates HTTP/1.1 vs HTTP/2 based on ALPN.
///
/// Supports graceful shutdown: when `shutdown_rx` receives `true`, the server
/// stops accepting new connections and waits for in-flight connections to
/// complete (up to `drain_timeout_ms`). Individual connection tasks continue
/// to serve their in-flight requests naturally.
///
/// The `port_tx` uses `futures::channel::oneshot` for compatibility with the
/// existing runtime bootstrap infrastructure.
pub async fn serve_tls(
    listener: TcpListener,
    tls_acceptor: TlsAcceptor,
    app: Router,
    port_tx: Option<futures::channel::oneshot::Sender<u16>>,
    shutdown_rx: watch::Receiver<bool>,
    drain_timeout_ms: u64,
) {
    let local_addr = listener.local_addr().expect("Failed to get local addr");

    if let Some(tx) = port_tx {
        let _ = tx.send(local_addr.port());
    }

    info!("v2 API TLS server listening on {}", local_addr);

    let active_connections = Arc::new(AtomicUsize::new(0));
    let mut accept_shutdown_rx = shutdown_rx.clone();

    // Accept loop: race between new connections and shutdown signal.
    loop {
        tokio::select! {
            result = listener.accept() => {
                let (tcp_stream, remote_addr) = match result {
                    Ok(conn) => conn,
                    Err(e) => {
                        warn!("Failed to accept TCP connection: {}", e);
                        continue;
                    },
                };

                let tls_acceptor = tls_acceptor.clone();
                let app = app.clone();
                let active = active_connections.clone();
                active.fetch_add(1, Ordering::Relaxed);

                tokio::spawn(async move {
                    // Perform TLS handshake.
                    let tls_stream = match tls_acceptor.accept(tcp_stream).await {
                        Ok(stream) => stream,
                        Err(e) => {
                            warn!("TLS handshake failed from {}: {}", remote_addr, e);
                            active.fetch_sub(1, Ordering::Relaxed);
                            return;
                        },
                    };

                    // Wrap in TokioIo for hyper compatibility.
                    let io = TokioIo::new(tls_stream);

                    let hyper_service = hyper1::service::service_fn(move |req| {
                        let mut app = app.clone();
                        async move { app.call(req).await }
                    });

                    // Auto-builder negotiates HTTP/1.1 vs HTTP/2 based on ALPN.
                    let builder = AutoBuilder::new(TokioExecutor::new());
                    if let Err(e) = builder.serve_connection(io, hyper_service).await {
                        warn!("Connection error from {}: {}", remote_addr, e);
                    }

                    active.fetch_sub(1, Ordering::Relaxed);
                });
            }
            _ = accept_shutdown_rx.wait_for(|&v| v) => {
                info!("[v2] TLS server received shutdown signal, stopping accept loop");
                break;
            }
        }
    }

    // Drain phase: wait for in-flight connections to complete.
    let remaining = active_connections.load(Ordering::Relaxed);
    if remaining > 0 {
        info!(
            "[v2] Draining {} active TLS connections (timeout: {}ms)...",
            remaining, drain_timeout_ms
        );

        if drain_timeout_ms == 0 {
            info!("[v2] Drain timeout = 0, shutting down immediately");
            return;
        }

        let deadline = tokio::time::Instant::now() + Duration::from_millis(drain_timeout_ms);
        loop {
            let count = active_connections.load(Ordering::Relaxed);
            if count == 0 {
                info!("[v2] All TLS connections drained successfully");
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                warn!(
                    "[v2] TLS drain timeout ({}ms) exceeded, {} connections remaining",
                    drain_timeout_ms, count
                );
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    } else {
        info!("[v2] TLS server shut down (no active connections)");
    }
}
