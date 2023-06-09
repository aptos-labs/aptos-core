// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module provides common utilities for the DB pruner.

use crate::{
    ledger_db::LedgerDb,
    pruner::{
        ledger_store::ledger_store_pruner::LedgerPruner,
        state_kv_pruner::StateKvPruner,
        state_store::{generics::StaleNodeIndexSchemaTrait, StateMerklePruner},
    },
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        version_data::VersionDataSchema,
    },
    state_kv_db::StateKvDb,
    state_merkle_db::StateMerkleDb,
    utils::get_progress,
};
use anyhow::Result;
use aptos_jellyfish_merkle::StaleNodeIndex;
use aptos_schemadb::{schema::KeyCodec, ReadOptions, DB};
use aptos_types::transaction::Version;
use std::sync::Arc;

/// A utility function to instantiate the state pruner
pub fn create_state_merkle_pruner<S: StaleNodeIndexSchemaTrait>(
    state_merkle_db: Arc<StateMerkleDb>,
) -> Arc<StateMerklePruner<S>>
where
    StaleNodeIndex: KeyCodec<S>,
{
    Arc::new(StateMerklePruner::<S>::new(Arc::clone(&state_merkle_db)))
}

/// A utility function to instantiate the ledger pruner
pub(crate) fn create_ledger_pruner(ledger_db: Arc<LedgerDb>) -> Arc<LedgerPruner> {
    Arc::new(LedgerPruner::new(ledger_db).expect("Failed to create ledger pruner."))
}

/// A utility function to instantiate the state kv pruner.
pub(crate) fn create_state_kv_pruner(state_kv_db: Arc<StateKvDb>) -> Arc<StateKvPruner> {
    Arc::new(StateKvPruner::new(state_kv_db))
}

pub(crate) fn get_ledger_pruner_progress(ledger_db: &LedgerDb) -> Result<Version> {
    Ok(
        if let Some(version) = get_progress(
            ledger_db.metadata_db(),
            &DbMetadataKey::LedgerPrunerProgress,
        )? {
            version
        } else {
            let mut iter = ledger_db
                .metadata_db()
                .iter::<VersionDataSchema>(ReadOptions::default())?;
            iter.seek_to_first();
            match iter.next().transpose()? {
                Some((version, _)) => version,
                None => 0,
            }
        },
    )
}

pub(crate) fn get_state_kv_pruner_progress(state_kv_db: &StateKvDb) -> Result<Version> {
    Ok(get_progress(
        state_kv_db.metadata_db(),
        &DbMetadataKey::StateKvPrunerProgress,
    )?
    .unwrap_or(0))
}

pub(crate) fn get_state_merkle_pruner_progress<S: StaleNodeIndexSchemaTrait>(
    state_merkle_db: &StateMerkleDb,
) -> Result<Version>
where
    StaleNodeIndex: KeyCodec<S>,
{
    Ok(get_progress(state_merkle_db.metadata_db(), &S::tag())?.unwrap_or(0))
}

pub(crate) fn get_or_initialize_ledger_subpruner_progress(
    sub_db: &DB,
    progress_key: &DbMetadataKey,
    metadata_progress: Version,
) -> Result<Version> {
    Ok(
        if let Some(v) = sub_db.get::<DbMetadataSchema>(progress_key)? {
            v.expect_version()
        } else {
            sub_db.put::<DbMetadataSchema>(
                progress_key,
                &DbMetadataValue::Version(metadata_progress),
            )?;
            metadata_progress
        },
    )
}
