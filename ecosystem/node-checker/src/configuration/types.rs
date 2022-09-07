// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::evaluator::EvaluationResult;
use crate::{
    evaluators::{
        direct::{
            get_node_identity, HandshakeEvaluatorArgs, LatencyEvaluatorArgs,
            NodeIdentityEvaluatorArgs, StateSyncVersionEvaluatorArgs, TpsEvaluatorArgs,
            TransactionAvailabilityEvaluatorArgs,
        },
        metrics::{
            ConsensusProposalsEvaluatorArgs, ConsensusRoundEvaluatorArgs,
            ConsensusTimeoutsEvaluatorArgs, NetworkMinimumPeersEvaluatorArgs,
            NetworkPeersWithinToleranceEvaluatorArgs, StateSyncVersionMetricsEvaluatorArgs,
        },
        system_information::{BuildVersionEvaluatorArgs, HardwareEvaluatorArgs},
    },
    runner::BlockingRunnerArgs,
};
use anyhow::{bail, format_err, Context, Result};
use aptos_config::config::RoleType;
use aptos_crypto::{x25519, ValidCryptoMaterialStringExt};
use aptos_rest_client::{Client as AptosRestClient, IndexResponse};
use aptos_sdk::types::{chain_id::ChainId, network_address::NetworkAddress};
use clap::Parser;
use once_cell::sync::Lazy;
use poem_openapi::{types::Example, Object as PoemObject};
use reqwest::cookie::Jar;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use url::Url;

pub const DEFAULT_METRICS_PORT: u16 = 9101;
pub const DEFAULT_API_PORT: u16 = 8080;
pub const DEFAULT_NOISE_PORT: u16 = 6180;

pub static DEFAULT_METRICS_PORT_STR: Lazy<String> =
    Lazy::new(|| format!("{}", DEFAULT_METRICS_PORT));
pub static DEFAULT_API_PORT_STR: Lazy<String> = Lazy::new(|| format!("{}", DEFAULT_API_PORT));
pub static DEFAULT_NOISE_PORT_STR: Lazy<String> = Lazy::new(|| format!("{}", DEFAULT_NOISE_PORT));

// To briefly explain why many of these structs derive 3 different classes of traits:
// - Parser (clap): To allow users to generate configs easily using nhc configuration create
// - Serialize / Deserialize (serde): So we can read / write configs from / to disk
// - PoemObject: So we can return the configuration over the API

#[derive(Clone, Debug, Deserialize, Parser, PoemObject, Serialize)]
#[clap(author, version, about, long_about = None)]
pub struct NodeConfiguration {
    #[clap(flatten)]
    pub node_address: NodeAddress,

    /// This is the name we expect clients to send over the wire to select
    /// which configuration they want to use. e.g. devnet_fullnode
    #[clap(long)]
    pub configuration_name: String,

    /// This is the name we will show for this configuration to users.
    /// For example, if someone opens the NHC frontend, they will see this name
    /// in a dropdown list of configurations they can test their node against.
    /// e.g. "Devnet FullNode", "Testnet Validator Node", etc.
    #[clap(long)]
    pub configuration_name_pretty: String,

    /// The chain ID we expect to find when we speak to the baseline node
    /// at `node_address`. Regardless of whether this is set, at startup we
    /// will contact the node to see what its chain ID is. If `chain_id` is
    /// set here and doesn't match the chain ID returned by the node, we
    /// will exit, signalling a configuration error.
    #[clap(long)]
    #[oai(skip)]
    chain_id: Option<ChainId>,

    /// This works the same as `chain_id` above, but for role type. Example
    /// values: "full_node", "validator", etc.
    #[clap(long)]
    #[oai(skip)]
    role_type: Option<RoleType>,

    /// The evaluators to use, e.g. state_sync_version, consensus_proposals, etc.
    #[clap(long, required = true, min_values = 1, use_value_delimiter = true)]
    pub evaluators: Vec<String>,

    #[clap(flatten)]
    pub evaluator_args: EvaluatorArgs,

    #[clap(flatten)]
    pub runner_args: RunnerArgs,
}

// TODO: Having comments like "only call this after X" is obviously a bad sign.
// It'd be better to have an enum with two variants, e.g. unfetched and fetched.
impl NodeConfiguration {
    /// Only call this after fetch_additional_configuration has been called.
    pub fn get_chain_id(&self) -> ChainId {
        self.chain_id
            .expect("get_chain_id called before fetch_additional_configuration")
    }

