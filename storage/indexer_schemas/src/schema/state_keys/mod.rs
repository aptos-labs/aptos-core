// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{schema::STATE_KEYS_CF_NAME, utils::ensure_slice_len_eq};
use anyhow::Result;
use aptos_schemadb::{
    define_pub_schema,
    schema::{KeyCodec, SeekKeyCodec, ValueCodec},
};
use aptos_types::state_store::state_key::{StateKey, prefix::StateKeyPrefix};

define_pub_schema!(StateKeysSchema, StateKey, (), STATE_KEYS_CF_NAME);

impl KeyCodec<StateKeysSchema> for StateKey {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(self.encoded().to_vec())
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        let state_key: StateKey = StateKey::decode(data)?;
        Ok(state_key)
    }
}

impl ValueCodec<StateKeysSchema> for () {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        ensure_slice_len_eq(data, 0)?;
        Ok(())
    }
}

impl SeekKeyCodec<StateKeysSchema> for &StateKeyPrefix {
    fn encode_seek_key(&self) -> Result<Vec<u8>> {
        self.encode()
    }
}

#[cfg(test)]
mod test;
