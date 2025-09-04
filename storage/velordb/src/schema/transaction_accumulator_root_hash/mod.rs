// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines physical storage schema for the transaction accumulator hash.
//!
//! The transaction accumulator root hash value is stored at each version.
//! ```text
//! |<--key-->|<-value->|
//! | version |   hash  |
//! ```

use crate::schema::{ensure_slice_len_eq, TRANSACTION_ACCUMULATOR_HASH_CF_NAME};
use anyhow::Result;
use velor_crypto::HashValue;
use velor_schemadb::{
    define_schema,
    schema::{KeyCodec, ValueCodec},
};
use velor_types::transaction::Version;
use byteorder::{BigEndian, ReadBytesExt};
use std::mem::size_of;

type Key = Version;
type Value = HashValue;

define_schema!(
    TransactionAccumulatorRootHashSchema,
    Key,
    Value,
    TRANSACTION_ACCUMULATOR_HASH_CF_NAME
);

impl KeyCodec<TransactionAccumulatorRootHashSchema> for Key {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(self.to_be_bytes().to_vec())
    }

    fn decode_key(mut data: &[u8]) -> Result<Self> {
        ensure_slice_len_eq(data, size_of::<Self>())?;
        Ok(data.read_u64::<BigEndian>()?)
    }
}

impl ValueCodec<TransactionAccumulatorRootHashSchema> for HashValue {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(self.to_vec())
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Self::from_slice(data).map_err(Into::into)
    }
}

#[cfg(test)]
mod test;
