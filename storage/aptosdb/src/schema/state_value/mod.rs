// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module defines the physical storage schema for state value, which is used
//! to access the state value directly.
//!
//! An Index Key in this data set has 2 pieces of information:
//!     1. The state key
//!     2. The version associated with the key
//! The value associated with the key is the serialized State Value.
//!
//! ```text
//! |<-------- key -------->|<--- value --->|
//! |  state key  | version |  state value  |
//! ```

use crate::schema::{ensure_slice_len_gt, STATE_VALUE_CF_NAME};
use anyhow::Result;
use aptos_types::{
    state_store::{state_key::StateKey, state_key_prefix::StateKeyPrefix, state_value::StateValue},
    transaction::Version,
};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use schemadb::{
    define_schema,
    schema::{KeyCodec, SeekKeyCodec, ValueCodec},
};
use std::{io::Write, mem::size_of};

type Key = (StateKey, Version);

define_schema!(
    StateValueSchema,
    Key,
    Option<StateValue>,
    STATE_VALUE_CF_NAME
);

impl KeyCodec<StateValueSchema> for Key {
    fn encode_key(&self) -> Result<Vec<u8>> {
        let mut encoded = vec![];
        encoded.write_all(&self.0.encode()?)?;
        encoded.write_u64::<BigEndian>(!self.1)?;
        Ok(encoded)
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        const VERSION_SIZE: usize = size_of::<Version>();

        ensure_slice_len_gt(data, VERSION_SIZE)?;
        let state_key_len = data.len() - VERSION_SIZE;
        let state_key: StateKey = StateKey::decode(&data[..state_key_len])?;
        let version = !(&data[state_key_len..]).read_u64::<BigEndian>()?;
        Ok((state_key, version))
    }
}

impl ValueCodec<StateValueSchema> for Option<StateValue> {
    fn encode_value(&self) -> Result<Vec<u8>> {
        bcs::to_bytes(self).map_err(Into::into)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        bcs::from_bytes(data).map_err(Into::into)
    }
}

impl SeekKeyCodec<StateValueSchema> for &StateKeyPrefix {
    fn encode_seek_key(&self) -> Result<Vec<u8>> {
        self.encode()
    }
}

#[cfg(test)]
mod test;
