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
pub type AggregatorID = u64;

/// Uniquely identifies aggregator or aggregator snapshot instances in
/// extension and possibly storage.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum VersionedID {
    // Aggregator V1 is implemented as a state item, and so can be queried by
    // the state key.
    V1(StateKey),
    // Aggregator V2 is embedded into resources, and uses ephemeral identifiers
    // which are unique per block.
    V2(AggregatorID),
}

impl VersionedID {
    pub fn legacy(handle: TableHandle, key: AccountAddress) -> Self {
        let state_key = StateKey::table_item(handle, key.to_vec());
        VersionedID::V1(state_key)
    }
}

impl TryFrom<VersionedID> for StateKey {
    type Error = AggregatorError;

    fn try_from(vid: VersionedID) -> Result<Self, Self::Error> {
        match vid {
            VersionedID::V1(state_key) => Ok(state_key),
            VersionedID::V2(_) => Err(AggregatorError::WrongVersionID),
        }
    }
}
