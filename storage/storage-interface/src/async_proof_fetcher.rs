// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{metrics::TIMER, DbReader};
use anyhow::{anyhow, Result};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_logger::{error, sample, sample::SampleRate};
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
    string::ToString,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use threadpool::ThreadPool;

static IO_POOL: Lazy<ThreadPool> = Lazy::new(|| {
    ThreadPool::with_name(
        "proof_reader".to_string(),
        AptosVM::get_num_proof_reading_threads(),
    )
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

    pub fn fetch_state_value(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<(Version, StateValue)>> {
        let _timer = TIMER
            .with_label_values(&["async_proof_fetcher_fetch"])
            .start_timer();
        Ok(self
            .reader
            .get_state_value_with_version_by_version(state_key, version)?)
    }

    pub fn fetch_state_value_with_version_and_schedule_proof_read(
        &self,
        state_key: &StateKey,
        version: Version,
        subtree_root_depth: usize,
        subtree_root_hash: Option<HashValue>,
    ) -> Result<Option<(Version, StateValue)>> {
        let version_and_value_opt = self.fetch_state_value(state_key, version)?;
        self.schedule_proof_read(
            state_key.clone(),
            version,
            subtree_root_depth,
            subtree_root_hash,
            version_and_value_opt.as_ref().map(|v| {
                let state_value = &v.1;
                state_value.hash()
            }),
        );
        Ok(version_and_value_opt)
    }

    pub fn get_proof_cache(&self) -> HashMap<HashValue, SparseMerkleProofExt> {
        self.wait()
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
        subtree_root_depth: usize,
        subtree_root_hash: Option<HashValue>,
        value_hash: Option<HashValue>,
    ) {
        let _timer = TIMER
            .with_label_values(&["schedule_async_proof_read"])
            .start_timer();
        self.num_proofs_to_read.fetch_add(1, Ordering::SeqCst);
        let reader = self.reader.clone();
        let data_sender = self.data_sender.clone();
        IO_POOL.execute(move || {
            let proof = reader
                .get_state_proof_by_version_ext(&state_key, version, subtree_root_depth)
                .expect("Proof reading should succeed.");
            // NOTE: Drop the reader here to make sure reader has shorter lifetime than the async
            // proof fetcher.
            drop(reader);
            if let Some(subtree_root_hash) = subtree_root_hash {
                proof
                    .verify_by_hash(subtree_root_hash, state_key.hash(), value_hash)
                    .map_err(|err| {
                        anyhow!(
                            "Proof is invalid for key {:?} with subtree root hash {:?}, depth {}, at version {}: {}.",
                            state_key,
                            subtree_root_hash,
                            subtree_root_depth,
                            version,
                            err
                        )
                    })
                    .expect("Failed to verify proof.");
            }
            match data_sender.send(Proof {
                state_key_hash: state_key.hash(),
                proof,
            }) {
                Ok(_) => {}
                Err(_) => {
                    sample!(
                        SampleRate::Duration(Duration::from_secs(5)),
                        error!("Failed to send proof, something is wrong in execution.")
                    );
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockDbReaderWriter;
    use aptos_types::state_store::state_key::inner::StateKeyInner;
    use assert_unordered::assert_eq_unordered;

    #[test]
    fn test_fetch() {
        let fetcher = AsyncProofFetcher::new(Arc::new(MockDbReaderWriter));
        let mut expected_key_hashes = vec![];
        for i in 0..10 {
            let state_key: StateKey = StateKey::raw(format!("test_key_{}", i).as_bytes());
            expected_key_hashes.push(state_key.hash());
            let result = fetcher
                .fetch_state_value_with_version_and_schedule_proof_read(&state_key, 0, 0, None)
                .expect("Should not fail.");
            let expected_value = StateValue::from(match state_key.inner() {
                StateKeyInner::Raw(key) => key.to_owned(),
                _ => unreachable!(),
            });
            assert_eq!(result, Some((0, expected_value)));
        }

        let proofs = fetcher.get_proof_cache();
        assert_eq!(proofs.len(), 10);
        assert_eq_unordered!(proofs.into_keys().collect::<Vec<_>>(), expected_key_hashes);
    }
}
