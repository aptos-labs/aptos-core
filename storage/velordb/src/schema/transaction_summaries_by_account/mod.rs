// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This module defines physical storage schema for a transaction index via which the version of a
//! transaction sent by `account_address` with `sequence_number` can be found. With the version one
//! can resort to `TransactionSchema` for the transaction content.
//!
//! ```text
//! |<-------key------->|<---value--->|
//! | address | version | txn_summary |
//! ```

use crate::schema::{ensure_slice_len_eq, TRANSACTION_SUMMARIES_BY_ACCOUNT_CF_NAME};
use anyhow::Result;
use velor_schemadb::{
    define_pub_schema,
    schema::{KeyCodec, ValueCodec},
};
use velor_types::{
    account_address::AccountAddress,
    transaction::{IndexedTransactionSummary, Version},
};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::{convert::TryFrom, mem::size_of};

define_pub_schema!(
    TransactionSummariesByAccountSchema,
    Key,
    IndexedTransactionSummary,
    TRANSACTION_SUMMARIES_BY_ACCOUNT_CF_NAME
);

type Key = (AccountAddress, Version);

impl KeyCodec<TransactionSummariesByAccountSchema> for Key {
    fn encode_key(&self) -> Result<Vec<u8>> {
        let (ref account_address, version) = *self;

        let mut encoded = account_address.to_vec();
        encoded.write_u64::<BigEndian>(version)?;

        Ok(encoded)
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        ensure_slice_len_eq(data, size_of::<Self>())?;

        let address = AccountAddress::try_from(&data[..AccountAddress::LENGTH])?;
        let version = (&data[AccountAddress::LENGTH..]).read_u64::<BigEndian>()?;

        Ok((address, version))
    }
}

impl ValueCodec<TransactionSummariesByAccountSchema> for IndexedTransactionSummary {
    fn encode_value(&self) -> Result<Vec<u8>> {
        bcs::to_bytes(self).map_err(Into::into)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        bcs::from_bytes(data).map_err(Into::into)
    }
}

#[cfg(test)]
mod test;
