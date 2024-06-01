// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::DbReader;
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_types::{
    ledger_info::LedgerInfo,
    state_store::{
        errors::StateviewError, state_key::StateKey, state_storage_usage::StateStorageUsage,
        state_value::StateValue, TStateView,
    },
    transaction::Version,
};
use std::sync::Arc;

type Result<T, E = StateviewError> = std::result::Result<T, E>;

pub struct DbStateView {
    db: Arc<dyn DbReader>,
    version: Option<Version>,
    verify_against_state_root_hash: Option<HashValue>,
}

impl DbStateView {
    fn get(&self, key: &StateKey) -> Result<Option<StateValue>> {
        Ok(if let Some(version) = self.version {
            if let Some(root_hash) = self.verify_against_state_root_hash {
                let (value, proof) = self
                    .db
                    .get_state_value_with_proof_by_version(key, version)?;
                proof.verify(root_hash, CryptoHash::hash(key), value.as_ref())?;
                value
            } else {
                self.db.get_state_value_by_version(key, version)?
            }
        } else {
            None
        })
    }
}

impl TStateView for DbStateView {
    type Key = StateKey;

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<StateValue>> {
        self.get(state_key).map_err(Into::into)
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        self.db
            .get_state_storage_usage(self.version)
            .map_err(Into::into)
    }
}

pub trait LatestDbStateCheckpointView {
    fn latest_state_checkpoint_view(&self) -> Result<DbStateView>;
}

impl LatestDbStateCheckpointView for Arc<dyn DbReader> {
    fn latest_state_checkpoint_view(&self) -> Result<DbStateView> {
        Ok(DbStateView {
            db: self.clone(),
            version: self
                .get_latest_state_checkpoint_version()
                .map_err(Into::<StateviewError>::into)?,
            verify_against_state_root_hash: None,
        })
    }
}

pub trait DbStateViewAtVersion {
    fn state_view_at_version(&self, version: Option<Version>) -> Result<DbStateView>;
}

impl DbStateViewAtVersion for Arc<dyn DbReader> {
    fn state_view_at_version(&self, version: Option<Version>) -> Result<DbStateView> {
        Ok(DbStateView {
            db: self.clone(),
            version,
            verify_against_state_root_hash: None,
        })
    }
}

pub trait VerifiedStateViewAtVersion {
    fn verified_state_view_at_version(
        &self,
        version: Option<Version>,
        ledger_info: &LedgerInfo,
    ) -> Result<DbStateView>;
}

impl VerifiedStateViewAtVersion for Arc<dyn DbReader> {
    fn verified_state_view_at_version(
        &self,
        version: Option<Version>,
        ledger_info: &LedgerInfo,
    ) -> Result<DbStateView> {
        let db = self.clone();

        if let Some(version) = version {
            let txn_with_proof =
                db.get_transaction_by_version(version, ledger_info.version(), false)?;
            txn_with_proof.verify(ledger_info)?;

            let state_root_hash = txn_with_proof
                .proof
                .transaction_info
                .state_checkpoint_hash()
                .ok_or_else(|| StateviewError::NotFound("state_checkpoint_hash".to_string()))?;

            Ok(DbStateView {
                db,
                version: Some(version),
                verify_against_state_root_hash: Some(state_root_hash),
            })
        } else {
            Ok(DbStateView {
                db,
                version: None,
                verify_against_state_root_hash: None,
            })
        }
    }
}
