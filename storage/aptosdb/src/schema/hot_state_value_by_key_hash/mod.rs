// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines the physical storage schema for hot state value, it is
//! similar to the cold state, but note that the version here means
//! `hot_since_version`, which indicates when the value becomes hot or gets
//! refreshed in the hot state.
//!
//! ```text
//! |<---------- key ----------->|<--- value --->|
//! |  state key hash | version  |  state value  |
//! ```

use crate::schema::{HOT_STATE_VALUE_BY_KEY_HASH_CF_NAME, ensure_slice_len_eq};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_schemadb::{
    define_schema,
    schema::{KeyCodec, ValueCodec},
};
use aptos_types::{state_store::state_value::StateValue, transaction::Version};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};
use std::{io::Write, mem::size_of};

/// `Version` is the version when this key is changed in hot state, could be
/// updated, refreshed, or evicted.
type Key = (HashValue, Version);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub(crate) enum HotStateValue {
    Occupied {
        value_version: Version,
        value: StateValue,
    },
    Vacant,
}
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
