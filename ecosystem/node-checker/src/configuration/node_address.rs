// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, bail, Context, Result};
use velor_crypto::x25519;
use velor_rest_client::Client as VelorRestClient;
use velor_sdk::types::network_address::NetworkAddress;
use reqwest::cookie::Jar;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use url::Url;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NodeAddress {
    /// Target URL. This should include a scheme (e.g. http://). If there is no
    /// scheme, we will prepend http://.
    pub url: Url,

    /// API port.
    api_port: Option<u16>,

    /// Metrics port.
    metrics_port: Option<u16>,

    /// Validator communication port.
    noise_port: Option<u16>,

    /// Public key for the node. This is used for the HandshakeChecker.
    /// If that Checker is not enabled, this is not necessary.
    public_key: Option<x25519::PublicKey>,

    // Cookie store.
    #[serde(skip)]
    cookie_store: Arc<Jar>,
}

impl NodeAddress {
    pub fn new(
        url: Url,
        api_port: Option<u16>,
        metrics_port: Option<u16>,
        noise_port: Option<u16>,
        public_key: Option<x25519::PublicKey>,
    ) -> Self {
        Self {
            url,
            api_port,
            metrics_port,
            noise_port,
            public_key,
            cookie_store: Arc::new(Jar::default()),
        }
    }

    /// Do not use this to build a client, use get_metrics_client.
    pub fn get_metrics_port(&self) -> Option<u16> {
        self.metrics_port
    }

    /// Do not use this to build a client, use get_api_client.
    pub fn get_api_port(&self) -> Option<u16> {
        self.api_port
    }

    pub fn get_noise_port(&self) -> Option<u16> {
        self.noise_port
    }

    pub fn get_public_key(&self) -> Option<x25519::PublicKey> {
        self.public_key
    }

    pub fn get_api_url(&self) -> Result<Url> {
        let mut url = self.url.clone();
        url.set_port(Some(
            self.api_port
                .context("Can't build API URL without an API port")?,
        ))
        .unwrap();
        Ok(url)
    }

    pub fn get_metrics_url(&self, path: &str) -> Result<Url> {
        let mut url = self.url.clone();
        url.set_port(Some(
            self.api_port
                .context("Can't build metrics URL without a metrics port")?,
        ))
        .unwrap();
        url.set_path(path);
        Ok(url)
    }

    pub fn get_metrics_client(&self, timeout: Duration) -> Result<reqwest::Client> {
        match self.metrics_port {
            Some(_) => Ok(reqwest::ClientBuilder::new()
                .timeout(timeout)
                .cookie_provider(self.cookie_store.clone())
                .build()
                .unwrap()),
            None => Err(anyhow!(
                "Cannot build metrics client without a metrics port"
            )),
        }
    }

    pub fn get_api_client(&self, timeout: Duration) -> Result<VelorRestClient> {
        let client = reqwest::ClientBuilder::new()
            .timeout(timeout)
            .cookie_provider(self.cookie_store.clone())
            .build()
            .unwrap();

        Ok(VelorRestClient::from((client, self.get_api_url()?)))
    }

    /// Gets the NodeAddress as a NetworkAddress. If the URL is a domain name,
    /// it will automatically perform DNS resolution. This method returns an
    /// error if `public_key` is None.
    pub fn as_noise_network_address(&self) -> Result<NetworkAddress> {
        // Confirm we have a public key. Technically we can build a NetworkAddress
        // without one, but it's not useful for any of our needs without one.
        let public_key = match self.public_key {
            Some(public_key) => public_key,
            None => bail!("Cannot convert NodeAddress to NetworkAddress without a public key"),
        };

        // Ensure we can get socket addrs from the URL. If the URL is a domain
        // name, it will automatically perform DNS resolution.
        let socket_addrs = self
            .url
            .socket_addrs(|| None)
            .with_context(|| format!("Failed to get SocketAddrs from address {}", self.url))?;

        // Ensure this results in exactly one SocketAddr.
        if socket_addrs.is_empty() {
            bail!(
                "NodeAddress {} did not resolve to any SocketAddrs. If DNS, ensure domain name is valid",
                self.url
            );
        }
        if socket_addrs.len() > 1 {
            velor_logger::warn!(
                "NodeAddress {} resolved to multiple SocketAddrs, but we're only checking the first one: {:?}",
                self.url,
                socket_addrs,
            );
        }

        // Configure the SocketAddr with the provided noise port.
        let mut socket_addr = socket_addrs[0];
        socket_addr.set_port(
            self.noise_port
                .context("Can't build NetworkAddress without a noise port")?,
        );

        // Build a network address, including the public key and protocol.
        Ok(NetworkAddress::from(socket_addr).append_prod_protos(public_key, 0))
    }
}
