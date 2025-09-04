// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_config::config::ApiConfig;
use std::{convert::Infallible, net::SocketAddr};
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
                warp::serve(routes)
                    .tls()
                    .cert_path(cert_path)
                    .key_path(self.tls_key_path.as_ref().unwrap())
                    .bind(self.address)
                    .await
            },
        }
    }
}
