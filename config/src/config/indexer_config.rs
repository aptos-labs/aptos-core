// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

pub const DEFAULT_CHECK_CHAIN_ID: bool = true;
pub const DEFAULT_BATCH_SIZE: u16 = 500;
pub const DEFAULT_FETCH_TASKS: u8 = 5;
pub const DEFAULT_PROCESSOR_TASKS: u8 = 5;
pub const DEFAULT_EMIT_EVERY: u64 = 1000;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct IndexerConfig {
    /// Whether the indexer is enabled or not
    /// Alternatively can set the `INDEXER_ENABLED` env var
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Postgres database uri, ex: "postgresql://user:pass@localhost/postgres"
    /// Alternatively can set the `INDEXER_DATABASE_URL` env var
    #[serde(default = "default_database_url")]
    pub postgres_uri: String,

    /// The specific processor that it will run, ex: "token_processor"
    /// Alternatively can set the `PROCESSOR_NAME` env var
    #[serde(default = "default_processor_name")]
    pub processor: String,

    /// If set, will ignore database contents and start processing from the specified version.
    /// This will not delete any database contents, just transactions as it reprocesses them.
    /// Alternatively can set the `STARTING_VERSION` env var
    #[serde(default = "default_starting_version")]
    pub starting_version: Option<u64>,

    ///////////////////
    ///////////////////
    ///////////////////
    /// If set, don't run any migrations
    #[serde(default)]
    pub skip_migrations: bool,

    /// turn on the token URI fetcher
    #[serde(default)]
    pub index_token_uri_data: bool,

    /// If set, will make sure that we're still indexing the right chain every 100K transactions
    #[serde(default = "default_true")]
    pub check_chain_id: bool,

    /// How many versions to fetch and process from a node in parallel
    #[serde(default = "default_batch_size")]
    pub batch_size: u16,

    /// How many tasks to run for fetching the transactions
    #[serde(default = "default_fetch_tasks")]
    pub fetch_tasks: u8,

    /// How many tasks to run for processing the transactions
    #[serde(default = "default_processor_tasks")]
    pub processor_tasks: u8,

    /// How many versions to process before logging a "processed X versions" message.
    /// This will only be checked every `batch_size` number of versions.
    /// Set to 0 to disable.
    #[serde(default = "default_emit_every")]
    pub emit_every: u64,
}

fn default_batch_size() -> u16 {
    DEFAULT_BATCH_SIZE
}

fn default_fetch_tasks() -> u8 {
    DEFAULT_FETCH_TASKS
}

fn default_processor_tasks() -> u8 {
    DEFAULT_PROCESSOR_TASKS
}

fn default_emit_every() -> u64 {
    DEFAULT_EMIT_EVERY
}

fn default_true() -> bool {
    true
}

fn default_enabled() -> bool {
    std::env::var("INDEXER_ENABLED").ok().is_some()
}

fn default_starting_version() -> Option<u64> {
    std::env::var("STARTING_VERSION")
        .ok()
        .and_then(|s| s.parse().ok())
}

fn default_database_url() -> String {
    env_or_default(
        "INDEXER_DATABASE_URL",
        None,
        must_be_set("postgres_uri", "INDEXER_DATABASE_URL"),
    )
}

fn default_processor_name() -> String {
    env_or_default(
        "PROCESSOR_NAME",
        Some("default_processor".to_string()),
        None,
    )
}

fn env_or_default<T: std::str::FromStr>(
    env_var: &'static str,
    default: Option<T>,
    expected_message: Option<String>,
) -> T {
    let partial = std::env::var(env_var).ok().and_then(|s| s.parse().ok());
    match default {
        None => partial.unwrap_or_else(|| panic!("{}", expected_message.unwrap())),
        Some(default_value) => partial.unwrap_or(default_value),
    }
}

fn must_be_set(config_var: &'static str, env_var: &'static str) -> Option<String> {
    Some(format!(
        "Either 'config.indexer.{}' or '{}' must be set!",
        config_var, env_var
    ))
}
