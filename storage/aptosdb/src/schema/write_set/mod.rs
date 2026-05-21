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
    write_set::{WriteSet, WriteSetV0},
};
use byteorder::{BigEndian, ReadBytesExt};
use serde::Deserialize;
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

/// Legacy V1 payload (without the enum tag byte) from binaries that pre-date the
/// `pub enum WriteSet { V0, V1 }` representation.
/// TODO(HotState): this is only needed temporarily to avoid forge-compat test failures because in
/// these tests the baseline validators would write legacy format to DB.
#[derive(Deserialize)]
struct LegacyWriteSetV1Payload {
    value: WriteSetV0,
    hotness: BTreeSet<StateKey>,
}

impl ValueCodec<WriteSetSchema> for WriteSet {
    fn encode_value(&self) -> Result<Vec<u8>> {
        bcs::to_bytes(self).map_err(Into::into)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        // TODO(HotState): we could simply use `bcs::from_bytes` for everything once the latest
        // release branch does not write LegacyWriteSetV1Payload and forge-compat test would not
        // fail in CI.
        match data.first() {
            Some(&1) => match bcs::from_bytes::<WriteSet>(data) {
                Ok(ws) => Ok(ws),
                // Legacy V1 payload lacks the trailing `extensions` field, so the new decoder
                // runs out of bytes while reading it. Any other error indicates real corruption
                // and should propagate.
                Err(bcs::Error::Eof) => {
                    let legacy: LegacyWriteSetV1Payload = bcs::from_bytes(&data[1..])?;
                    let mut ws = WriteSet::V0(legacy.value);
                    ws.add_hotness(legacy.hotness);
                    Ok(ws)
                },
                Err(e) => Err(e.into()),
            },
            _ => bcs::from_bytes::<WriteSet>(data).map_err(Into::into),
        }
    }
}

#[cfg(test)]
mod test;
