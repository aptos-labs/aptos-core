// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{proof_fetcher::ProofFetcher, DbReader};

use anyhow::Result;
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_types::{
    proof::SparseMerkleProof,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use crossbeam_channel::{unbounded, Receiver, Sender};
use num_cpus;
use once_cell::sync::Lazy;
use std::{
    cmp::min,
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

static IO_POOL: Lazy<rayon::ThreadPool> = Lazy::new(|| {
    rayon::ThreadPoolBuilder::new()
        // TODO(grao): Do more analysis to tune this magic number.
        .num_threads(min(256, 4 * num_cpus::get()))
        .thread_name(|index| format!("proof_reader_{}", index))
        .build()
        .unwrap()
});

struct Proof {
    state_key_hash: HashValue,
    proof: SparseMerkleProof,
}

pub struct AsyncProofFetcher {
    reader: Arc<dyn DbReader>,
    data_sender: Sender<Proof>,
    data_receiver: Receiver<Proof>,
    num_proofs_to_read: AtomicUsize,
}

impl AsyncProofFetcher {
    pub fn new(reader: Arc<dyn DbReader>) -> Self {
        let (data_sender, data_receiver) = unbounded();

        Self {
            reader,
            data_sender,
            data_receiver,
            num_proofs_to_read: AtomicUsize::new(0),
        }
    }

    // Returns all read proofs. This is only expected to be called after all reads being scheduled,
    // in a single thread.
    fn finish_and_read_proofs(&self) -> HashMap<HashValue, SparseMerkleProof> {
        let mut proofs = HashMap::new();
        for _ in 0..self.num_proofs_to_read.load(Ordering::Relaxed) {
            let data = self
                .data_receiver
                .recv()
                .expect("Failed to receive proof on the channel.");
            match data {
                Proof {
                    state_key_hash,
                    proof,
                } => {
                    proofs.insert(state_key_hash, proof);
                }
            }
        }
        self.num_proofs_to_read.store(0, Ordering::Relaxed);
        proofs
    }

    // Schedules proof reading work in a background running thread pool.
    fn read_proof_async(&self, state_key: StateKey, version: Version) {
        self.num_proofs_to_read.fetch_add(1, Ordering::Relaxed);
        let reader = self.reader.clone();
        let data_sender = self.data_sender.clone();
        IO_POOL.spawn(move || {
            let proof = reader
                .get_state_proof_by_version(&state_key, version)
                .expect("Proof reading should succeed.");
            data_sender
                .send(Proof {
                    state_key_hash: state_key.hash(),
                    proof,
                })
                .expect("Sending proof should succeed.");
        });
    }
}

impl ProofFetcher for AsyncProofFetcher {
    fn fetch_state_value_and_proof(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<(Option<StateValue>, Option<SparseMerkleProof>)> {
        self.read_proof_async(state_key.clone(), version);
        let value = self.reader.get_state_value_by_version(state_key, version)?;
        Ok((value, None))
    }

    fn get_proof_cache(&self) -> HashMap<HashValue, SparseMerkleProof> {
        self.finish_and_read_proofs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockDbReaderWriter;
    use assert_unordered::assert_eq_unordered;

    #[test]
    fn test_fetch() {
        let fetcher = AsyncProofFetcher::new(Arc::new(MockDbReaderWriter));
        let mut expected_key_hashes = vec![];
        for i in 0..10 {
            let state_key = StateKey::Raw(format!("test_key_{}", i).into_bytes());
            expected_key_hashes.push(state_key.hash());
            let result = fetcher
                .fetch_state_value_and_proof(&state_key, 0)
                .expect("Should not fail.");
            let expected_value = StateValue::from(match state_key {
                StateKey::Raw(key) => key,
                _ => unreachable!(),
            });
            assert_eq!(result.0, Some(expected_value));
            assert!(result.1.is_none());
        }

        let proofs = fetcher.get_proof_cache();
        assert_eq!(proofs.len(), 10);
        assert_eq_unordered!(proofs.into_keys().collect::<Vec<_>>(), expected_key_hashes);
    }
}
