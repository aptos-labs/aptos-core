// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::state_store::state_key::StateKey;
use move_core_types::value::MoveTypeLayout;
use move_vm_types::values::Value;
use std::sync::Arc;

/// Represents output data of VM session.
#[derive(Debug)]
pub enum OutputData {
    // Legacy mode with serialized value.
    Serialized(Arc<Vec<u8>>),
    // Not serialized Move value with its type layout.
    MoveValue(Arc<Value>, MoveTypeLayout),
}

impl OutputData {
    /// Adapter to view output data as cached data. Useful when when the client
    /// only has a reference to the data but wants to treat it as a cache.
    pub fn as_cached_data(&self) -> CachedData {
        match self {
            OutputData::Serialized(blob) => CachedData::Serialized(Arc::clone(blob)),
            OutputData::MoveValue(value, _) => CachedData::MoveValue(Arc::clone(value)),
        }
    }

    /// Adapter to turn output data into cached data.
    pub fn into_cached_data(self) -> CachedData {
        match self {
            OutputData::Serialized(blob) => CachedData::Serialized(blob),
            OutputData::MoveValue(value, _) => CachedData::MoveValue(value),
        }
    }
}

/// Represents cached data during transaction/block execution.
#[derive(Debug)]
pub enum CachedData {
    /// Data cached as a pointer to a blob.
    Serialized(Arc<Vec<u8>>),
    /// Data cached as a pointer to a Move value.
    /// TODO: We may want to store more information here?
    MoveValue(Arc<Value>),

    AggregatorValue(u128),
}

impl CachedData {
    /// Creates a new cached data from blob, usually from storage.
    pub fn from_blob(blob: Vec<u8>) -> Self {
        Self::Serialized(Arc::new(blob))
    }
}


/// Trait to define any cache built on top of the storage.
pub trait DataCache {
    /// Key for this data in storage.
    type Key;
    /// A deserialization hint (such as type information, etc.) in order to
    /// immediately deserialize data from storage on cache miss.
    type DeserializerHint;

    /// Returns the value from te cache or global storage if it is not in the
    /// cache. It is up to implementation to put the value in cache on cache
    /// miss.
    fn get_value(
        &self,
        key: &Self::Key,
        hint: Option<&Self::DeserializerHint>,
    ) -> anyhow::Result<Option<CachedData>>;
}

pub trait AptosDataCache: DataCache<Key = StateKey, DeserializerHint = MoveTypeLayout> {}

impl<T: DataCache<Key = StateKey, DeserializerHint = MoveTypeLayout>> AptosDataCache for T {}
