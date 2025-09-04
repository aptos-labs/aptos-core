// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines the physical storage schema for state value, which is used
//! to access the state value directly.
//!
//! An Index Key in this data set has 2 pieces of information:
//!     1. The state key hash
//!     2. The version associated with the key
//! The value associated with the key is the serialized State Value.
//!
//! ```text
//! |<-------- key -------->|<------ value ---->|
//! |  state key hash | version |  state value  |
//! ```

use crate::schema::{ensure_slice_len_eq, STATE_VALUE_BY_KEY_HASH_CF_NAME};
use anyhow::Result;
use velor_crypto::HashValue;
use velor_schemadb::{
    define_schema,
    schema::{KeyCodec, ValueCodec},
};
use velor_types::{state_store::state_value::StateValue, transaction::Version};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::{io::Write, mem::size_of};

type Key = (HashValue, Version);

define_schema!(
    StateValueByKeyHashSchema,
    Key,
    Option<StateValue>,
    STATE_VALUE_BY_KEY_HASH_CF_NAME
);

impl KeyCodec<StateValueByKeyHashSchema> for Key {
    fn encode_key(&self) -> Result<Vec<u8>> {
        let mut encoded = vec![];
        encoded.write_all(self.0.as_ref())?;
        encoded.write_u64::<BigEndian>(!self.1)?;
        Ok(encoded)
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        const VERSION_SIZE: usize = size_of::<Version>();

        ensure_slice_len_eq(data, VERSION_SIZE + HashValue::LENGTH)?;
        let state_key_hash: HashValue = HashValue::from_slice(&data[..HashValue::LENGTH])?;
        let version = !(&data[HashValue::LENGTH..]).read_u64::<BigEndian>()?;
        Ok((state_key_hash, version))
    }
}

impl ValueCodec<StateValueByKeyHashSchema> for Option<StateValue> {
    fn encode_value(&self) -> Result<Vec<u8>> {
        bcs::to_bytes(self).map_err(Into::into)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        bcs::from_bytes(data).map_err(Into::into)
    }
}

#[cfg(test)]
mod test;
