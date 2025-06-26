// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
//! This module defines the physical storage schema for db wide miscellaneous metadata entries.
//! For example, the progress of a db pruner.
//!
//! ```text
//! |<------key---->|<---- value --->|
//! | metadata key  | metadata value |
//! ```
//!

use crate::schema::DB_METADATA_CF_NAME;
use anyhow::Result;
use aptos_db_indexer_schemas::metadata::StateSnapshotProgress;
use aptos_schemadb::{
    define_schema,
    schema::{KeyCodec, ValueCodec},
};
use aptos_types::transaction::Version;
use serde::{Deserialize, Serialize};

type ShardId = usize;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub(crate) enum DbMetadataValue {
    Version(Version),
    StateSnapshotProgress(StateSnapshotProgress),
}

impl DbMetadataValue {
    pub fn expect_version(self) -> Version {
        match self {
            Self::Version(version) => version,
            _ => unreachable!("expected Version, got {:?}", self),
        }
    }

    pub fn expect_state_snapshot_progress(self) -> StateSnapshotProgress {
        match self {
            Self::StateSnapshotProgress(progress) => progress,
            _ => unreachable!("expected KeyHashAndUsage, got {:?}", self),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub enum DbMetadataKey {
    LedgerPrunerProgress,
    StateMerklePrunerProgress,
    EpochEndingStateMerklePrunerProgress,
    StateKvPrunerProgress,
    StateSnapshotKvRestoreProgress(Version),
    LedgerCommitProgress,
    StateKvCommitProgress,
    OverallCommitProgress,
    StateKvShardCommitProgress(ShardId),
    StateMerkleCommitProgress,
    StateMerkleShardCommitProgress(ShardId),
    EventPrunerProgress,
    TransactionAccumulatorPrunerProgress,
    TransactionInfoPrunerProgress,
    TransactionPrunerProgress,
    WriteSetPrunerProgress,
    StateMerkleShardPrunerProgress(ShardId),
    EpochEndingStateMerkleShardPrunerProgress(ShardId),
    StateKvShardPrunerProgress(ShardId),
    StateMerkleShardRestoreProgress(ShardId, Version),
    TransactionAuxiliaryDataPrunerProgress,
    PersistedAuxiliaryInfoPrunerProgress,
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
