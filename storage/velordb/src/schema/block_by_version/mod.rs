// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines physical storage schema for an index to help us fine out which block a
//! ledger version is in, by storing a block_start_version <-> block_height pair.
//!
//! ```text
//! |<--------key-------->|<---value---->|
//! | block_start_version | block_height |
//! ```

use crate::schema::{ensure_slice_len_eq, BLOCK_BY_VERSION_CF_NAME};
use anyhow::Result;
use velor_schemadb::{
    define_schema,
    schema::{KeyCodec, ValueCodec},
};
use velor_types::{block_info::BlockHeight, transaction::Version};
use byteorder::{BigEndian, ReadBytesExt};
use std::mem::size_of;

type Key = Version;
type Value = BlockHeight;

define_schema!(BlockByVersionSchema, Key, Value, BLOCK_BY_VERSION_CF_NAME);

impl KeyCodec<BlockByVersionSchema> for Key {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(self.to_be_bytes().to_vec())
    }

    fn decode_key(mut data: &[u8]) -> Result<Self> {
        ensure_slice_len_eq(data, size_of::<Self>())?;
        Ok(data.read_u64::<BigEndian>()?)
    }
}

impl ValueCodec<BlockByVersionSchema> for Value {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(self.to_be_bytes().to_vec())
    }

    fn decode_value(mut data: &[u8]) -> Result<Self> {
        ensure_slice_len_eq(data, size_of::<Self>())?;
        Ok(data.read_u64::<BigEndian>()?)
    }
}

#[cfg(test)]
mod test;
