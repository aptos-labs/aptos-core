// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::config::{
    config_sanitizer::ConfigSanitizer, node_config_loader::NodeType, Error, NodeConfig,
};
use aptos_types::chain_id::ChainId;
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

// Useful indexer defaults
const DEFAULT_PROCESSOR_TASK_COUNT: u16 = 20;
const DEFAULT_PROCESSOR_BATCH_SIZE: u16 = 1000;
const DEFAULT_OUTPUT_BATCH_SIZE: u16 = 100;
pub const DEFAULT_GRPC_STREAM_PORT: u16 = 50051;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
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
        }
    }
}

impl ConfigSanitizer for IndexerGrpcConfig {
    fn sanitize(
        node_config: &NodeConfig,
        _node_type: NodeType,
        _chain_id: Option<ChainId>,
    ) -> Result<(), Error> {
        // let sanitizer_name = Self::get_sanitizer_name();

        if !node_config.indexer_grpc.enabled {
            return Ok(());
        }

        // if !node_config.storage.enable_indexer {
        //     return Err(Error::ConfigSanitizerFailed(
        //         sanitizer_name,
        //         "storage.enable_indexer must be true if indexer_grpc.enabled is true".to_string(),
        //     ));
        // }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::StorageConfig;

    #[test]
    fn test_sanitize_enable_indexer() {
        // Create a storage config and disable the storage indexer
        let mut storage_config = StorageConfig::default();
        storage_config.enable_indexer = false;

        // Create a node config with the indexer enabled, but the storage indexer disabled
        let mut node_config = NodeConfig {
            storage: storage_config,
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
    }
}
