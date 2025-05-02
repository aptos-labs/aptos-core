// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::DbReader;
use aptos_crypto::HashValue;
use aptos_types::{
    ledger_info::LedgerInfo,
    state_store::{
        errors::StateViewError, state_key::StateKey, state_storage_usage::StateStorageUsage,
        state_value::StateValue, StateViewResult, TStateView,
    },
    transaction::Version,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct DbStateView {
    db: Arc<dyn DbReader>,
    version: Option<Version>,
    /// DB doesn't support returning proofs for buffered state, so only optionally verify proof.
    /// TODO: support returning state proof for buffered state.
    maybe_verify_against_state_root_hash: Option<HashValue>,
}

impl DbStateView {
    fn get(&self, key: &StateKey) -> StateViewResult<StateValue> {
        if let Some(version) = self.version {
            if let Some(root_hash) = self.maybe_verify_against_state_root_hash {
                // TODO(aldenhu): sample-verify proof inside DB
                // DB doesn't support returning proofs for buffered state, so only optionally
                // verify proof.
                // TODO: support returning state proof for buffered state.
                if let Ok((value, proof)) =
                    self.db.get_state_value_with_proof_by_version(key, version)
                {
                    proof.verify(root_hash, *key.crypto_hash_ref(), value.as_ref())?;
                    return Ok(value);
                }
            }
            Ok(self.db.get_state_value_by_version(key, version)?)
        } else {
            Ok(None)
        }
    }
}

impl TStateView for DbStateView {
    type Key = StateKey;

    fn get_state_value(&self, state_key: &StateKey) -> StateViewResult<StateValue> {
        self.get(state_key).map_err(Into::into)
    }

    fn get_usage(&self) -> StateViewResult<StateStorageUsage> {
        self.db
            .get_state_storage_usage(self.version)
            .map_err(Into::into)
    }
}

pub trait LatestDbStateCheckpointView {
    fn latest_state_checkpoint_view(&self) -> StateViewResult<DbStateView>;
}

impl LatestDbStateCheckpointView for Arc<dyn DbReader> {
    fn latest_state_checkpoint_view(&self) -> StateViewResult<DbStateView> {
        Ok(DbStateView {
            db: self.clone(),
            version: self
                .get_latest_state_checkpoint_version()
                .map_err(Into::<StateViewError>::into)?,
            maybe_verify_against_state_root_hash: None,
        })
    }
}

pub trait DbStateViewAtVersion {
    fn state_view_at_version(&self, version: Option<Version>) -> StateViewResult<DbStateView>;
}

impl DbStateViewAtVersion for Arc<dyn DbReader> {
    fn state_view_at_version(&self, version: Option<Version>) -> StateViewResult<DbStateView> {
        Ok(DbStateView {
            db: self.clone(),
            version,
            maybe_verify_against_state_root_hash: None,
        })
    }
}

pub trait VerifiedStateViewAtVersion {
    fn verified_state_view_at_version(
        &self,
        version: Option<Version>,
        ledger_info: &LedgerInfo,
    ) -> StateViewResult<DbStateView>;
}

impl VerifiedStateViewAtVersion for Arc<dyn DbReader> {
    fn verified_state_view_at_version(
        &self,
        version: Option<Version>,
        ledger_info: &LedgerInfo,
    ) -> StateViewResult<DbStateView> {
        let db = self.clone();

        if let Some(version) = version {
            let txn_with_proof =
                db.get_transaction_by_version(version, ledger_info.version(), false)?;
            txn_with_proof.verify(ledger_info)?;

            let state_root_hash = txn_with_proof
                .proof
                .transaction_info
                .state_checkpoint_hash()
                .ok_or_else(|| StateViewError::NotFound("state_checkpoint_hash".to_string()))?;

            Ok(DbStateView {
                db,
                version: Some(version),
                maybe_verify_against_state_root_hash: Some(state_root_hash),
            })
        } else {
            Ok(DbStateView {
                db,
                version: None,
                maybe_verify_against_state_root_hash: None,
            })
        }
    }
}
