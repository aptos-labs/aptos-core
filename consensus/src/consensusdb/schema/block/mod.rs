// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This module defines physical storage schema for consensus block.
//!
//! Serialized block bytes identified by block_hash.
//! ```text
//! |<---key---->|<---value--->|
//! | block_hash |    block    |
//! ```

use crate::define_schema;
use anyhow::Result;
use velor_consensus_types::block::Block;
use velor_crypto::HashValue;
use velor_schemadb::{
    schema::{KeyCodec, ValueCodec},
    ColumnFamilyName,
};

pub const BLOCK_CF_NAME: ColumnFamilyName = "block";

define_schema!(BlockSchema, HashValue, Block, BLOCK_CF_NAME);

impl KeyCodec<BlockSchema> for HashValue {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(self.to_vec())
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        Ok(HashValue::from_slice(data)?)
    }
}

impl ValueCodec<BlockSchema> for Block {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self)?)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

#[cfg(test)]
mod test;
