// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::DbReader;
use aptos_crypto::HashValue;
use aptos_types::{
    ledger_info::LedgerInfo,
    state_store::{
        errors::StateViewError, state_key::StateKey, state_slot::StateSlot,
        state_storage_usage::StateStorageUsage, state_value::StateValue, StateViewId,
        StateViewResult, TStateView,
    },
    transaction::Version,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct DbStateView {
    db: Arc<dyn DbReader>,
    version: Option<Version>,
    /// Whether `version` is known to be a block end, which allows a cheaper read.
    at_checkpoint: bool,
    /// DB doesn't support returning proofs for buffered state, so only optionally verify proof.
    /// TODO: support returning state proof for buffered state.
    maybe_verify_against_state_root_hash: Option<HashValue>,
}

impl DbStateView {
    fn get(&self, key: &StateKey) -> StateViewResult<Option<(Version, StateValue)>> {
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
                }
            }
            Ok(if self.at_checkpoint {
                self.db
                    .get_state_value_with_version_at_checkpoint(key, version)?
            } else {
                self.db
                    .get_state_value_with_version_by_version(key, version)?
            })
        } else {
            Ok(None)
        }
    }
}

impl TStateView for DbStateView {
    type Key = StateKey;

    fn id(&self) -> StateViewId {
        if let Some(version) = self.version {
            StateViewId::TransactionValidation {
                base_version: version,
            }
        } else {
            StateViewId::Miscellaneous
        }
    }

    fn get_state_slot(&self, state_key: &StateKey) -> StateViewResult<StateSlot> {
        Ok(StateSlot::from_db_get(
            state_key.clone(),
            self.get(state_key)?,
        ))
    }

    fn get_usage(&self) -> StateViewResult<StateStorageUsage> {
        self.db
            .get_state_storage_usage(self.version)
            .map_err(Into::into)
    }

    fn next_version(&self) -> Version {
        self.version.map_or(0, |v| v + 1)
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
            at_checkpoint: true,
            maybe_verify_against_state_root_hash: None,
        })
    }
}

pub trait DbStateViewAtVersion {
    fn state_view_at_version(&self, version: Option<Version>) -> StateViewResult<DbStateView>;

    /// Like `state_view_at_version`, but `version` must be a block end (state checkpoint), which
    /// allows cheaper reads.
    fn state_view_at_checkpoint(&self, version: Option<Version>) -> StateViewResult<DbStateView>;
}

impl DbStateViewAtVersion for Arc<dyn DbReader> {
    fn state_view_at_version(&self, version: Option<Version>) -> StateViewResult<DbStateView> {
        Ok(DbStateView {
            db: self.clone(),
            version,
            at_checkpoint: false,
            maybe_verify_against_state_root_hash: None,
        })
    }

    fn state_view_at_checkpoint(&self, version: Option<Version>) -> StateViewResult<DbStateView> {
        Ok(DbStateView {
            db: self.clone(),
            version,
            at_checkpoint: true,
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
                at_checkpoint: true,
                maybe_verify_against_state_root_hash: Some(state_root_hash),
            })
        } else {
            Ok(DbStateView {
                db,
                version: None,
                at_checkpoint: true,
                maybe_verify_against_state_root_hash: None,
            })
        }
    }
}
