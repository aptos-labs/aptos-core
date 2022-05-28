// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module defines the physical storage schema for state value, which is used
//! to access the state value directly.
//!
//! An Index Key in this data set has 2 pieces of information:
//!     1. The state key
//!     2. The version associated with the key
//! The value associated with the key is the the serialized State Value.
//!
//! //! ```text
//! |<-------- key -------->|<-- value --->|
//! |  state key  | version |  state value  |
//! ```

use crate::schema::LATEST_STATE_VALUE_CF_NAME;
use anyhow::Result;
use aptos_types::state_store::{
    state_key::StateKey, state_key_prefix::StateKeyPrefix, state_value::StateValue,
};

use schemadb::{
    define_schema,
    schema::{KeyCodec, SeekKeyCodec, ValueCodec},
};
use std::io::Write;

type Key = StateKey;

define_schema!(
    LatestStateValueSchema,
    Key,
    StateValue,
    LATEST_STATE_VALUE_CF_NAME
);

impl KeyCodec<LatestStateValueSchema> for Key {
    fn encode_key(&self) -> Result<Vec<u8>> {
        let mut encoded = vec![];
        encoded.write_all(&self.encode()?)?;
        Ok(encoded)
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        Ok(StateKey::decode(data)?)
    }
}

impl ValueCodec<LatestStateValueSchema> for StateValue {
    fn encode_value(&self) -> Result<Vec<u8>> {
        bcs::to_bytes(self).map_err(Into::into)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        bcs::from_bytes(data).map_err(Into::into)
    }
}

impl SeekKeyCodec<LatestStateValueSchema> for &StateKeyPrefix {
    fn encode_seek_key(&self) -> Result<Vec<u8>> {
        self.encode()
    }
}

#[cfg(test)]
mod test;
