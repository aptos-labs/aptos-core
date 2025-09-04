// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines the physical storage schema for information related to outdated state
//! values, which are ready to be pruned after being old enough.
//!
//! An index entry in this data set has 3 pieces of information:
//!     1. The version since which a state value (in another data set) becomes stale, meaning,
//! replaced by an updated value.
//!     2. The version this state value was updated identified by the state key.
//!     3. The state_key to identify the stale state value.
//!
//! ```text
//! |<-------------------key------------------------>|
//! | stale_since_version | version | state_key_hash |
//! ```
//!
//! `stale_since_version` is serialized in big endian so that records in RocksDB will be in order of
//! its numeric value.

use crate::schema::{ensure_slice_len_eq, STALE_STATE_VALUE_INDEX_BY_KEY_HASH_CF_NAME};
use anyhow::Result;
use velor_crypto::HashValue;
use velor_schemadb::{
    define_schema,
    schema::{KeyCodec, SeekKeyCodec, ValueCodec},
};
use velor_types::{state_store::state_value::StaleStateValueByKeyHashIndex, transaction::Version};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::{io::Write, mem::size_of};

define_schema!(
    StaleStateValueIndexByKeyHashSchema,
    StaleStateValueByKeyHashIndex,
    (),
    STALE_STATE_VALUE_INDEX_BY_KEY_HASH_CF_NAME
);

impl KeyCodec<StaleStateValueIndexByKeyHashSchema> for StaleStateValueByKeyHashIndex {
    fn encode_key(&self) -> Result<Vec<u8>> {
        let mut encoded = vec![];
        encoded.write_u64::<BigEndian>(self.stale_since_version)?;
        encoded.write_u64::<BigEndian>(self.version)?;
        encoded.write_all(self.state_key_hash.as_ref())?;

        Ok(encoded)
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        const VERSION_SIZE: usize = size_of::<Version>();

        ensure_slice_len_eq(data, 2 * VERSION_SIZE + HashValue::LENGTH)?;
        let stale_since_version = (&data[..VERSION_SIZE]).read_u64::<BigEndian>()?;
        let version = (&data[VERSION_SIZE..2 * VERSION_SIZE]).read_u64::<BigEndian>()?;
        let state_key_hash = HashValue::from_slice(&data[2 * VERSION_SIZE..])?;

        Ok(Self {
            stale_since_version,
            version,
            state_key_hash,
        })
    }
}

impl ValueCodec<StaleStateValueIndexByKeyHashSchema> for () {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        ensure_slice_len_eq(data, 0)?;
        Ok(())
    }
}

impl SeekKeyCodec<StaleStateValueIndexByKeyHashSchema> for Version {
    fn encode_seek_key(&self) -> Result<Vec<u8>> {
        Ok(self.to_be_bytes().to_vec())
    }
}

#[cfg(test)]
mod test;
