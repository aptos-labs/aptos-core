// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::{state_key::StateKey, table::TableHandle};
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AggregatorResource<T> {
    value: T,
    max_value: T,
}

impl<T> AggregatorResource<T> {
    pub fn new(value: T, max_value: T) -> Self {
        Self { value, max_value }
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    pub fn set(&mut self, value: T) {
        self.value = value;
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggregatorSnapshotResource<T> {
    pub value: T,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DerivedStringSnapshotResource {
    value: String,
    padding: Vec<u8>,
}

/// Deprecated:

/// Rust representation of Aggregator Move struct.
#[derive(Debug, Serialize, Deserialize)]
pub struct AggregatorV1Resource {
    handle: AccountAddress,
    key: AccountAddress,
    limit: u128,
}

impl AggregatorV1Resource {
    pub fn new(handle: AccountAddress, key: AccountAddress, limit: u128) -> Self {
        Self { handle, key, limit }
    }

    /// Helper function to return the state key where the actual value is stored.
    pub fn state_key(&self) -> StateKey {
        StateKey::table_item(&TableHandle(self.handle), self.key.as_ref())
    }
}

/// Rust representation of Integer Move struct.
#[derive(Debug, Serialize, Deserialize)]
pub struct IntegerResource {
    pub value: u128,
    limit: u128,
}

/// Rust representation of OptionalAggregator Move struct.
#[derive(Debug, Serialize, Deserialize)]
pub struct OptionalAggregatorV1Resource {
    pub aggregator: Option<AggregatorV1Resource>,
    pub integer: Option<IntegerResource>,
}
