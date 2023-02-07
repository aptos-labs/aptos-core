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
    Serialized(Vec<u8>),
    // Not serialized Move value.
    MoveValue(Value),
}

/// Represents output data of VM session placed into MV hash map.
#[derive(Debug)]
pub enum OutputDataArced {
    // Legacy mode with serialized value.
    Serialized(Arc<Vec<u8>>),
    // Not serialized Move value.
    MoveValue(Arc<Value>),
}

pub trait Cache {
    type Target: Readable;
    fn cache(self) -> Self::Target;
}

pub trait Readable {
    fn read_ref(&self) -> Option<CachedData>;
    fn read(self) -> Option<CachedData>;
}

/// Represents cached data during transaction/block execution returned to the client.
#[derive(Debug)]
pub enum CachedData {
    /// Data cached as a pointer to a blob.
    Serialized(Arc<Vec<u8>>),
    /// Data cached as a pointer to any Move value.
    MoveValue(Arc<Value>),
}

// impl CachedData {
// Creates a new cached data from blobe.
// pub fn from_blob(blob: Vec<u8>) -> Self {
//     Self::Serialized(Arc::new(blob))
// }

// /// Creates a new cached data from blob.
// pub fn from_value(value: Value) -> Self {
//     Self::MoveValue(Arc::new(value))
// }

// /// Creates a new cached data from blob, usually from storage.
// pub fn from_u128(value: u128) -> Self {
//     Self::AggregatorValue(value)
// }
// }

// impl OutputData {
//     /// Adapter to view output data as cached data. Useful when when the client
//     /// only has a reference to the data but wants to treat it as a cache.
//     pub fn as_cached_data(&self) -> CachedData {
//         match self {
//             OutputData::Serialized(blob) => CachedData::Serialized(Arc::new(blob.clone())),
//             OutputData::MoveValue(value) => CachedData::MoveValue(Arc::new(value.copy_value().expect("copy should succeed."))),
//         }
//     }

//     /// Adapter to turn output data into cached data.
//     pub fn into_cached_data(self) -> CachedData {
//         match self {
//             OutputData::Serialized(blob) => CachedData::Serialized(Arc::new(blob)),
//             OutputData::MoveValue(value) => CachedData::MoveValue(value),
//         }
//     }
// }

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
