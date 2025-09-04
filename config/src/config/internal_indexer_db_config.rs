// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::config::{
    config_sanitizer::ConfigSanitizer, node_config_loader::NodeType, Error, NodeConfig,
};
use velor_types::chain_id::ChainId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct InternalIndexerDBConfig {
    pub enable_transaction: bool,
    pub enable_event: bool,
    pub enable_event_v2_translation: bool,
    pub event_v2_translation_ignores_below_version: u64,
    pub enable_statekeys: bool,
    pub batch_size: usize,
}

impl InternalIndexerDBConfig {
    pub fn new(
        enable_transaction: bool,
        enable_event: bool,
        enable_event_v2_translation: bool,
        event_v2_translation_ignores_below_version: u64,
        enable_statekeys: bool,
        batch_size: usize,
    ) -> Self {
        Self {
            enable_transaction,
            enable_event,
            enable_event_v2_translation,
            event_v2_translation_ignores_below_version,
            enable_statekeys,
            batch_size,
        }
    }

    pub fn enable_transaction(&self) -> bool {
        self.enable_transaction
    }

    pub fn enable_event(&self) -> bool {
        self.enable_event
    }

    pub fn enable_event_v2_translation(&self) -> bool {
        self.enable_event_v2_translation
    }

    pub fn event_v2_translation_ignores_below_version(&self) -> u64 {
        self.event_v2_translation_ignores_below_version
    }

    pub fn enable_statekeys(&self) -> bool {
        self.enable_statekeys
    }

    pub fn is_internal_indexer_db_enabled(&self) -> bool {
        self.enable_transaction || self.enable_event || self.enable_statekeys
    }

    pub fn batch_size(&self) -> usize {
        self.batch_size
    }
}

impl Default for InternalIndexerDBConfig {
    fn default() -> Self {
        Self {
            enable_transaction: false,
            enable_event: false,
            enable_event_v2_translation: false,
            event_v2_translation_ignores_below_version: 0,
            enable_statekeys: false,
            batch_size: 10_000,
        }
    }
}

impl ConfigSanitizer for InternalIndexerDBConfig {
    fn sanitize(
        node_config: &NodeConfig,
        _node_type: NodeType,
        _chain_id: Option<ChainId>,
    ) -> Result<(), Error> {
        let sanitizer_name = Self::get_sanitizer_name();
        let config = node_config.indexer_db_config;

        // Shouldn't turn on internal indexer for db without sharding
        if !node_config.storage.rocksdb_configs.enable_storage_sharding
            && config.is_internal_indexer_db_enabled()
        {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                "Don't turn on internal indexer db if DB sharding is off".into(),
            ));
        }

        Ok(())
    }
}
