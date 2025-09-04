// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::config::{
    config_optimizer::ConfigOptimizer, node_config_loader::NodeType, Error, NodeConfig,
};
use velor_logger::warn;
use velor_types::chain_id::ChainId;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::fmt::{Debug, Formatter};

// Useful indexer environment variables
const GAP_LOOKBACK_VERSIONS: &str = "GAP_LOOKBACK_VERSIONS";
const INDEXER_DATABASE_URL: &str = "INDEXER_DATABASE_URL";
const PROCESSOR_NAME: &str = "PROCESSOR_NAME";
const STARTING_VERSION: &str = "STARTING_VERSION";

// Useful indexer defaults
pub const DEFAULT_BATCH_SIZE: u16 = 500;
pub const DEFAULT_FETCH_TASKS: u8 = 5;
pub const DEFAULT_PROCESSOR_TASKS: u8 = 5;
pub const DEFAULT_EMIT_EVERY: u64 = 1000;

#[derive(Clone, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct IndexerConfig {
    /// Whether the indexer is enabled or not
    /// Alternatively can set the `INDEXER_ENABLED` env var
    #[serde(default)]
    pub enabled: bool,

    /// Postgres database uri, ex: "postgresql://user:pass@localhost/postgres"
    /// Alternatively can set the `INDEXER_DATABASE_URL` env var
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postgres_uri: Option<String>,

    /// The specific processor that it will run, ex: "token_processor"
    /// Alternatively can set the `PROCESSOR_NAME` env var
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processor: Option<String>,

    /// If set, will ignore database contents and start processing from the specified version.
    /// This will not delete any database contents, just transactions as it reprocesses them.
    /// Alternatively can set the `STARTING_VERSION` env var
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_version: Option<u64>,

    ///////////////////
    ///////////////////
    ///////////////////
    /// If set, don't run any migrations
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_migrations: Option<bool>,

    /// If set, will make sure that we're indexing the right chain
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check_chain_id: Option<bool>,

    /// How many versions to fetch and process from a node in parallel
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<u16>,

    /// How many tasks to run for fetching the transactions
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetch_tasks: Option<u8>,

    /// How many tasks to run for processing the transactions
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processor_tasks: Option<u8>,

    /// How many versions to process before logging a "processed X versions" message.
    /// This will only be checked every `batch_size` number of versions.
    /// Set to 0 to disable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emit_every: Option<u64>,

    /// Indicates how many versions we should look back for gaps (default 1.5M versions, meaning
    /// we will only find gaps within MAX - 1.5M versions)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap_lookback_versions: Option<u64>,

    /// Which address does the ans contract live at. Only available for token_processor. If null, disable ANS indexing
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ans_contract_address: Option<String>,

    /// Custom NFT points contract
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nft_points_contract: Option<String>,
}

impl Debug for IndexerConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let postgres_uri = self.postgres_uri.as_ref().map(|u| {
            let mut parsed_url = url::Url::parse(u).expect("Invalid postgres uri");
            if parsed_url.password().is_some() {
                parsed_url.set_password(Some("*")).unwrap();
            }
            parsed_url.to_string()
        });
        f.debug_struct("IndexerConfig")
            .field("enabled", &self.enabled)
            .field("postgres_uri", &postgres_uri)
            .field("processor", &self.processor)
            .field("starting_version", &self.starting_version)
            .field("skip_migrations", &self.skip_migrations)
            .field("check_chain_id", &self.check_chain_id)
            .field("batch_size", &self.batch_size)
            .field("fetch_tasks", &self.fetch_tasks)
            .field("processor_tasks", &self.processor_tasks)
            .field("emit_every", &self.emit_every)
            .field("gap_lookback_versions", &self.gap_lookback_versions)
            .field("ans_contract_address", &self.ans_contract_address)
            .field("nft_points_contract", &self.nft_points_contract)
            .finish()
    }
}

