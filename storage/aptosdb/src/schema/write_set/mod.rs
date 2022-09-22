// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module defines physical storage schema for write set emitted by each transaction
//! saved to storage.
//!
//! Serialized signed transaction bytes identified by version.
//! ```text
//! |<--key-->|<-----value----->|
//! | version | write_set bytes |
//! ```
//!
//! `Version` is serialized in big endian so that records in RocksDB will be in order of it's
//! numeric value.

use crate::schema::{ensure_slice_len_eq, WRITE_SET_CF_NAME};
use anyhow::Result;
use aptos_types::{transaction::Version, write_set::WriteSet};
use byteorder::{BigEndian, ReadBytesExt};
use schemadb::{
    define_schema,
    schema::{KeyCodec, ValueCodec},
};
use std::mem::size_of;

define_schema!(WriteSetSchema, Version, WriteSet, WRITE_SET_CF_NAME);

impl KeyCodec<WriteSetSchema> for Version {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(self.to_be_bytes().to_vec())
    }

    fn decode_key(mut data: &[u8]) -> Result<Self> {
        ensure_slice_len_eq(data, size_of::<Version>())?;
        Ok(data.read_u64::<BigEndian>()?)
    }
}

impl ValueCodec<WriteSetSchema> for WriteSet {
    fn encode_value(&self) -> Result<Vec<u8>> {
        bcs::to_bytes(self).map_err(Into::into)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        bcs::from_bytes(data).map_err(Into::into)
    }
}

#[cfg(test)]
mod test;
