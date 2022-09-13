// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{proof_fetcher::ProofFetcher, DbReader};

use crate::metrics::TIMER;
use anyhow::{anyhow, Result};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_types::{
    proof::SparseMerkleProofExt,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use aptos_vm::AptosVM;
use crossbeam_channel::{unbounded, Receiver, Sender};
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

static IO_POOL: Lazy<rayon::ThreadPool> = Lazy::new(|| {
    rayon::ThreadPoolBuilder::new()
        .num_threads(AptosVM::get_num_proof_reading_threads())
        .thread_name(|index| format!("proof_reader_{}", index))
        .build()
        .unwrap()
});

struct Proof {
    state_key_hash: HashValue,
    proof: SparseMerkleProofExt,
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

    // Waits scheduled proof read to finish, and returns all read proofs.
    //
    // This is only expected to be called in a single thread, after all reads being scheduled.
    fn wait(&self) -> HashMap<HashValue, SparseMerkleProofExt> {
        let _timer = TIMER.with_label_values(&["wait_async_proof"]).start_timer();
        // TODO(grao): Find a way to verify the proof.
        let mut proofs = HashMap::new();
        for _ in 0..self.num_proofs_to_read.load(Ordering::SeqCst) {
            let data = self
                .data_receiver
                .recv()
                .expect("Failed to receive proof on the channel.");
            let Proof {
                state_key_hash,
                proof,
            } = data;
            proofs.insert(state_key_hash, proof);
        }
        self.num_proofs_to_read.store(0, Ordering::SeqCst);
        proofs
    }

    // Schedules proof reading work in a background running thread pool.
    fn schedule_proof_read(
        &self,
        state_key: StateKey,
        version: Version,
        root_hash: Option<HashValue>,
        value_hash: Option<HashValue>,
    ) {
        let _timer = TIMER
            .with_label_values(&["schedule_async_proof_read"])
            .start_timer();
        self.num_proofs_to_read.fetch_add(1, Ordering::SeqCst);
        let reader = self.reader.clone();
        let data_sender = self.data_sender.clone();
        IO_POOL.spawn(move || {
            let proof = reader
                .get_state_proof_by_version_ext(&state_key, version)
                .expect("Proof reading should succeed.");
            if let Some(root_hash) = root_hash {
                proof
                    .verify_by_hash(root_hash, state_key.hash(), value_hash)
                    .map_err(|err| {
                        anyhow!(
                            "Proof is invalid for key {:?} with state root hash {:?}: {}.",
                            state_key,
                            root_hash,
                            err
                        )
                    })
                    .expect("Failed to verify proof.");
            }
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
        root_hash: Option<HashValue>,
    ) -> Result<(Option<StateValue>, Option<SparseMerkleProofExt>)> {
        let _timer = TIMER
            .with_label_values(&["async_proof_fetcher_fetch"])
            .start_timer();
        let value = self.reader.get_state_value_by_version(state_key, version)?;
        self.schedule_proof_read(
            state_key.clone(),
            version,
            root_hash,
            value.as_ref().map(|v| v.hash()),
        );
        Ok((value, None))
    }

    fn get_proof_cache(&self) -> HashMap<HashValue, SparseMerkleProofExt> {
        self.wait()
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
                .fetch_state_value_and_proof(&state_key, 0, None)
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
