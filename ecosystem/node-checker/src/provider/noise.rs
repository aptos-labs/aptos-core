// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! There is only one noise based Checker right now, so this Provider is a bit light
//! on features, it just makes it possible to make a noise connection.

use super::{
    api_index::ApiIndexProvider,
    traits::{Provider, ProviderError},
    CommonProviderConfig,
};
use anyhow::Result;
use velor_network_checker::{
    args::{CheckEndpointArgs, HandshakeArgs, NodeAddressArgs},
    check_endpoint::check_endpoint,
};
use velor_sdk::types::{chain_id::ChainId, network_address::NetworkAddress};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NoiseProviderConfig {
    #[serde(default, flatten)]
    pub common: CommonProviderConfig,

    #[serde(default, flatten)]
    pub handshake_args: HandshakeArgs,
}

#[derive(Clone, Debug)]
pub struct NoiseProvider {
    pub config: NoiseProviderConfig,

    /// A noise NetworkAddress. We can use this to establish a noise connection.
    pub network_address: NetworkAddress,

    /// An API index provider. We use this to get the chain ID.
    pub api_indexer_provider: Arc<ApiIndexProvider>,
}

impl NoiseProvider {
    pub fn new(
        config: NoiseProviderConfig,
        network_address: NetworkAddress,
        api_indexer_provider: Arc<ApiIndexProvider>,
    ) -> Self {
        Self {
            config,
            network_address,
            api_indexer_provider,
        }
    }

    /// This function provides an opionated extra step over just the `provide` call by
    /// actually trying to establih a connection.
    pub async fn establish_connection(&self) -> Result<String> {
        check_endpoint(
            &CheckEndpointArgs {
                node_address_args: self.provide().await?,
                handshake_args: self.config.handshake_args.clone(),
            },
            None,
        )
        .await
    }
}

#[async_trait]
impl Provider for NoiseProvider {
    type Output = NodeAddressArgs;

    async fn provide(&self) -> Result<Self::Output, ProviderError> {
        Ok(NodeAddressArgs {
            address: self.network_address.clone(),
            chain_id: ChainId::new(self.api_indexer_provider.provide().await?.chain_id),
        })
    }

    fn explanation() -> &'static str {
        "The noise port was not included in the request or the public key was invalid."
    }
}
