// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! Similar to `state_node_index`, this records the same node replacement information except that
//! the stale nodes here are the latest in at least one epoch.
//!
//! ```text
//! |<--------------key-------------->|
//! | stale_since_version | node_key |
//! ```
//!
//! `stale_since_version` is serialized in big endian so that records in RocksDB will be in order of
//! its numeric value.

use crate::schema::{
    ensure_slice_len_eq, ensure_slice_len_gt, STALE_NODE_INDEX_CROSS_EPOCH_CF_NAME,
};
use anyhow::Result;
use velor_jellyfish_merkle::{node_type::NodeKey, StaleNodeIndex};
use velor_schemadb::{
    define_schema,
    schema::{KeyCodec, SeekKeyCodec, ValueCodec},
};
use velor_types::transaction::Version;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::{io::Write, mem::size_of};

define_schema!(
    StaleNodeIndexCrossEpochSchema,
    StaleNodeIndex,
    (),
    STALE_NODE_INDEX_CROSS_EPOCH_CF_NAME
);

impl KeyCodec<StaleNodeIndexCrossEpochSchema> for StaleNodeIndex {
    fn encode_key(&self) -> Result<Vec<u8>> {
        let mut encoded = vec![];
        encoded.write_u64::<BigEndian>(self.stale_since_version)?;
        encoded.write_all(&self.node_key.encode()?)?;

        Ok(encoded)
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        const VERSION_SIZE: usize = size_of::<Version>();

        ensure_slice_len_gt(data, VERSION_SIZE)?;
        let stale_since_version = (&data[..VERSION_SIZE]).read_u64::<BigEndian>()?;
        let node_key = NodeKey::decode(&data[VERSION_SIZE..])?;

        Ok(Self {
            stale_since_version,
            node_key,
        })
    }
}

impl ValueCodec<StaleNodeIndexCrossEpochSchema> for () {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        ensure_slice_len_eq(data, 0)?;
        Ok(())
    }
}

impl SeekKeyCodec<StaleNodeIndexCrossEpochSchema> for Version {
    fn encode_seek_key(&self) -> Result<Vec<u8>> {
        Ok(self.to_be_bytes().to_vec())
    }
}

#[cfg(test)]
mod test;
