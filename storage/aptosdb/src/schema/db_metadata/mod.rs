// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
//! This module defines the physical storage schema for indexes of min_readable_version of pruners.
//! For the state pruner, the metadata represents the key of the StaleNodeIndexSchema and for the
//! ledger pruner, the metadata represents the key of the TransactionSchema.
//!
//! ```text
//! |<------key---->|<------ value ----->|
//! | pruner tag    | pruned until values|
//! ```
//!

use crate::schema::DB_METADATA_CF_NAME;
use anyhow::Result;
use aptos_types::transaction::Version;
use schemadb::{
    define_schema,
    schema::{KeyCodec, ValueCodec},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub(crate) enum DbMetadataValue {
    Version(Version),
}

impl DbMetadataValue {
    pub fn expect_version(self) -> Version {
        match self {
            Self::Version(v) => v,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub enum DbMetadataKey {
    LedgerPrunerProgress,
    StateMerklePrunerProgress,
    EpochEndingStateMerklePrunerProgress,
}

define_schema!(
    DbMetadataSchema,
    DbMetadataKey,
    DbMetadataValue,
    DB_METADATA_CF_NAME
);

impl KeyCodec<DbMetadataSchema> for DbMetadataKey {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl ValueCodec<DbMetadataSchema> for DbMetadataValue {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

#[cfg(test)]
mod test;
