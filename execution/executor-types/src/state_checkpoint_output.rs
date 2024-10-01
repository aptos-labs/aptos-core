// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use anyhow::{ensure, Result};
use aptos_crypto::HashValue;
use aptos_storage_interface::{
    state_delta::StateDelta,
};
use aptos_types::{
    state_store::ShardedStateUpdates,
};
use itertools::zip_eq;

#[derive(Debug, Default)]
pub struct StateCheckpointOutput {
    /// includes state updates between the last checkpoint version and the current version
    pub result_state: StateDelta,
    /// state updates between the base version and the last checkpoint version
    pub state_updates_before_last_checkpoint: Option<ShardedStateUpdates>,
    pub per_version_state_updates: Vec<ShardedStateUpdates>,
    pub state_checkpoint_hashes: Vec<Option<HashValue>>,
}

impl StateCheckpointOutput {
    pub fn new(
        result_state: StateDelta,
        state_updates_before_last_checkpoint: Option<ShardedStateUpdates>,
        state_checkpoint_hashes: Vec<Option<HashValue>>,
        per_version_state_updates: Vec<ShardedStateUpdates>,
    ) -> Self {
        Self {
            result_state,
            per_version_state_updates,
            state_checkpoint_hashes,
            state_updates_before_last_checkpoint,
        }
    }

    pub fn check_and_update_state_checkpoint_hashes(
        &mut self,
        trusted_hashes: Vec<Option<HashValue>>,
    ) -> Result<()> {
        let len = self.state_checkpoint_hashes.len();
        ensure!(
            len == trusted_hashes.len(),
            "Number of txns doesn't match. self: {len}, trusted: {}",
            trusted_hashes.len()
        );

        zip_eq(
            self.state_checkpoint_hashes.iter_mut(),
            trusted_hashes.iter(),
        )
        .try_for_each(|(self_hash, trusted_hash)| {
            if self_hash.is_none() && trusted_hash.is_some() {
                *self_hash = *trusted_hash;
            } else {
                ensure!(self_hash == trusted_hash,
                    "State checkpoint hash doesn't match, self: {self_hash:?}, trusted: {trusted_hash:?}"
                );
            }
            Ok(())
        })
    }

    /// FIXME(aldenhu): move (for executor-benchmark)
    pub fn check_aborts_discards_retries(
        &self,
        allow_aborts: bool,
        allow_discards: bool,
        allow_retries: bool,
    ) {
        let aborts = self
            .txns
            .to_commit
            .iter()
            .flat_map(|(txn, output)| match output.status().status() {
                Ok(execution_status) => {
                    if execution_status.is_success() {
                        None
                    } else {
                        Some(format!("{:?}: {:?}", txn, output.status()))
                    }
                },
                Err(_) => None,
            })
            .collect::<Vec<_>>();

        let discards_3 = self
            .txns
            .to_discard
            .iter()
            .take(3)
            .map(|(txn, output)| format!("{:?}: {:?}", txn, output.status()))
            .collect::<Vec<_>>();
        let retries_3 = self
            .txns
            .to_retry
            .iter()
            .take(3)
            .map(|(txn, output)| format!("{:?}: {:?}", txn, output.status()))
            .collect::<Vec<_>>();

        if !aborts.is_empty() || !discards_3.is_empty() || !retries_3.is_empty() {
            println!(
                "Some transactions were not successful: {} aborts, {} discards and {} retries out of {}, examples: aborts: {:?}, discards: {:?}, retries: {:?}",
                aborts.len(),
                self.txns.to_discard.len(),
                self.txns.to_retry.len(),
                self.input_txns_len(),
                &aborts[..(aborts.len().min(3))],
                discards_3,
                retries_3,
            )
        }

        assert!(
            allow_aborts || aborts.is_empty(),
            "No aborts allowed, {}, examples: {:?}",
            aborts.len(),
            &aborts[..(aborts.len().min(3))]
        );
        assert!(
            allow_discards || discards_3.is_empty(),
            "No discards allowed, {}, examples: {:?}",
            self.txns.to_discard.len(),
            discards_3,
        );
        assert!(
            allow_retries || retries_3.is_empty(),
            "No retries allowed, {}, examples: {:?}",
            self.txns.to_retry.len(),
            retries_3,
        );
    }
}
