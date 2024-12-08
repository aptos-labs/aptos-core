// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics::TIMER,
    state_store::{
        state::{LedgerState, State},
        state_update_ref_map::BatchedStateUpdateRefs,
    },
};
use anyhow::Result;
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_metrics_core::TimerHelper;
use aptos_scratchpad::{ProofRead, SparseMerkleTree};
use aptos_types::{
    state_store::{state_storage_usage::StateStorageUsage, state_value::StateValue},
    transaction::Version,
};
use derive_more::Deref;
use itertools::Itertools;
use rayon::prelude::*;

/// The data structure through which the entire state at a given
/// version can be summarized to a concise digest (the root hash).
#[derive(Clone, Debug)]
pub struct StateSummary {
    /// The next version. If this is 0, the state is the "pre-genesis" empty state.
    next_version: Version,
    pub global_state_summary: SparseMerkleTree<StateValue>,
}

impl StateSummary {
    pub fn new_at_version(
        version: Option<Version>,
        global_state_summary: SparseMerkleTree<StateValue>,
    ) -> Self {
        Self {
            next_version: version.map_or(0, |v| v + 1),
            global_state_summary,
        }
    }

    pub fn new_empty() -> Self {
        Self {
            next_version: 0,
            global_state_summary: SparseMerkleTree::new_empty(),
        }
    }

    pub fn root_hash(&self) -> HashValue {
        self.global_state_summary.root_hash()
    }

    pub fn next_version(&self) -> Version {
        self.next_version
    }

    pub fn is_the_same(&self, _rhs: &Self) -> bool {
        // FIXME(aldenhu)
        todo!()
    }

    pub fn is_family(&self, _rhs: &Self) -> bool {
        // FIXME(aldenhu)
        todo!()
    }

    pub fn update(
        &self,
        persisted: &StateSummary,
        // Must read proof at the `persisted` version. TODO(aldenhu): refactor to enforce that.
        proof_reader: &impl ProofRead,
        updates: &BatchedStateUpdateRefs,
    ) -> Result<Self> {
        let _timer = TIMER.timer_with(&["state_summary__update"]);

        // Persisted must be before or at my version.
        assert!(persisted.next_version() <= self.next_version());
        // Updates must start at exactly my version.
        assert_eq!(updates.first_version(), self.next_version());

        let smt_updates = updates
            .shards
            .par_iter() // clone hashes and sort items in parallel
            // TODO(aldenhu): smt per shard?
            .flat_map(|shard| {
                shard
                    .iter()
                    .sorted_by(|(k1, _u1), (k2, _u2)| {
                        k1.crypto_hash_ref().cmp(k2.crypto_hash_ref())
                    })
                    .map(|(k, u)| (CryptoHash::hash(*k), u.value))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        // TODO(aldenhu): smt leaf not carry StateValue
        let smt = self
            .global_state_summary
            .freeze(&persisted.global_state_summary)
            .batch_update(
                smt_updates,
                // TODO(aldenhu): smt not carry usage
                StateStorageUsage::Untracked,
                proof_reader,
            )?
            .unfreeze();

        Ok(Self {
            next_version: updates.next_version(),
            global_state_summary: smt,
        })
    }
}

/// At a given version, the summaries of the state and the last checkpoint state at or before the version.
#[derive(Clone, Debug, Deref)]
pub struct LedgerStateSummary {
    #[deref]
    latest: StateSummary,
    last_checkpoint: StateSummary,
}

impl LedgerStateSummary {
    pub fn new(last_checkpoint: StateSummary, latest: StateSummary) -> Self {
        assert!(last_checkpoint.next_version() <= latest.next_version());

        Self {
            last_checkpoint,
            latest,
        }
    }

    pub fn new_empty() -> Self {
        let state_summary = StateSummary::new_empty();
        Self::new(state_summary.clone(), state_summary)
    }

    pub fn next_version(&self) -> Version {
        self.latest.next_version()
    }

    pub fn assert_versions_match(&self, latest_state: &LedgerState) {
        assert_eq!(self.next_version(), latest_state.next_version());
        assert_eq!(
            self.last_checkpoint.next_version(),
            latest_state.last_checkpoint().next_version()
        );
    }

    pub fn latest(&self) -> &StateSummary {
        &self.latest
    }

    pub fn last_checkpoint(&self) -> &StateSummary {
        &self.last_checkpoint
    }

    pub fn update<'kv>(
        &self,
        persisted: &StateSummary,
        // Must read proof at the `persisted` version. TODO(aldenhu): refactor to enforce that.
        proof_reader: &impl ProofRead,
        updates_for_last_checkpoint: Option<&BatchedStateUpdateRefs<'kv>>,
        updates_for_latest: &BatchedStateUpdateRefs<'kv>,
    ) -> Result<Self> {
        let _timer = TIMER.timer_with(&["ledger_state_summary__update"]);

        let last_checkpoint = if let Some(updates) = updates_for_last_checkpoint {
            self.latest.update(persisted, proof_reader, updates)?
        } else {
            self.last_checkpoint.clone()
        };

        let base_of_latest = if updates_for_last_checkpoint.is_none() {
            self.latest()
        } else {
            &last_checkpoint
        };
        let latest = base_of_latest.update(persisted, proof_reader, updates_for_latest)?;

        Ok(Self::new(last_checkpoint, latest))
    }
}

#[derive(Clone, Debug, Deref)]
pub struct StateWithSummary {
    #[deref]
    state: State,
    summary: StateSummary,
}

impl StateWithSummary {
    pub fn new_empty() -> Self {
        Self {
            state: State::new_empty(),
            summary: StateSummary::new_empty(),
        }
    }

    // FIXME(aldenhu): rename
    pub fn new_at_version(
        version: Option<Version>,
        global_state_root_hash: HashValue,
        state_usage: StateStorageUsage,
    ) -> Self {
        Self {
            state: State::new_empty_at_version(version, state_usage),
            summary: StateSummary::new_at_version(
                version,
                SparseMerkleTree::new(global_state_root_hash, state_usage),
            ),
        }
    }

    pub fn new(state: State, summary: StateSummary) -> Self {
        assert_eq!(state.next_version(), summary.next_version());
        Self { state, summary }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn summary(&self) -> &StateSummary {
        &self.summary
    }
}
