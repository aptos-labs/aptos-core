// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{metrics::TIMER, DbReader};
use anyhow::{anyhow, Result};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_logger::{error, info, sample, sample::SampleRate};
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
    thread::JoinHandle,
    time::Duration,
};

struct Proof {
    state_key_hash: HashValue,
    proof: SparseMerkleProofExt,
}

enum Command {
    AsyncRead {
        state_key: StateKey,
        version: Version,
        root_hash: Option<HashValue>,
        value_hash: Option<HashValue>,
    },
}

pub struct AsyncProofFetcher {
    reader: Arc<dyn DbReader>,
    command_sender: Sender<Command>,
    data_receiver: Receiver<Proof>,
    num_proofs_to_read: AtomicUsize,
    worker_threads: Vec<Option<JoinHandle<()>>>,
}

impl AsyncProofFetcher {
    pub fn new(reader: Arc<dyn DbReader>) -> Self {
        let (command_sender, command_receiver) = unbounded();
        let (data_sender, data_receiver) = unbounded();
        let mut workers = vec![];

        for i in 0..AptosVM::get_num_proof_reading_threads() {
            let command_receiver = command_receiver.clone();
            let reader = reader.clone();
            let data_sender = data_sender.clone();
            let worker_thread = std::thread::Builder::new()
                .name(format!("async-proof-fetcher-master-{:?}", i))
                .spawn(move || Self::proof_read_loop(reader, data_sender, command_receiver))
                .expect("Creating proof fetcher thread should succeed.");
            workers.push(Some(worker_thread))
        }

        Self {
            reader,
            command_sender,
            data_receiver,
            num_proofs_to_read: AtomicUsize::new(0),
            worker_threads: workers,
        }
    }

    fn proof_read_loop(
        reader: Arc<dyn DbReader>,
        data_sender: Sender<Proof>,
        command_receiver: Receiver<Command>,
    ) {
        // Loop and Receive command from the channel.
        loop {
            let command = command_receiver.recv();
            if command.is_err() {
                error!(
                    "Failed to receive command on the channel, most likely the channel is closed."
                );
                break;
            }
            let command = command.unwrap();
            match command {
                Command::AsyncRead {
                    state_key,
                    version,
                    root_hash,
                    value_hash,
                } => {
                    let proof = reader
                        .get_state_proof_by_version_ext(&state_key, version)
                        .expect("Proof reading should succeed.");
                    if let Some(root_hash) = root_hash {
                        proof
                            .verify_by_hash(root_hash, state_key.hash(), value_hash)
                            .map_err(|err| {
                                anyhow!(
                            "Proof is invalid for key {:?} with state root hash {:?}, at version {}: {}.",
                            state_key,
                            root_hash,
                            version,
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
                        .expect("Failed to send proof, something is wrong in execution.");
                },
                _ => unreachable!(),
            }
        }
    }

    pub fn fetch_state_value_with_version_and_schedule_proof_read(
        &self,
        state_key: &StateKey,
        version: Version,
        root_hash: Option<HashValue>,
    ) -> Result<Option<(Version, StateValue)>> {
        let _timer = TIMER
            .with_label_values(&["async_proof_fetcher_fetch"])
            .start_timer();
        let version_and_value_opt = self
            .reader
            .get_state_value_with_version_by_version(state_key, version)?;
        self.schedule_proof_read(
            state_key.clone(),
            version,
            root_hash,
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
        root_hash: Option<HashValue>,
        value_hash: Option<HashValue>,
    ) {
        let _timer = TIMER
            .with_label_values(&["schedule_async_proof_read"])
            .start_timer();
        self.num_proofs_to_read.fetch_add(1, Ordering::SeqCst);
        self.command_sender
            .send(Command::AsyncRead {
                state_key,
                version,
                root_hash,
                value_hash,
            })
            .expect("Failed to send command on the channel.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockDbReaderWriter;
    use aptos_types::state_store::state_key::StateKeyInner;
    use assert_unordered::assert_eq_unordered;

    #[test]
    fn test_fetch() {
        let fetcher = AsyncProofFetcher::new(Arc::new(MockDbReaderWriter));
        let mut expected_key_hashes = vec![];
        for i in 0..10 {
            let state_key: StateKey = StateKey::raw(format!("test_key_{}", i).into_bytes());
            expected_key_hashes.push(state_key.hash());
            let result = fetcher
                .fetch_state_value_with_version_and_schedule_proof_read(&state_key, 0, None)
                .expect("Should not fail.");
            let expected_value = StateValue::from(match state_key.into_inner() {
                StateKeyInner::Raw(key) => key,
                _ => unreachable!(),
            });
            assert_eq!(result, Some((0, expected_value)));
        }

        let proofs = fetcher.get_proof_cache();
        assert_eq!(proofs.len(), 10);
        assert_eq_unordered!(proofs.into_keys().collect::<Vec<_>>(), expected_key_hashes);
    }
}