    /// Only call this after fetch_additional_configuration has been called.
    pub fn get_role_type(&self) -> RoleType {
        self.role_type
            .expect("get_role_type called before fetch_additional_configuration")
    }

    /// In this function we fetch the chain ID and role type from the node.
    /// If chain_id and role_type are already set, we validate that the values
    /// match up. If they're not set, we set them using the values we find.
    pub async fn fetch_additional_configuration(&mut self) -> Result<()> {
        let (reported_chain_id, reported_role_type) =
            get_node_identity(&self.node_address, Duration::from_secs(5))
                .await
                .map_err(|e| {
                    format_err!(
                    "Failed to fetch chain ID and role type for baseline node configuration: {}",
                    e
                )
                })?;
        if let Some(configured_chain_id) = self.chain_id {
            if configured_chain_id != reported_chain_id {
                bail!("Chain ID mismatch: The baseline configuration {} says the chain ID is {} but the node reports chain ID {}", self.configuration_name, configured_chain_id, reported_chain_id);
            }
        }
        if let Some(configured_role_type) = self.role_type {
            if configured_role_type != reported_role_type {
                bail!("Role type mismatch: The baseline configuration {} says the role type is {} but the node reports role type {}", self.configuration_name, configured_role_type, reported_role_type);
            }
        }
        self.chain_id = Some(reported_chain_id);
        self.role_type = Some(reported_role_type);
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Parser, PoemObject, Serialize)]
pub struct EvaluatorArgs {
    #[clap(flatten)]
    pub build_version_args: BuildVersionEvaluatorArgs,

    #[clap(flatten)]
    pub consensus_proposals_args: ConsensusProposalsEvaluatorArgs,

    #[clap(flatten)]
    pub consensus_round_args: ConsensusRoundEvaluatorArgs,

    #[clap(flatten)]
    pub consensus_timeouts_args: ConsensusTimeoutsEvaluatorArgs,

    #[clap(flatten)]
    pub handshake_args: HandshakeEvaluatorArgs,

    #[clap(flatten)]
    pub hardware_args: HardwareEvaluatorArgs,

    #[clap(flatten)]
    pub latency_args: LatencyEvaluatorArgs,

    #[clap(flatten)]
    pub network_minimum_peers_args: NetworkMinimumPeersEvaluatorArgs,

    #[clap(flatten)]
    pub network_peers_tolerance_args: NetworkPeersWithinToleranceEvaluatorArgs,

    #[clap(flatten)]
    pub node_identity_args: NodeIdentityEvaluatorArgs,

    #[clap(flatten)]
    pub state_sync_version_args: StateSyncVersionEvaluatorArgs,

    #[clap(flatten)]
    pub state_sync_version_metrics_args: StateSyncVersionMetricsEvaluatorArgs,

    #[clap(flatten)]
    #[oai(skip)]
    pub tps_args: TpsEvaluatorArgs,

    #[clap(flatten)]
    pub transaction_availability_args: TransactionAvailabilityEvaluatorArgs,
}

#[derive(Clone, Debug, Deserialize, Parser, PoemObject, Serialize)]
pub struct RunnerArgs {
    #[clap(flatten)]
    pub blocking_runner_args: BlockingRunnerArgs,
}

#[derive(Clone, Debug, Deserialize, Parser, PoemObject, Serialize)]
#[oai(example)]
pub struct NodeAddress {
    /// Target URL. This should include a scheme (e.g. http://). If there is
    /// no scheme, we will prepend http://.
    #[clap(long)]
    pub url: Url,

    /// Metrics port.
    #[clap(long, default_value = &DEFAULT_METRICS_PORT_STR)]
    #[oai(default = "Self::default_metrics_port")]
    #[serde(default = "NodeAddress::default_metrics_port")]
    metrics_port: u16,

    /// API port.
    #[clap(long, default_value = &DEFAULT_API_PORT_STR)]
    #[oai(default = "Self::default_api_port")]
    #[serde(default = "NodeAddress::default_api_port")]
    api_port: u16,

    /// Validator communication port.
    #[clap(long, default_value = &DEFAULT_NOISE_PORT_STR)]
    #[oai(default = "Self::default_noise_port")]
    #[serde(default = "NodeAddress::default_noise_port")]
    noise_port: u16,

    /// Public key for the node. This is used for the HandshakeEvaluator.
    /// If that evaluator is not enabled, this is not necessary.
    #[clap(long, value_parser = x25519::PublicKey::from_encoded_string)]
    #[oai(skip)]
    public_key: Option<x25519::PublicKey>,

