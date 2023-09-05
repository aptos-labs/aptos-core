// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::{state_key::StateKey, table::TableHandle};
use move_core_types::account_address::AccountAddress;

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct AggregatorHandle(pub AccountAddress);

/// Uniquely identifies each aggregator instance in storage.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum AggregatorID {
    // Aggregator V1 is implemented as a state item, and so can be queried by
    // the state key.
    Legacy(StateKey),
    // Aggregator V2 is embedded into resources, and uses ephemeral identifiers
    // which are unique per block.
    Ephemeral(u64),
}

impl AggregatorID {
    pub fn legacy(handle: TableHandle, key: AggregatorHandle) -> Self {
        let state_key = StateKey::table_item(handle, key.0.to_vec());
        AggregatorID::Legacy(state_key)
    }

    pub fn ephemeral(id: u64) -> Self {
        AggregatorID::Ephemeral(id)
    }

    pub fn as_state_key(&self) -> Option<&StateKey> {
        match self {
            Self::Legacy(state_key) => Some(state_key),
            Self::Ephemeral(_) => None,
        }
    }

    pub fn into_state_key(self) -> Option<StateKey> {
        match self {
            Self::Legacy(state_key) => Some(state_key),
            Self::Ephemeral(_) => None,
        }
    }
}
