// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_config::config::ApiConfig;
use axum::Router;
use std::net::SocketAddr;
use tokio::net::TcpListener;

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

    pub async fn serve(&self, routes: Router) {
        assert!(
            self.tls_cert_path.is_none() && self.tls_key_path.is_none(),
            "TLS for aptos-warp-webserver is not yet supported with axum",
        );
        let listener = TcpListener::bind(self.address)
            .await
            .expect("Failed to bind aptos webserver listener");
        axum::serve(listener, routes)
            .await
            .expect("aptos webserver terminated unexpectedly");
    }
}
