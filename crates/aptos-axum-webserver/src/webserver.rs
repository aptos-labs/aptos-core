// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_config::config::ApiConfig;
use axum::Router;
use std::net::SocketAddr;

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

    /// Serves the given fully-composed axum [`Router`] (state and layers already
    /// applied). Uses plaintext `axum::serve` unless a TLS certificate is
    /// configured, in which case `axum-server` with rustls is used.
    pub async fn serve(&self, router: Router) -> anyhow::Result<()> {
        match &self.tls_cert_path {
            None => {
                let listener = tokio::net::TcpListener::bind(self.address).await?;
                axum::serve(listener, router.into_make_service()).await?;
            },
            Some(cert_path) => {
                let key_path = self.tls_key_path.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("tls_key_path is required when tls_cert_path is set")
                })?;
                let config = axum_server::tls_rustls::RustlsConfig::from_pem_file(
                    cert_path.clone(),
                    key_path.clone(),
                )
                .await?;
                axum_server::bind_rustls(self.address, config)
                    .serve(router.into_make_service())
                    .await?;
            },
        }
        Ok(())
    }
}
