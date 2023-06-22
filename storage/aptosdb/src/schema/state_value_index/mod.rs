// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Historically we support an API that returns everything by key prefix (e.g. return all states
// under an account) operation. This was implemented by seeking the StateKv db by prefix directly.
// However in the new sharding world, account (or whatever prefix) is not a first class concept in
// the storage layer, and we will store data for an account in different shards, based on the hash
// of StateKey. Therefore the API cannot be supported by doing a single db seek.
//
// Our long term vision, is to move such support into indexer, before they are ready, we add this
// index for now to temporarily unblock the sharded db migration.

use crate::schema::{ensure_slice_len_eq, ensure_slice_len_gt, STATE_VALUE_INDEX_CF_NAME};
use anyhow::Result;
use aptos_schemadb::{
    define_schema,
    schema::{KeyCodec, SeekKeyCodec, ValueCodec},
};
use aptos_types::{
    state_store::{state_key::StateKey, state_key_prefix::StateKeyPrefix},
    transaction::Version,
};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::{io::Write, mem::size_of};

type Key = (StateKey, Version);

define_schema!(StateValueIndexSchema, Key, (), STATE_VALUE_INDEX_CF_NAME);

impl KeyCodec<StateValueIndexSchema> for Key {
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

impl ValueCodec<StateValueIndexSchema> for () {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        ensure_slice_len_eq(data, 0)?;
        Ok(())
    }
}

impl SeekKeyCodec<StateValueIndexSchema> for &StateKeyPrefix {
    fn encode_seek_key(&self) -> Result<Vec<u8>> {
        self.encode()
    }
}

#[cfg(test)]
mod test;
