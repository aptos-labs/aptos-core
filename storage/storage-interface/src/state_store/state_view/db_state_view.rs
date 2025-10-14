// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{state_store::state_view::cached_state_view::ShardedStateCache, DbReader};
use aptos_crypto::HashValue;
use aptos_logger::info;
use aptos_types::{
    ledger_info::LedgerInfo,
    state_store::{
        errors::StateViewError, state_key::StateKey, state_slot::StateSlot,
        state_storage_usage::StateStorageUsage, state_value::StateValue, StateViewId,
        StateViewResult, TStateView,
    },
    transaction::Version,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

pub struct DbStateView {
    db: Arc<dyn DbReader>,
    version: Option<Version>,
    /// DB doesn't support returning proofs for buffered state, so only optionally verify proof.
    /// dummy change
    /// TODO: support returning state proof for buffered state.
    maybe_verify_against_state_root_hash: Option<HashValue>,
    memorized: ShardedStateCache,
    n_total_get: AtomicUsize,
    n_memorized: AtomicUsize,
}

impl Drop for DbStateView {
    fn drop(&mut self) {
        let n_total = self.n_total_get.load(Ordering::Relaxed);
        let n_cached = self.n_memorized.load(Ordering::Relaxed);
        info!(
            "total get: {}, cached: {}. Hit rate: {}%",
            n_total,
            n_cached,
            n_cached as f64 * 100.0 / n_total as f64,
        )
    }
}

impl DbStateView {
    fn get_unmemorized(&self, key: &StateKey) -> StateViewResult<Option<(Version, StateValue)>> {
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
            Ok(self
                .db
                .get_state_value_with_version_by_version(key, version)?)
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
        self.n_total_get.fetch_add(1, Ordering::Relaxed);
        if let Some(slot) = self.memorized.get_cloned(state_key) {
            self.n_memorized.fetch_add(1, Ordering::Relaxed);
            return Ok(slot);
        }

        let slot = StateSlot::from_db_get(self.get_unmemorized(state_key)?);
        self.memorized.try_insert(state_key, &slot);
        Ok(slot)
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
        let version = self
            .get_latest_state_checkpoint_version()
            .map_err(Into::<StateViewError>::into)?;
        Ok(DbStateView {
            db: self.clone(),
            version,
            maybe_verify_against_state_root_hash: None,
            memorized: ShardedStateCache::new_empty(version),
            n_total_get: AtomicUsize::new(0),
            n_memorized: AtomicUsize::new(0),
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
            memorized: ShardedStateCache::new_empty(version),
            n_total_get: AtomicUsize::new(0),
            n_memorized: AtomicUsize::new(0),
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
                memorized: ShardedStateCache::new_empty(Some(version)),
                n_total_get: AtomicUsize::new(0),
                n_memorized: AtomicUsize::new(0),
            })
        } else {
            Ok(DbStateView {
                db,
                version: None,
                maybe_verify_against_state_root_hash: None,
                memorized: ShardedStateCache::new_empty(None),
                n_total_get: AtomicUsize::new(0),
                n_memorized: AtomicUsize::new(0),
            })
        }
    }
}
