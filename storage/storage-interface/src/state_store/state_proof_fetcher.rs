// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    state_store::state_summary::StateSummary,
    utils::planned::{Plan, Planned},
    DbReader,
};
use aptos_crypto::HashValue;
use aptos_scratchpad::ProofRead;
use aptos_types::{proof::SparseMerkleProofExt, transaction::Version};
use aptos_vm::AptosVM;
use derive_more::Deref;
use once_cell::sync::Lazy;
use once_map::OnceMap;
use std::{fmt::Formatter, sync::Arc};

static IO_POOL: Lazy<Arc<rayon::ThreadPool>> = Lazy::new(|| {
    Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(AptosVM::get_num_proof_reading_threads()) // More than 8 threads doesn't seem to help much
            .thread_name(|index| format!("proof-read-{}", index))
            .build()
            .unwrap(),
    )
});

#[derive(Deref)]
pub struct StateProofFetcher {
    #[deref]
    state_summary: StateSummary,
    db: Arc<dyn DbReader>,
    // with OnceMap one can get a reference to the proof without locking the whole map up and
    // prevent updating.
    memorized_proofs: OnceMap<HashValue, Box<Planned<SparseMerkleProofExt>>>,
}

impl StateProofFetcher {
    pub fn new_persisted(db: Arc<dyn DbReader>) -> anyhow::Result<Self> {
        Ok(Self::new(db.get_persisted_state_summary()?, db))
    }

    pub fn new(state_summary: StateSummary, db: Arc<dyn DbReader>) -> Self {
        Self {
            state_summary,
            db,
            memorized_proofs: OnceMap::new(),
        }
    }

    pub fn new_dummy() -> Self {
        struct Dummy;
        impl DbReader for Dummy {}

        Self::new(StateSummary::new_empty(), Arc::new(Dummy))
    }

    fn root_hash(&self) -> HashValue {
        self.state_summary.root_hash()
    }

    pub fn get_proof_impl(
        db: Arc<dyn DbReader>,
        key_hash: HashValue,
        version: Version,
        root_depth: usize,
        root_hash: HashValue,
    ) -> anyhow::Result<SparseMerkleProofExt> {
        if rand::random::<usize>() % 10000 == 0 {
            // 1 out of 10000 times, verify the proof.
            let (val_opt, proof) = db
                // verify the full proof
                .get_state_value_with_proof_by_version_ext(&key_hash, version, 0)?;
            proof.verify(root_hash, key_hash, val_opt.as_ref())?;
            Ok(proof)
        } else {
            Ok(db.get_state_proof_by_version_ext(&key_hash, version, root_depth)?)
        }
    }

    pub fn schedule_get_proof_once(
        &self,
        key_hash: HashValue,
        root_depth: usize,
    ) -> Option<&Planned<SparseMerkleProofExt>> {
        self.version().map(|ver| {
            self.memorized_proofs.insert(key_hash, |key_hash| {
                let key_hash = *key_hash;
                let db = self.db.clone();
                let root_hash = self.root_hash();

                Box::new(IO_POOL.plan(move || {
                    Self::get_proof_impl(db, key_hash, ver, root_depth, root_hash)
                        .expect("Failed getting state proof.")
                }))
            })
        })
    }
}

impl ProofRead for StateProofFetcher {
    fn get_proof(&self, key: HashValue, root_depth: usize) -> Option<&SparseMerkleProofExt> {
        self.schedule_get_proof_once(key, root_depth)
            .map(|planned| planned.wait(Some("state_proof_wait")))
    }
}

impl std::fmt::Debug for StateProofFetcher {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "StateProofFetcher {{ .. }}")
    }
}
