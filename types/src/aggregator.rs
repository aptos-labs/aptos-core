// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::{state_key::StateKey, table::TableHandle};
use move_core_types::account_address::AccountAddress;

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct AggregatorHandle(pub AccountAddress);

/// Uniquely identifies each aggregator instance in storage.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct AggregatorID(StateKey);

impl AggregatorID {
    pub fn new(handle: TableHandle, key: AggregatorHandle) -> Self {
        let state_key = StateKey::table_item(handle, key.0.to_vec());
        AggregatorID(state_key)
    }

    pub fn as_state_key(&self) -> &StateKey {
        &self.0
    }

    pub fn into_state_key(self) -> StateKey {
        self.0
    }
}
