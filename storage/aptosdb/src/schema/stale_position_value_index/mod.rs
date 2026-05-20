// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Physical storage schema for the native-position
//! `stale_position_value_index` CF. Hash-keyed, mirroring
//! `stale_state_value_index_by_key_hash` from the main-state tier —
//! same `(stale_since_version, version, state_key_hash)` shape, so
//! the position pruner reuses
//! [`aptos_types::state_store::state_value::StaleStateValueByKeyHashIndex`]
//! directly under the local alias [`StalePositionValueIndex`]. Only
//! the CF and the schema type differ; the byte format is shared and
//! the [`KeyCodec`] impl below is byte-identical to the main-state
//! schema's.
//!
//! Drives the position-value pruner. When a new Position write
//! supersedes an older row at version `version`, this index emits an
//! entry with `stale_since_version` = the new-write version,
//! `version` = the superseded-row version, and the 32-byte
//! `state_key_hash`.
//!
//! ```text
//! |<--------- key ----------------------------->|<-value->|
//! | stale_since_version | version | key_hash    |   ()    |
//! ```
//!
//! `stale_since_version` is big-endian so RocksDB orders records
//! numerically; the pruner seeks up to a horizon via the
//! [`SeekKeyCodec<Version>`] impl below.

use crate::schema::STALE_POSITION_VALUE_INDEX_CF_NAME;
use anyhow::{ensure, Result};
use aptos_crypto::HashValue;
use aptos_schemadb::{
    define_schema,
    schema::{KeyCodec, SeekKeyCodec, ValueCodec},
};
pub use aptos_types::state_store::state_value::StaleStateValueByKeyHashIndex as StalePositionValueIndex;
use aptos_types::transaction::Version;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::{io::Write, mem::size_of};

define_schema!(
    StalePositionValueIndexSchema,
    StalePositionValueIndex,
    (),
    STALE_POSITION_VALUE_INDEX_CF_NAME
);

impl KeyCodec<StalePositionValueIndexSchema> for StalePositionValueIndex {
    fn encode_key(&self) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(2 * size_of::<Version>() + HashValue::LENGTH);
        out.write_u64::<BigEndian>(self.stale_since_version)?;
        out.write_u64::<BigEndian>(self.version)?;
        out.write_all(self.state_key_hash.as_ref())?;
        Ok(out)
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        const VERSION_SIZE: usize = size_of::<Version>();
        ensure!(
            data.len() == 2 * VERSION_SIZE + HashValue::LENGTH,
            "stale_position_value_index key: unexpected length {}",
            data.len(),
        );
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

impl ValueCodec<StalePositionValueIndexSchema> for () {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        ensure!(
            data.is_empty(),
            "stale_position_value_index value: expected empty"
        );
        Ok(())
    }
}

/// Pruner-side seek key: encodes just the `stale_since_version` so
/// the pruner can seek to "first row with stale_since_version >= X"
/// without constructing the full composite key.
impl SeekKeyCodec<StalePositionValueIndexSchema> for Version {
    fn encode_seek_key(&self) -> Result<Vec<u8>> {
        Ok(self.to_be_bytes().to_vec())
    }
}

#[cfg(test)]
mod test;
