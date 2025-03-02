// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines physical storage schema for an event index by its type.
//!
//! ```text
//! |<------------key----------->|<-value->|
//! | event_type | txn_ver | idx |   N/A   |
//! ```

use crate::{
    schema::EVENT_BY_TYPE_CF_NAME,
    utils::{ensure_slice_len_eq, ensure_slice_len_gt},
};
use anyhow::Result;
use aptos_schemadb::{
    define_pub_schema,
    schema::{KeyCodec, ValueCodec},
};
use aptos_types::transaction::Version;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use move_core_types::language_storage::TypeTag;
use std::mem::size_of;

define_pub_schema!(EventByTypeSchema, Key, Value, EVENT_BY_TYPE_CF_NAME);

pub(super) type Index = u16;

type Key = (TypeTag, Version, Index);
type Value = ();

impl KeyCodec<EventByTypeSchema> for Key {
    fn encode_key(&self) -> Result<Vec<u8>> {
        let mut encoded = bcs::to_bytes(&self.0)?;
        encoded.write_u64::<BigEndian>(self.1)?;
        encoded.write_u16::<BigEndian>(self.2)?;

        Ok(encoded)
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        let version_len = size_of::<Version>();
        let index_len = size_of::<Index>();
        ensure_slice_len_gt(data, version_len + index_len)?;

        let type_tag_len = data.len() - version_len - index_len;

        let type_tag = bcs::from_bytes(&data[..type_tag_len])?;
        let version = (&data[type_tag_len..]).read_u64::<BigEndian>()?;
        let index = (&data[type_tag_len + version_len..]).read_u16::<BigEndian>()?;

        Ok((type_tag, version, index))
    }
}

impl ValueCodec<EventByTypeSchema> for Value {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(vec![])
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        ensure_slice_len_eq(data, 0)?;
        Ok(())
    }
}

#[cfg(test)]
mod test;