    // Cookie store. We don't include this in anything external (clap, the
    // OpenAPI spec, serde, etc.), this is just for internal use.
    #[oai(skip)]
    #[clap(skip)]
    #[serde(skip)]
    cookie_store: Arc<Jar>,
}

impl NodeAddress {
    pub fn new(url: Url) -> Self {
        Self {
            url,
            metrics_port: Self::default_metrics_port(),
            api_port: Self::default_api_port(),
            noise_port: Self::default_noise_port(),
            public_key: None,
            cookie_store: Arc::new(Jar::default()),
        }
    }

    pub fn metrics_port(mut self, port: u16) -> Self {
        self.metrics_port = port;
        self
    }

    pub fn api_port(mut self, port: u16) -> Self {
        self.api_port = port;
        self
    }

    pub fn noise_port(mut self, port: u16) -> Self {
        self.noise_port = port;
        self
    }

    pub fn public_key(mut self, public_key: Option<x25519::PublicKey>) -> Self {
        self.public_key = public_key;
        self
    }

    /// Do not use this to build a client, use get_metrics_client.
    pub fn get_metrics_port(&self) -> u16 {
        self.metrics_port
    }

    /// Do not use this to build a client, use get_api_client.
    pub fn get_api_port(&self) -> u16 {
        self.api_port
    }

    pub fn get_noise_port(&self) -> u16 {
        self.noise_port
    }

    pub fn get_public_key(&self) -> Option<x25519::PublicKey> {
        self.public_key
    }

    pub fn default_metrics_port() -> u16 {
        DEFAULT_METRICS_PORT
    }

    pub fn default_api_port() -> u16 {
        DEFAULT_API_PORT
    }

    pub fn default_noise_port() -> u16 {
        DEFAULT_NOISE_PORT
    }

    pub fn get_api_url(&self) -> Url {
        let mut url = self.url.clone();
        url.set_port(Some(self.api_port)).unwrap();
        url
    }

    pub fn get_metrics_url(&self) -> Url {
        let mut url = self.url.clone();
        url.set_port(Some(self.metrics_port)).unwrap();
        url
    }

    pub fn get_metrics_client(&self, timeout: Duration) -> reqwest::Client {
        reqwest::ClientBuilder::new()
            .timeout(timeout)
            .cookie_provider(self.cookie_store.clone())
            .build()
            .unwrap()
    }

    pub fn get_api_client(&self, timeout: Duration) -> AptosRestClient {
        let client = reqwest::ClientBuilder::new()
            .timeout(timeout)
            .cookie_provider(self.cookie_store.clone())
            .build()
            .unwrap();

        AptosRestClient::from((client, self.get_api_url()))
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
            aptos_logger::warn!(
                "NodeAddress {} resolved to multiple SocketAddrs, but we're only checking the first one: {:?}",
                self.url,
                socket_addrs,
            );
        }

        // Configure the SocketAddr with the provided noise port.
        let mut socket_addr = socket_addrs[0];
        socket_addr.set_port(self.noise_port);

        // Build a network address, including the public key and protocol.
        Ok(NetworkAddress::from(socket_addr).append_prod_protos(public_key, 0))
    }

    pub async fn get_index_response(&self, timeout: Duration) -> Result<IndexResponse> {
        Ok(self.get_api_client(timeout).get_index().await?.into_inner())
    }

    pub async fn get_index_response_or_evaluation_result(
        &self,
        timeout: Duration,
    ) -> Result<IndexResponse, EvaluationResult> {
        match self.get_index_response(timeout).await {
            Ok(index_response) => Ok(index_response),
            Err(error) => Err(EvaluationResult {
                headline: "Failed to read response from / on API".to_string(),
                score: 0,
                explanation: format!(
                    "We received an error response hitting / (the index) of the API of \
                    your node, make sure your API port ({}) is publicly accessible: {}.",
                    self.api_port, error
                ),
                category: "api".to_string(),
                evaluator_name: "index_response".to_string(),
                links: vec![],
            }),
        }
    }
}

impl Example for NodeAddress {
    fn example() -> Self {
        Self {
            url: Url::parse("http://mynode.mysite.com").unwrap(),
            metrics_port: Self::default_metrics_port(),
            api_port: Self::default_api_port(),
            noise_port: Self::default_noise_port(),
            public_key: Some(
                x25519::PublicKey::from_encoded_string(
                    "0x44fd1324c66371b4788af0b901c9eb8088781acb29e6b8b9c791d5d9838fbe1f",
                )
                .unwrap(),
            ),
            cookie_store: Arc::new(Jar::default()),
        }
    }
}
