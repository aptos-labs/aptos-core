// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::state_store::state_key::StateKey;
use move_core_types::value::MoveTypeLayout;
use move_vm_types::values::Value;
use std::sync::Arc;

#[derive(Debug)]
pub enum OutputData {
    Serialized(Vec<u8>),
    MoveValue(Arc<Value>, MoveTypeLayout),
}

impl OutputData {
    pub fn as_cached_data(&self) -> CachedData {
        match self {
            OutputData::Serialized(blob) => CachedData::Serialized(Arc::new(blob.clone())),
            OutputData::MoveValue(value, _) => CachedData::MoveValue(Arc::clone(value)),
        }
    }

    pub fn into_cached_data(self) -> CachedData {
        match self {
            OutputData::Serialized(blob) => CachedData::Serialized(Arc::new(blob)),
            OutputData::MoveValue(value, _) => CachedData::MoveValue(value),
        }
    }
}

// TODO: Conversion from Move output type.

#[derive(Debug)]
pub enum CachedData {
    Serialized(Arc<Vec<u8>>),
    MoveValue(Arc<Value>),
}

// TODO: Conversion into Move input type.

pub trait Cache: Sized {
    fn cache(&self) -> Self;
}

impl Cache for CachedData {
    fn cache(&self) -> CachedData {
        match self {
            CachedData::Serialized(blob) => CachedData::Serialized(Arc::clone(blob)),
            CachedData::MoveValue(value) => CachedData::MoveValue(Arc::clone(value)),
        }
    }
}

pub trait DataCache {
    type Key;

    fn get_value(&self, key: &Self::Key) -> anyhow::Result<Option<CachedData>>;
}

pub trait AptosDataCache: DataCache<Key = StateKey> {}

impl<T: DataCache<Key = StateKey>> AptosDataCache for T {}
