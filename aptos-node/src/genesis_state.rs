// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::{anyhow, Result};
use aptos_config::{config::NodeConfig, utils::get_genesis_txn};
use aptos_storage_interface::{DbReader, Result as DbResult};
use aptos_types::{
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
    transaction::{Transaction, Version, WriteSetPayload},
};
use std::{collections::HashMap, sync::Arc};

/// Serves the genesis state straight out of the node's genesis blob.
///
/// A node that is about to fast sync holds no state of its own until the
/// snapshot lands, but some components cannot make progress without the
/// genesis configs: on-chain network discovery in particular needs the
/// validator set before it can connect to the peers it would fast sync from.
///
/// The genesis transaction is a direct write set, so it already contains the
/// full genesis state; no execution is needed to read it back.
pub struct GenesisStateReader {
    values: HashMap<StateKey, StateValue>,
}

impl GenesisStateReader {
    /// The genesis state is the state at version 0
    pub const VERSION: Version = 0;

    /// Builds a reader from the node's genesis blob, if it has one that carries
    /// state (i.e. a direct write set).
    pub fn new(node_config: &NodeConfig) -> Result<Self> {
        let genesis_txn = get_genesis_txn(node_config)
            .ok_or_else(|| anyhow!("No genesis txn provided in the node config!"))?;
        let Transaction::GenesisTransaction(WriteSetPayload::Direct(change_set)) = genesis_txn
        else {
            return Err(anyhow!("The genesis txn is not a direct write set!"));
        };

        let values = change_set
            .write_set()
            .write_op_iter()
            .filter_map(|(state_key, write_op)| {
                write_op
                    .bytes()
                    .map(|bytes| (state_key.clone(), StateValue::new_legacy(bytes.clone())))
            })
            .collect();
        Ok(Self { values })
    }
}

impl DbReader for GenesisStateReader {
    fn get_state_value_by_version(
        &self,
        state_key: &StateKey,
        _version: Version,
    ) -> DbResult<Option<StateValue>> {
        Ok(self.values.get(state_key).cloned())
    }

    fn get_state_value_with_version_by_version(
        &self,
        state_key: &StateKey,
        _version: Version,
    ) -> DbResult<Option<(Version, StateValue)>> {
        Ok(self
            .values
            .get(state_key)
            .map(|value| (Self::VERSION, value.clone())))
    }

    fn get_state_storage_usage(&self, _version: Option<Version>) -> DbResult<StateStorageUsage> {
        Ok(StateStorageUsage::new_untracked())
    }
}

/// Builds a genesis state reader, logging rather than failing when the node has
/// no usable genesis blob: such a node simply has to fast sync everything,
/// including genesis, from its peers.
pub fn try_new_genesis_state_reader(node_config: &NodeConfig) -> Option<Arc<dyn DbReader>> {
    match GenesisStateReader::new(node_config) {
        Ok(reader) => Some(Arc::new(reader) as Arc<dyn DbReader>),
        Err(error) => {
            aptos_logger::warn!(
                "Unable to read the genesis state from the node config: {}. \
                 The node will have no on-chain configs until it finishes bootstrapping.",
                error
            );
            None
        },
    }
}
