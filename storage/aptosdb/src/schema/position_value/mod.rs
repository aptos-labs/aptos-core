// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Physical storage schema for the native-position `position_value` CF.
//!
//! Hash-keyed, mirroring `state_value_by_key_hash` from the main-state
//! tier. The row key is the 32-byte `StateKey::hash()` (the same hash
//! the JMT uses as `account_key`) plus the bit-inverted version for
//! newest-first ordering.
//!
//! ```text
//! |<------------ key ------------>|<---- value ---->|
//! | state_key_hash (32B) | !version (8B BE) | Option<StateValue>
//! ```
//!
//! Reverse lookup (hash → StateKey) goes through `position_merkle_db`,
//! whose JMT leaves carry the original [`StateKey`] (see
//! [`crate::position_merkle_db`]). Cold-load + backup walk the JMT
//! iterator at the snapshot version to enumerate live positions, then
//! query this CF by hash to fetch values.

use crate::schema::{ensure_slice_len_eq, POSITION_VALUE_CF_NAME};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_schemadb::{
    define_schema,
    schema::{KeyCodec, ValueCodec},
};
use aptos_types::{state_store::state_value::StateValue, transaction::Version};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::{io::Write, mem::size_of};

pub type Key = (HashValue, Version);

define_schema!(
    PositionValueSchema,
    Key,
    Option<StateValue>,
    POSITION_VALUE_CF_NAME
);

impl KeyCodec<PositionValueSchema> for Key {
    fn encode_key(&self) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(HashValue::LENGTH + size_of::<Version>());
        out.write_all(self.0.as_ref())?;
        out.write_u64::<BigEndian>(!self.1)?;
        Ok(out)
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        const VERSION_SIZE: usize = size_of::<Version>();
        ensure_slice_len_eq(data, HashValue::LENGTH + VERSION_SIZE)?;
        let state_key_hash = HashValue::from_slice(&data[..HashValue::LENGTH])?;
        let version = !(&data[HashValue::LENGTH..]).read_u64::<BigEndian>()?;
        Ok((state_key_hash, version))
    }
}

impl ValueCodec<PositionValueSchema> for Option<StateValue> {
    fn encode_value(&self) -> Result<Vec<u8>> {
        bcs::to_bytes(self).map_err(Into::into)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        bcs::from_bytes(data).map_err(Into::into)
    }
}

#[cfg(test)]
mod test;
