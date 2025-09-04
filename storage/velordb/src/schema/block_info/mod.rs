// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines physical storage schema for block info.
//!
//! ```text
//! |<-----key----->|<---value--->|
//! |  block_height |  block_info |
//! ```

use crate::schema::{ensure_slice_len_eq, BLOCK_INFO_CF_NAME};
use anyhow::Result;
use velor_schemadb::{
    define_schema,
    schema::{KeyCodec, ValueCodec},
};
use velor_storage_interface::block_info::BlockInfo;
use velor_types::block_info::BlockHeight;
use byteorder::{BigEndian, ReadBytesExt};
use std::mem::size_of;

type Key = BlockHeight;
type Value = BlockInfo;

define_schema!(BlockInfoSchema, Key, Value, BLOCK_INFO_CF_NAME);

impl KeyCodec<BlockInfoSchema> for Key {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(self.to_be_bytes().to_vec())
    }

    fn decode_key(mut data: &[u8]) -> Result<Self> {
        ensure_slice_len_eq(data, size_of::<Self>())?;
        Ok(data.read_u64::<BigEndian>()?)
    }
}

impl ValueCodec<BlockInfoSchema> for Value {
    fn encode_value(&self) -> Result<Vec<u8>> {
        bcs::to_bytes(self).map_err(Into::into)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        bcs::from_bytes(data).map_err(Into::into)
    }
}

#[cfg(test)]
mod test;
