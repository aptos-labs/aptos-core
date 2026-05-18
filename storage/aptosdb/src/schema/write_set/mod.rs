// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This module defines physical storage schema for write set emitted by each transaction
//! saved to storage.
//!
//! Serialized signed transaction bytes identified by version.
//! ```text
//! |<--key-->|<-----value----->|
//! | version | write_set bytes |
//! ```
//!
//! `Version` is serialized in big endian so that records in RocksDB will be in order of it's
//! numeric value.

use crate::schema::{ensure_slice_len_eq, WRITE_SET_CF_NAME};
use anyhow::Result;
use aptos_schemadb::{
    define_schema,
    schema::{KeyCodec, ValueCodec},
};
use aptos_types::{
    state_store::state_key::StateKey,
    transaction::Version,
    write_set::{HotStateOp, ValueWriteSet, WriteSet, WriteSetV0},
};
use byteorder::{BigEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, mem::size_of};

define_schema!(WriteSetSchema, Version, WriteSet, WRITE_SET_CF_NAME);

impl KeyCodec<WriteSetSchema> for Version {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(self.to_be_bytes().to_vec())
    }

    fn decode_key(mut data: &[u8]) -> Result<Self> {
        ensure_slice_len_eq(data, size_of::<Version>())?;
        Ok(data.read_u64::<BigEndian>()?)
    }
}

/// Storage-only serialization format for `WriteSet`. Separate from `ValueWriteSet` so that the
/// `BCSCryptoHash` (which goes through `WriteSet::Serialize` → `ValueWriteSet`) is not affected
/// for now.
#[derive(Serialize, Deserialize)]
enum PersistedWriteSet {
    /// Legacy format: identical BCS layout to `ValueWriteSet::V0`.
    V0(WriteSetV0),
    /// Extended format that also persists the set of hot state keys.
    V1 {
        value: WriteSetV0,
        hotness: BTreeSet<StateKey>,
    },
}

/// Encode a `WriteSet` for storage. When `persist_hotness` is true, produces the V1 format
/// (which includes the set of hot state keys); otherwise produces the legacy V0 format
/// (byte-identical to `bcs::to_bytes(write_set)`).
pub(crate) fn encode_write_set(
    write_set: &WriteSet,
    persist_hotness: bool,
) -> bcs::Result<Vec<u8>> {
    if persist_hotness {
        let persisted = PersistedWriteSet::V1 {
            value: write_set.as_v0().clone(),
            hotness: write_set.hotness_keys().cloned().collect(),
        };
        bcs::to_bytes(&persisted)
    } else {
        // Delegates to WriteSet::Serialize → ValueWriteSet → V0 layout.
        bcs::to_bytes(write_set)
    }
}

/// Decode a `WriteSet` from storage, handling both V0 (legacy) and V1 (with hotness) formats.
fn decode_write_set(data: &[u8]) -> bcs::Result<WriteSet> {
    match bcs::from_bytes(data)? {
        PersistedWriteSet::V0(ws_v0) => Ok(WriteSet::new_from_value(ValueWriteSet::V0(ws_v0))),
        PersistedWriteSet::V1 { value, hotness } => Ok(WriteSet::new_from_value_with_hotness(
            ValueWriteSet::V0(value),
            hotness
                .into_iter()
                .map(|key| (key, HotStateOp::make_hot()))
                .collect(),
        )),
    }
}

impl ValueCodec<WriteSetSchema> for WriteSet {
    fn encode_value(&self) -> Result<Vec<u8>> {
        bcs::to_bytes(self).map_err(Into::into)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        decode_write_set(data).map_err(Into::into)
    }
}

#[cfg(test)]
mod test;
