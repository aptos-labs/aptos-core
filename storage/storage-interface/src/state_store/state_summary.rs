// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    metrics::TIMER,
    state_store::{
        state::LedgerState,
        state_update_refs::{BatchedStateUpdateRefs, StateUpdateRefs},
    },
    DbReader,
};
use anyhow::Result;
use aptos_crypto::{
    hash::{CryptoHash, CORRUPTION_SENTINEL},
    HashValue,
};
use aptos_logger::info;
use aptos_metrics_core::TimerHelper;
use aptos_scratchpad::{ProofRead, SparseMerkleTree};
use aptos_types::{
    proof::SparseMerkleProofExt,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use derive_more::Deref;
use itertools::Itertools;
use rayon::prelude::*;
use std::collections::HashMap;

/// The data structure through which the entire state at a given
/// version can be summarized to a concise digest (the root hash).
#[derive(Clone, Debug)]
pub struct StateSummary {
    /// The next version. If this is 0, the state is the "pre-genesis" empty state.
    next_version: Version,
    pub hot_state_summary: SparseMerkleTree,
    pub global_state_summary: SparseMerkleTree,
}

impl StateSummary {
    pub fn new_at_version(
        version: Option<Version>,
        hot_state_summary: SparseMerkleTree,
        global_state_summary: SparseMerkleTree,
    ) -> Self {
        Self {
            next_version: version.map_or(0, |v| v + 1),
            hot_state_summary,
            global_state_summary,
        }
    }

    pub fn new_empty() -> Self {
        Self {
            next_version: 0,
            hot_state_summary: SparseMerkleTree::new_empty(),
            global_state_summary: SparseMerkleTree::new_empty(),
        }
    }

    pub fn root_hash(&self) -> HashValue {
        self.global_state_summary.root_hash()
    }

    pub fn hot_root_hash(&self) -> HashValue {
        self.hot_state_summary.root_hash()
    }

    pub fn next_version(&self) -> Version {
        self.next_version
    }

    pub fn version(&self) -> Option<Version> {
        self.next_version.checked_sub(1)
    }

    pub fn is_descendant_of(&self, other: &Self) -> bool {
        self.hot_state_summary
            .is_descendant_of(&other.hot_state_summary)
            && self
                .global_state_summary
                .is_descendant_of(&other.global_state_summary)
    }

    pub fn update(
        &self,
        hot_persisted: &ProvableStateSummary,
        cold_persisted: &ProvableStateSummary,
        hot_inserted: &[HashMap<StateKey, Option<StateValue>>; 16],
        hot_evicted: &[HashMap<StateKey, Option<StateValue>>; 16],
        new_next_version: Version,
    ) -> Result<Self> {
        let _timer = TIMER.timer_with(&["state_summary__update"]);

        assert_ne!(self.hot_state_summary.root_hash(), *CORRUPTION_SENTINEL);
        assert_ne!(self.global_state_summary.root_hash(), *CORRUPTION_SENTINEL);

        // Persisted must be before or at my version.
        assert!(hot_persisted.next_version() <= self.next_version());
        assert!(cold_persisted.next_version() <= self.next_version());
        // Updates must start at exactly my version.
        // assert_eq!(updates.first_version(), self.next_version());

        info!("start compute hot smt updates");
        let hot_smt_updates = hot_inserted
            .par_iter() // clone hashes and sort items in parallel
            // TODO(aldenhu): smt per shard?
            .flat_map(|shard| {
                shard
                    .iter()
                    .map(|(k, value_opt)| (k, value_opt.as_ref().map(|v| v.hash())))
                    // The keys in the shard are already unique, and shards are ordered by the
                    // first nibble of the key hash. `batch_update_sorted_uniq` can be
                    // called if within each shard items are sorted by key hash.
                    .sorted_by_key(|(k, _v)| k.crypto_hash_ref())
                    .collect_vec()
            })
            .collect::<Vec<_>>();
        info!("done compute hot smt updates");

        let hot_smt = self
            .hot_state_summary
            .freeze(&hot_persisted.hot_state_summary)
            .batch_update_sorted_uniq(&hot_smt_updates, hot_persisted)?
            .unfreeze();
        info!("done computing hot smt");

        info!("start compute cold smt updates");
        let cold_smt_updates = hot_evicted
            .par_iter() // clone hashes and sort items in parallel
            // TODO(aldenhu): smt per shard?
            .flat_map(|shard| {
                shard
                    .iter()
                    .map(|(k, value_opt)| (k, value_opt.as_ref().map(|v| v.hash())))
                    // The keys in the shard are already unique, and shards are ordered by the
                    // first nibble of the key hash. `batch_update_sorted_uniq` can be
                    // called if within each shard items are sorted by key hash.
                    .sorted_by_key(|(k, _v)| k.crypto_hash_ref())
                    .collect_vec()
            })
            .collect::<Vec<_>>();
        info!("done compute cold smt updates");

        let cold_smt = self
            .global_state_summary
            .freeze(&cold_persisted.global_state_summary)
            .batch_update_sorted_uniq(&cold_smt_updates, cold_persisted)?
            .unfreeze();
        info!("done computing cold smt");

        // TODO(HotState): compute new hot state from the `self.hot_state_summary` and
        // `updates`.
        Ok(Self {
            next_version: new_next_version,
            hot_state_summary: hot_smt,
            global_state_summary: cold_smt,
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

    pub fn assert_versions_match(&self, state: &LedgerState) {
        assert_eq!(self.next_version(), state.next_version());
        assert_eq!(
            self.last_checkpoint.next_version(),
            state.last_checkpoint().next_version()
        );
    }

    pub fn latest(&self) -> &StateSummary {
        &self.latest
    }

    pub fn last_checkpoint(&self) -> &StateSummary {
        &self.last_checkpoint
    }

    pub fn update(
        &self,
        hot_persisted: &ProvableStateSummary,
        cold_persisted: &ProvableStateSummary,
        hot_inserted: &[HashMap<StateKey, Option<StateValue>>; 16],
        hot_evicted: &[HashMap<StateKey, Option<StateValue>>; 16],
        new_next_version: Version,
    ) -> Result<Self> {
        let _timer = TIMER.timer_with(&["ledger_state_summary__update"]);
        assert_eq!(
            self.latest.next_version(),
            self.last_checkpoint.next_version()
        );

        let new_ckpt = self.latest.update(
            hot_persisted,
            cold_persisted,
            hot_inserted,
            hot_evicted,
            new_next_version,
        )?;
        Ok(Self::new(new_ckpt.clone(), new_ckpt))
    }
}

#[derive(Deref)]
pub struct ProvableStateSummary<'db> {
    #[deref]
    state_summary: StateSummary,
    db: &'db (dyn DbReader + Sync),
    is_hot: bool,
}

impl<'db> ProvableStateSummary<'db> {
    pub fn new_persisted(db: &'db (dyn DbReader + Sync), is_hot: bool) -> Result<Self> {
        Ok(Self::new(db.get_persisted_state_summary()?, db, is_hot))
    }

    pub fn new(state_summary: StateSummary, db: &'db (dyn DbReader + Sync), is_hot: bool) -> Self {
        Self {
            state_summary,
            db,
            is_hot,
        }
    }

    fn get_proof(
        &self,
        key: &HashValue,
        version: Version,
        root_depth: usize,
    ) -> Result<SparseMerkleProofExt> {
        if rand::random::<usize>() % 10000 == 0 {
            // 1 out of 10000 times, verify the proof.
            let (val_opt, proof) = self
                .db
                // check the full proof
                .get_state_value_with_proof_by_version_ext(key, version, 0, self.is_hot)?;
            proof.verify(
                if self.is_hot {
                    self.state_summary.hot_state_summary.root_hash()
                } else {
                    self.state_summary.global_state_summary.root_hash()
                },
                *key,
                val_opt.as_ref(),
            )?;
            Ok(proof)
        } else {
            Ok(self
                .db
                .get_state_proof_by_version_ext(key, version, root_depth, self.is_hot)?)
        }
    }
}

impl ProofRead for ProvableStateSummary<'_> {
    // TODO(aldenhu): return error
    fn get_proof(&self, key: &HashValue, root_depth: usize) -> Option<SparseMerkleProofExt> {
        self.version().map(|ver| {
            let _timer = TIMER.timer_with(&["provable_state_summary__get_proof"]);

            self.get_proof(key, ver, root_depth)
                .expect("Failed to get account state with proof by version.")
        })
    }
}