impl ConfigOptimizer for IndexerConfig {
    fn optimize(
        node_config: &mut NodeConfig,
        _local_config_yaml: &Value,
        _node_type: NodeType,
        _chain_id: Option<ChainId>,
    ) -> Result<bool, Error> {
        // If the indexer is not enabled, there's nothing to do
        let indexer_config = &mut node_config.indexer;
        if !indexer_config.enabled {
            return Ok(false);
        }

        // TODO: we really shouldn't be overriding the configs if they are
        // specified in the local node config file. This optimizer should
        // migrate to the pattern used by other optimizers, but for now, we'll
        // just keep the legacy behaviour to avoid breaking anything.

        // Verify and set the postgres uri
        indexer_config.postgres_uri = env_var_or_default(
            INDEXER_DATABASE_URL,
            indexer_config.postgres_uri.clone(),
            Some(format!(
                "Either 'config.indexer.postgres_uri' or '{}' must be set!",
                INDEXER_DATABASE_URL
            )),
        );

        // Verify and set the processor
        indexer_config.processor = env_var_or_default(
            PROCESSOR_NAME,
            indexer_config
                .processor
                .clone()
                .or_else(|| Some("default_processor".to_string())),
            None,
        );

        // Verify and set the starting version
        indexer_config.starting_version = match std::env::var(STARTING_VERSION).ok() {
            None => indexer_config.starting_version,
            Some(starting_version) => match starting_version.parse::<u64>() {
                Ok(version) => Some(version),
                Err(error) => {
                    // This will allow a processor to have STARTING_VERSION undefined when deploying
                    warn!(
                        "Invalid STARTING_VERSION: {}. Error: {:?}. Using {:?} instead.",
                        starting_version, error, indexer_config.starting_version
                    );
                    indexer_config.starting_version
                },
            },
        };

        // Set appropriate defaults
        indexer_config.skip_migrations = indexer_config.skip_migrations.or(Some(false));
        indexer_config.check_chain_id = indexer_config.check_chain_id.or(Some(true));
        indexer_config.batch_size = default_if_zero(
            indexer_config.batch_size.map(|v| v as u64),
            DEFAULT_BATCH_SIZE as u64,
        )
        .map(|v| v as u16);
        indexer_config.fetch_tasks = default_if_zero(
            indexer_config.fetch_tasks.map(|v| v as u64),
            DEFAULT_FETCH_TASKS as u64,
        )
        .map(|v| v as u8);
        indexer_config.processor_tasks = default_if_zero(
            indexer_config.processor_tasks.map(|v| v as u64),
            DEFAULT_PROCESSOR_TASKS as u64,
        )
        .map(|value| value as u8);
        indexer_config.emit_every = indexer_config.emit_every.or(Some(0));
        indexer_config.gap_lookback_versions = env_var_or_default(
            GAP_LOOKBACK_VERSIONS,
            indexer_config.gap_lookback_versions.or(Some(1_500_000)),
            None,
        );

        Ok(true)
    }
}

/// Returns the default if the value is 0, otherwise returns the value
fn default_if_zero(value: Option<u64>, default: u64) -> Option<u64> {
    match value {
        None => Some(default),
        Some(0) => Some(default),
        Some(value) => Some(value),
    }
}

/// Returns the value of the environment variable `env_var`
/// if it is set, otherwise returns `default`.
fn env_var_or_default<T: std::str::FromStr>(
    env_var: &'static str,
    default: Option<T>,
    expected_message: Option<String>,
) -> Option<T> {
    let partial = std::env::var(env_var).ok().map(|s| s.parse().ok());
    match default {
        None => partial.unwrap_or_else(|| {
            panic!(
                "{}",
                expected_message
                    .unwrap_or_else(|| { format!("Expected env var {} to be set", env_var) })
            )
        }),
        Some(default_value) => partial.unwrap_or(Some(default_value)),
    }
}
