// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module provides common utilities for the DB pruner.

use crate::{
    ledger_db::LedgerDb,
    pruner::state_merkle_pruner::generics::StaleNodeIndexSchemaTrait,
    schema::db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
    state_kv_db::StateKvDb,
    state_merkle_db::StateMerkleDb,
    utils::get_progress,
};
use anyhow::Result;
use velor_jellyfish_merkle::StaleNodeIndex;
use velor_schemadb::{schema::KeyCodec, DB};
use velor_types::transaction::Version;

pub(crate) fn get_ledger_pruner_progress(ledger_db: &LedgerDb) -> Result<Version> {
    Ok(ledger_db.metadata_db().get_pruner_progress().unwrap_or(0))
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
    Ok(get_progress(
        state_merkle_db.metadata_db(),
        &S::progress_metadata_key(None),
    )?
    .unwrap_or(0))
}

pub(crate) fn get_or_initialize_subpruner_progress(
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
