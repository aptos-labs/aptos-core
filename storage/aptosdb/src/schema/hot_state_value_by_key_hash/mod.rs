// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines the physical storage schema for hot state value, it is
//! identical to the cold state, except that it needs a new column family.
//!
//! ```text
//! |<---------- key ----------->|<--- value --->|
//! |  state key hash | version  |  state value  |
//! ```

use crate::schema::{ensure_slice_len_eq, HOT_STATE_VALUE_BY_KEY_HASH_CF_NAME};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_schemadb::{
    define_schema,
    schema::{KeyCodec, ValueCodec},
};
use aptos_storage_interface::state_store::hot_state_value::HotStateValue;
use aptos_types::transaction::Version;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::{io::Write, mem::size_of};

/// `Version` is the version when this key is changed in hot state, could be
/// updated, refreshed, or evicted.
type Key = (HashValue, Version);

define_schema!(
    HotStateValueByKeyHashSchema,
    Key,
    Option<HotStateValue>, // None means being evicted.
    HOT_STATE_VALUE_BY_KEY_HASH_CF_NAME
);

impl KeyCodec<HotStateValueByKeyHashSchema> for Key {
    fn encode_key(&self) -> Result<Vec<u8>> {
        let mut encoded = vec![];
        encoded.write_all(self.0.as_ref())?;
        encoded.write_u64::<BigEndian>(!self.1)?;
        Ok(encoded)
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        const VERSION_SIZE: usize = size_of::<Version>();

        ensure_slice_len_eq(data, VERSION_SIZE + HashValue::LENGTH)?;
        let state_key_hash = HashValue::from_slice(&data[..HashValue::LENGTH])?;
        let version = !(&data[HashValue::LENGTH..]).read_u64::<BigEndian>()?;
        Ok((state_key_hash, version))
    }
}

impl ValueCodec<HotStateValueByKeyHashSchema> for Option<HotStateValue> {
    fn encode_value(&self) -> Result<Vec<u8>> {
        bcs::to_bytes(self).map_err(Into::into)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        bcs::from_bytes(data).map_err(Into::into)
    }
}

#[cfg(test)]
mod test;
