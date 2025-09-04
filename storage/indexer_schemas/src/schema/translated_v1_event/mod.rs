// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This module defines physical storage schema for the contract events.
//!
//! A translated v1 event is keyed by the version of the transaction it belongs to and the index of
//! the original v2 event among all events yielded by the same transaction.
//! ```text
//! |<-------key----->|<---value--->|
//! | version | index | event bytes |
//! ```

use crate::schema::{TRANSLATED_V1_EVENT_CF_NAME, ensure_slice_len_eq};
use anyhow::Result;
use aptos_schemadb::{
    define_pub_schema,
    schema::{KeyCodec, ValueCodec},
};
use aptos_types::{contract_event::ContractEventV1, transaction::Version};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::mem::size_of;

define_pub_schema!(
    TranslatedV1EventSchema,
    Key,
    ContractEventV1,
    TRANSLATED_V1_EVENT_CF_NAME
);

type Index = u64;
type Key = (Version, Index);

impl KeyCodec<TranslatedV1EventSchema> for Key {
    fn encode_key(&self) -> Result<Vec<u8>> {
        let (version, index) = *self;

        let mut encoded_key = Vec::with_capacity(size_of::<Version>() + size_of::<Index>());
        encoded_key.write_u64::<BigEndian>(version)?;
        encoded_key.write_u64::<BigEndian>(index)?;
        Ok(encoded_key)
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        ensure_slice_len_eq(data, size_of::<Self>())?;

        let version_size = size_of::<Version>();

        let version = (&data[..version_size]).read_u64::<BigEndian>()?;
        let index = (&data[version_size..]).read_u64::<BigEndian>()?;
        Ok((version, index))
    }
}

impl ValueCodec<TranslatedV1EventSchema> for ContractEventV1 {
    fn encode_value(&self) -> Result<Vec<u8>> {
        bcs::to_bytes(self).map_err(Into::into)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        bcs::from_bytes(data).map_err(Into::into)
    }
}

#[cfg(test)]
mod test;
