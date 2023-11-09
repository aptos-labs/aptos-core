// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This module defines physical storage schema for a block index.
//!
//! ```text
//! |<-----key----->|<---------value--------->|
//! |  block_height |   block_start_version   |
//! ```

use crate::schema::{ensure_slice_len_eq, BLOCK_INDEX_CF_NAME};
use anyhow::Result;
use aptos_schemadb::{
    define_schema,
    schema::{KeyCodec, ValueCodec},
};
use aptos_types::transaction::Version;
use byteorder::{BigEndian, ReadBytesExt};
use std::mem::size_of;

type BlockHeight = u64;
type Key = BlockHeight;
type Value = Version;

define_schema!(BlockIndexSchema, Key, Value, BLOCK_INDEX_CF_NAME);

impl KeyCodec<BlockIndexSchema> for Key {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(self.to_be_bytes().to_vec())
    }

    fn decode_key(mut data: &[u8]) -> Result<Self> {
        ensure_slice_len_eq(data, size_of::<Self>())?;
        Ok(data.read_u64::<BigEndian>()?)
    }
}

impl ValueCodec<BlockIndexSchema> for Value {
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
