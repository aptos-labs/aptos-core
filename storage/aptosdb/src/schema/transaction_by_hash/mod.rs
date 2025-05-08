// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This module defines physical storage schema mapping transaction.submitted_txn_hash() to its version.
//! With the version one can resort to `TransactionSchema` for the transaction content.
//!
//! ```text
//! |<--key-->|<-value->|
//! |   hash  | txn_ver |
//! ```

use crate::schema::{ensure_slice_len_eq, TRANSACTION_BY_HASH_CF_NAME};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_schemadb::{
    define_schema,
    schema::{KeyCodec, ValueCodec},
};
use aptos_types::transaction::Version;
use byteorder::{BigEndian, ReadBytesExt};
use std::mem::size_of;

define_schema!(
    TransactionByHashSchema,
    HashValue,
    Version,
    TRANSACTION_BY_HASH_CF_NAME
);

impl KeyCodec<TransactionByHashSchema> for HashValue {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(self.to_vec())
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        ensure_slice_len_eq(data, size_of::<Self>())?;
        Ok(HashValue::from_slice(data)?)
    }
}

impl ValueCodec<TransactionByHashSchema> for Version {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(self.to_be_bytes().to_vec())
    }

    fn decode_value(mut data: &[u8]) -> Result<Self> {
        ensure_slice_len_eq(data, size_of::<Self>())?;

        Ok(data.read_u64::<BigEndian>()?)
    }
}

#[cfg(test)]
mod test;
