// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines physical storage schema for PersistedAuxiliaryInfo structure.
//!
//! Serialized PersistedAuxiliaryInfo bytes keyed by transaction version.
//! ```text
//! |<--key-->|<------------value------------->|
//! | version | persisted_auxiliary_info bytes |
//! ```
//!
//! `Version` is serialized in big endian so that records in RocksDB will be in order of it's
//! numeric value.

use crate::schema::{ensure_slice_len_eq, PERSISTED_AUXILIARY_INFO_CF_NAME};
use anyhow::Result;
use velor_schemadb::{
    define_schema,
    schema::{KeyCodec, ValueCodec},
};
use velor_types::transaction::{PersistedAuxiliaryInfo, Version};
use byteorder::{BigEndian, ReadBytesExt};
use std::mem::size_of;

define_schema!(
    PersistedAuxiliaryInfoSchema,
    Version,
    PersistedAuxiliaryInfo,
    PERSISTED_AUXILIARY_INFO_CF_NAME
);

impl KeyCodec<PersistedAuxiliaryInfoSchema> for Version {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(self.to_be_bytes().to_vec())
    }

    fn decode_key(mut data: &[u8]) -> Result<Self> {
        ensure_slice_len_eq(data, size_of::<Version>())?;
        Ok(data.read_u64::<BigEndian>()?)
    }
}

impl ValueCodec<PersistedAuxiliaryInfoSchema> for PersistedAuxiliaryInfo {
    fn encode_value(&self) -> Result<Vec<u8>> {
        bcs::to_bytes(self).map_err(Into::into)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        bcs::from_bytes(data).map_err(Into::into)
    }
}

#[cfg(test)]
mod test;
