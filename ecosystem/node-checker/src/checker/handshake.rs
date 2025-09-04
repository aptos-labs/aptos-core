// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// TODO: This Checker requires that the target node API be open only because
// we need to know the chain ID. This is pretty unfortunate, especially since
// it's only really necessary for devnet. Try to find another approach.

use super::{CheckResult, Checker, CheckerError, CommonCheckerConfig};
use crate::{
    get_provider,
    provider::{noise::NoiseProvider, ProviderCollection},
};
use anyhow::Result;
use velor_network_checker::args::HandshakeArgs;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HandshakeCheckerConfig {
    #[serde(flatten)]
    pub common: CommonCheckerConfig,

    pub handshake_args: HandshakeArgs,
}

#[derive(Debug)]
pub struct HandshakeChecker {
    config: HandshakeCheckerConfig,
}

impl HandshakeChecker {
    pub fn new(config: HandshakeCheckerConfig) -> Self {
        Self { config }
    }
}

#[async_trait::async_trait]
impl Checker for HandshakeChecker {
    /// Assert that we can establish a noise connection with the target node
    /// with the given public key. If we cannot, it implies that either the
    /// node is not listening on that port, or the node is not running with
    /// the private key matching the public key provided as part of the request
    /// to NHC.
    async fn check(
        &self,
        providers: &ProviderCollection,
    ) -> Result<Vec<CheckResult>, CheckerError> {
        let target_noise_provider = get_provider!(
            providers.target_noise_provider,
            self.config.common.required,
            NoiseProvider
        );

        Ok(vec![
            match target_noise_provider.establish_connection().await {
                Ok(message) => Self::build_result(
                    "Noise connection established successfully".to_string(),
                    100,
                    format!(
                        "{}. This indicates your noise port ({}) is open and the node is \
                    running with the private key matching the provided public key.",
                        message,
                        target_noise_provider.network_address.find_port().unwrap()
                    ),
                ),
                Err(err) => Self::build_result(
                    "Failed to establish noise connection".to_string(),
                    0,
                    format!(
                        "{:#}. Either the noise port ({}) is closed or the node is not \
                    running with the private key matching the provided public key.",
                        err,
                        target_noise_provider.network_address.find_port().unwrap()
                    ),
                ),
            },
        ])
    }
}
