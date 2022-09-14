// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module defines physical storage schema for various data associated with each transaction
//! version.
//!
//! ```text
//! |<--key-->|<--value->|
//! | version |   data   |
//! ```
//!
//! `Version` is serialized in big endian so that records in RocksDB will be in order of it's
//! numeric value.

use super::VERSION_DATA_CF_NAME;
use crate::schema::ensure_slice_len_eq;
use anyhow::Result;
use aptos_types::state_store::state_storage_usage::StateStorageUsage;
use aptos_types::transaction::Version;
use byteorder::{BigEndian, ReadBytesExt};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use schemadb::{
    define_schema,
    schema::{KeyCodec, ValueCodec},
};
use serde::{Deserialize, Serialize};
use std::mem::size_of;

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct VersionData {
    pub state_items: usize,
    pub total_state_bytes: usize,
}

impl From<StateStorageUsage> for VersionData {
    fn from(usage: StateStorageUsage) -> Self {
        Self {
            state_items: usage.items(),
            total_state_bytes: usage.bytes(),
        }
    }
}

impl VersionData {
    pub fn get_state_storage_usage(&self) -> StateStorageUsage {
        StateStorageUsage::new(self.state_items, self.total_state_bytes)
    }
}

define_schema!(
    VersionDataSchema,
    Version,
    VersionData,
    VERSION_DATA_CF_NAME
);

impl KeyCodec<VersionDataSchema> for Version {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(self.to_be_bytes().to_vec())
    }

    fn decode_key(mut data: &[u8]) -> Result<Self> {
        ensure_slice_len_eq(data, size_of::<Version>())?;
        Ok(data.read_u64::<BigEndian>()?)
    }
}

impl ValueCodec<VersionDataSchema> for VersionData {
    fn encode_value(&self) -> Result<Vec<u8>> {
        bcs::to_bytes(self).map_err(Into::into)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        bcs::from_bytes(data).map_err(Into::into)
    }
}

#[cfg(test)]
mod test;
