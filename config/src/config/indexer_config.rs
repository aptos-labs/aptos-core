// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

pub const DEFAULT_BATCH_SIZE: u16 = 500;
pub const DEFAULT_FETCH_TASKS: u8 = 5;
pub const DEFAULT_PROCESSOR_TASKS: u8 = 5;
pub const DEFAULT_EMIT_EVERY: u64 = 1000;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
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
}

pub fn env_or_default<T: std::str::FromStr>(
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

pub fn default_if_zero_u8(value: Option<u8>, default: u8) -> Option<u8> {
    default_if_zero(value.map(|v| v as u64), default as u64).map(|v| v as u8)
}

pub fn default_if_zero(value: Option<u64>, default: u64) -> Option<u64> {
    match value {
        None => Some(default),
        Some(value) => {
            if value == 0 {
                Some(default)
            } else {
                Some(value)
            }
        }
    }
}

pub fn must_be_set(config_var: &'static str, env_var: &'static str) -> Option<String> {
    Some(format!(
        "Either 'config.indexer.{}' or '{}' must be set!",
        config_var, env_var
    ))
}
