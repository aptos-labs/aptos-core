// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::config::{
    config_sanitizer::ConfigSanitizer, node_config_loader::NodeType, Error, NodeConfig,
};
use aptos_types::chain_id::ChainId;
use serde::{Deserialize, Serialize};

// Useful indexer defaults
const DEFAULT_ADDRESS: &str = "0.0.0.0:50051";
const DEFAULT_OUTPUT_BATCH_SIZE: u16 = 100;
const DEFAULT_PROCESSOR_BATCH_SIZE: u16 = 1000;
const DEFAULT_PROCESSOR_TASK_COUNT: u16 = 20;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct IndexerGrpcConfig {
    pub enabled: bool,

    /// The address that the grpc server will listen on
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,

    /// Number of processor tasks to fan out
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processor_task_count: Option<u16>,

    /// Number of transactions each processor will process
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processor_batch_size: Option<u16>,

    /// Number of transactions returned in a single stream response
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_batch_size: Option<u16>,
}

impl ConfigSanitizer for IndexerGrpcConfig {
    fn sanitize(
        node_config: &mut NodeConfig,
        _node_type: NodeType,
        _chain_id: ChainId,
    ) -> Result<(), Error> {
        let indexer_grpc_config = &mut node_config.indexer_grpc;

        // If the indexer is not enabled, we don't need to do anything
        if !indexer_grpc_config.enabled {
            return Ok(());
        }

        // Set appropriate defaults
        indexer_grpc_config.address = indexer_grpc_config
            .address
            .clone()
            .or_else(|| Some(DEFAULT_ADDRESS.into()));
        indexer_grpc_config.processor_task_count = indexer_grpc_config
            .processor_task_count
            .or(Some(DEFAULT_PROCESSOR_TASK_COUNT));
        indexer_grpc_config.processor_batch_size = indexer_grpc_config
            .processor_batch_size
            .or(Some(DEFAULT_PROCESSOR_BATCH_SIZE));
        indexer_grpc_config.output_batch_size = indexer_grpc_config
            .output_batch_size
            .or(Some(DEFAULT_OUTPUT_BATCH_SIZE));

        Ok(())
    }
}
