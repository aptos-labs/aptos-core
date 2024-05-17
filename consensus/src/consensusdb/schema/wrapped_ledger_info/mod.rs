// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This module defines physical storage schema for consensus wrapped ledger info (of a block).
//! Wrapped Ledger Info is similar to a quorum certificate but VoteData is dummy. When round manager
//! collects enough order votes on a block, it forms an order certificate (WrappedLedgerInfo) and store in DB.
//! As the commit certificate is derived from order certificate, the commit certificate is also a WrappedLedgerInfo.
//!
//! Serialized wrapped ledger info bytes identified by block_hash of the ledger info.
//! ```text
//! |<---key---->|<--------value------>|
//! | block_hash |  WrappedLedgerInfo  |
//! ```

use crate::define_schema;
use anyhow::Result;
use aptos_consensus_types::wrapped_ledger_info::WrappedLedgerInfo;
use aptos_crypto::HashValue;
use aptos_schemadb::{
    schema::{KeyCodec, ValueCodec},
    ColumnFamilyName,
};

pub const WLI_CF_NAME: ColumnFamilyName = "wli";

define_schema!(WLISchema, HashValue, WrappedLedgerInfo, WLI_CF_NAME);

impl KeyCodec<WLISchema> for HashValue {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(self.to_vec())
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        Ok(HashValue::from_slice(data)?)
    }
}

impl ValueCodec<WLISchema> for WrappedLedgerInfo {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

#[cfg(test)]
mod test;
