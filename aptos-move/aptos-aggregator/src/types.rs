// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::state_store::{state_key::StateKey, table::TableHandle};
use move_core_types::account_address::AccountAddress;

pub type AggregatorResult<T> = Result<T, AggregatorError>;

// TODO: Use this instead of PartialVM errors.
#[derive(Debug)]
pub enum AggregatorError {
    WrongVersionID,
}

/// Ephemeral identifier type used by aggregators V2.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct AggregatorID(u64);

impl AggregatorID {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn id(&self) -> u64 {
        self.0
    }
}

/// Uniquely identifies aggregator or aggregator snapshot instances in
/// extension and possibly storage.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum AggregatorVersionedID {
    // Aggregator V1 is implemented as a state item, and so can be queried by
    // the state key.
    V1(StateKey),
    // Aggregator V2 is embedded into resources, and uses ephemeral identifiers
    // which are unique per block.
    V2(AggregatorID),
}

impl AggregatorVersionedID {
    pub fn v1(handle: TableHandle, key: AccountAddress) -> Self {
        let state_key = StateKey::table_item(handle, key.to_vec());
        Self::V1(state_key)
    }

    pub fn v2(value: u64) -> Self {
        Self::V2(AggregatorID::new(value))
    }
}

impl TryFrom<AggregatorVersionedID> for StateKey {
    type Error = AggregatorError;

    fn try_from(vid: AggregatorVersionedID) -> Result<Self, Self::Error> {
        match vid {
            AggregatorVersionedID::V1(state_key) => Ok(state_key),
            AggregatorVersionedID::V2(_) => Err(AggregatorError::WrongVersionID),
        }
    }
}
