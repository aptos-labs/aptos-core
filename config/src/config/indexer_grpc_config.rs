// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::config::{
    config_optimizer::ConfigOptimizer, config_sanitizer::ConfigSanitizer,
    node_config_loader::NodeType, Error, NodeConfig,
};
use aptos_types::chain_id::ChainId;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::{
    fmt::{Debug, Formatter},
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
};

// Useful indexer defaults
const DEFAULT_PROCESSOR_TASK_COUNT: u16 = 20;
const DEFAULT_PROCESSOR_BATCH_SIZE: u16 = 1000;
const DEFAULT_OUTPUT_BATCH_SIZE: u16 = 100;
pub const DEFAULT_GRPC_STREAM_PORT: u16 = 50051;

#[derive(Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct IndexerGrpcConfig {
    pub enabled: bool,

    /// If true, the GRPC stream interface exposed by the data service will be used
    /// instead of the standard fullnode GRPC stream interface. In other words, with
    /// this enabled, you can use an indexer fullnode like it is an instance of the
    /// indexer-grpc data service (aka the Transaction Stream Service API).
    pub use_data_service_interface: bool,

    /// The address that the grpc server will listen on.
    pub address: SocketAddr,

    /// Number of processor tasks to fan out
    pub processor_task_count: u16,

    /// Number of transactions each processor will process
    pub processor_batch_size: u16,

    /// Number of transactions returned in a single stream response
    pub output_batch_size: u16,

    pub enable_expensive_logging: bool,
}

impl Debug for IndexerGrpcConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexerGrpcConfig")
            .field("enabled", &self.enabled)
            .field(
                "use_data_service_interface",
                &self.use_data_service_interface,
            )
            .field("address", &self.address)
            .field("processor_task_count", &self.processor_task_count)
            .field("processor_batch_size", &self.processor_batch_size)
            .field("output_batch_size", &self.output_batch_size)
            .field("enable_expensive_logging", &self.enable_expensive_logging)
            .finish()
    }
}

// Reminder, #[serde(default)] on IndexerGrpcConfig means that the default values for
// fields will come from this Default impl, unless the field has a specific
// #[serde(default)] on it (which none of the above do).
impl Default for IndexerGrpcConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            use_data_service_interface: false,
            address: SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::new(0, 0, 0, 0),
                DEFAULT_GRPC_STREAM_PORT,
            )),
            processor_task_count: DEFAULT_PROCESSOR_TASK_COUNT,
            processor_batch_size: DEFAULT_PROCESSOR_BATCH_SIZE,
            output_batch_size: DEFAULT_OUTPUT_BATCH_SIZE,
            enable_expensive_logging: false,
        }
    }
}

impl ConfigSanitizer for IndexerGrpcConfig {
    fn sanitize(
        node_config: &NodeConfig,
        _node_type: NodeType,
        _chain_id: Option<ChainId>,
    ) -> Result<(), Error> {
        let sanitizer_name = Self::get_sanitizer_name();

        if !node_config.indexer_grpc.enabled {
            return Ok(());
        }

        if !node_config.storage.enable_indexer && !node_config.indexer_table_info.enabled {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                "storage.enable_indexer or indexer_table_info.enabled must be true if indexer_grpc.enabled is true".to_string(),
            ));
        }
        Ok(())
    }
}

impl ConfigOptimizer for IndexerGrpcConfig {
    fn optimize(
        node_config: &mut NodeConfig,
        _local_config_yaml: &Value,
        _node_type: NodeType,
        _chain_id: Option<ChainId>,
    ) -> Result<bool, Error> {
        let indexer_config = &mut node_config.indexer_grpc;
        // If the indexer is not enabled, there's nothing to do
        if !indexer_config.enabled {
            return Ok(false);
        }

        // TODO: we really shouldn't be overriding the configs if they are
        // specified in the local node config file. This optimizer should
        // migrate to the pattern used by other optimizers, but for now, we'll
        // just keep the legacy behaviour to avoid breaking anything.

        // Override with environment variables if they are set
        indexer_config.enable_expensive_logging = env_var_or_default(
            "INDEXER_GRPC_ENABLE_EXPENSIVE_LOGGING",
            Some(indexer_config.enable_expensive_logging),
            None,
        )
        .unwrap_or(false);

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{IndexerTableInfoConfig, StorageConfig};

    #[test]
    fn test_sanitize_enable_indexer() {
        // Create a storage config and disable the storage indexer
        let mut storage_config = StorageConfig::default();
        let mut table_info_config = IndexerTableInfoConfig::default();
        storage_config.enable_indexer = false;
        table_info_config.enabled = false;

        // Create a node config with the indexer enabled, but the storage indexer disabled
        let mut node_config = NodeConfig {
            storage: storage_config,
            indexer_table_info: table_info_config,
            indexer_grpc: IndexerGrpcConfig {
                enabled: true,
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error = IndexerGrpcConfig::sanitize(
            &node_config,
            NodeType::Validator,
            Some(ChainId::mainnet()),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));

        // Enable the storage indexer
        node_config.storage.enable_indexer = true;

        // Sanitize the config and verify that it now succeeds
        IndexerGrpcConfig::sanitize(&node_config, NodeType::Validator, Some(ChainId::mainnet()))
            .unwrap();

        // Disable the storage indexer and enable the table info service
        node_config.storage.enable_indexer = false;

        // Sanitize the config and verify that it fails
        let error = IndexerGrpcConfig::sanitize(
            &node_config,
            NodeType::Validator,
            Some(ChainId::mainnet()),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));

        // Enable the table info service
        node_config.indexer_table_info.enabled = true;

        // Sanitize the config and verify that it now succeeds
        IndexerGrpcConfig::sanitize(&node_config, NodeType::Validator, Some(ChainId::mainnet()))
            .unwrap();
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
