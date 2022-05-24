// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::DbReader;
use aptos_crypto::hash::{CryptoHash, SPARSE_MERKLE_PLACEHOLDER_HASH};
use aptos_crypto::HashValue;
use aptos_types::proof::SparseMerkleProof;
use aptos_types::state_store::state_key::StateKey;
use aptos_types::state_store::state_value::StateValue;
use aptos_types::transaction::Version;
use crossbeam_channel::{unbounded, Receiver, Sender};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::fmt::format;
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;

pub struct AsyncProofReader {
    status_receiver: Receiver<StatusCommand>,
    sender: Sender<Command>,
    receiver: Receiver<Command>,
    workers: Vec<Worker>,
    worker_threads: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl AsyncProofReader {
    pub fn new(
        num_threads: usize,
        status_sender: Sender<StatusCommand>,
        status_receiver: Receiver<StatusCommand>,
        cmd_sender: Sender<Command>,
        cmd_receiver: Receiver<Command>,
        reader: Arc<dyn DbReader>,
    ) -> Self {
        let workers: Vec<Worker> = (0..num_threads)
            .map(|i| Worker::new(status_sender.clone(), cmd_receiver.clone(), reader.clone()))
            .collect();
        let worker_threads: Vec<JoinHandle<()>> = (0..num_threads)
            .map(|i| {
                let worker = workers[i].clone();
                std::thread::Builder::new()
                    .name(format!("async-proof-reader-{}", i))
                    .spawn(move || worker.work())
                    .expect("Creating pruner thread should succeed.")
            })
            .collect();
        Self {
            status_receiver,
            sender: cmd_sender,
            receiver: cmd_receiver,
            workers,
            worker_threads: Arc::new(Mutex::new(worker_threads)),
        }
    }

    pub fn read_proof_async(&self, state_key: StateKey, version: Version) -> anyhow::Result<()> {
        self.sender
            .send(Command::AsyncRead { state_key, version })
            .map_err(anyhow::Error::from)
    }

    pub fn finish(&self) -> anyhow::Result<()> {
        self.sender
            .send(Command::Finish)
            .map_err(anyhow::Error::from)
    }

    pub fn finish_and_read_proofs(&self) -> HashMap<HashValue, SparseMerkleProof> {
        // Wait for all the threads to finish reading the proofs. This can be done by waiting for the
        // finish marker in the status channel.
        match self
            .status_receiver
            .recv()
            .expect("Sender should not destruct prematurely")
        {
            StatusCommand::Finished => {}
        }
        let mut state_proofs = HashMap::new();
        for worker in &self.workers {
            state_proofs.extend(worker.get_and_clear_state_proofs())
        }
        state_proofs
    }
}
impl Drop for AsyncProofReader {
    fn drop(&mut self) {
        // for thread in self.worker_threads.lock().into_iter() {
        //     thread
        //         .join()
        //         .expect("Worker thread should join peacefully.");
        // }
    }
}

#[derive(Clone)]
struct Worker {
    status_sender: Sender<StatusCommand>,
    receiver: Receiver<Command>,
    reader: Arc<dyn DbReader>,
    state_proofs: Arc<Mutex<HashMap<HashValue, SparseMerkleProof<StateValue>>>>,
}

impl Worker {
    pub fn new(
        status_sender: Sender<StatusCommand>,
        receiver: Receiver<Command>,
        reader: Arc<dyn DbReader>,
    ) -> Self {
        Self {
            status_sender,
            receiver,
            reader,
            state_proofs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn work(&self) {
        loop {
            match self
                .receiver
                .recv()
                .expect("Sender should not destruct prematurely")
            {
                Command::Finish => {
                    // Finish command acts as a marker for finishing a batch of async proof reads.
                    // Upon receiving this command, a thread just sends an ack on the status channel.
                    self.status_sender
                        .send(StatusCommand::Finished)
                        .expect("Failed to send Finished status on the status channel");
                }
                Command::AsyncRead { state_key, version } => self.read_proof(state_key, version),
                _ => {}
            }
        }
    }

    fn read_proof(&self, state_key: StateKey, version: Version) {
        let (_, proof) = self
            .reader
            // TODO(skedia): Replace this with a function which reads only the proof
            // and not value to save an extra DB read.
            .get_state_value_with_proof_by_version(&state_key, version)
            .expect("proof reading should succeed");
        self.state_proofs.lock().insert(state_key.hash(), proof);
    }

    // copy the snapshot of the state proofs into a separate hashmap
    pub fn get_and_clear_state_proofs(&self) -> HashMap<HashValue, SparseMerkleProof<StateValue>> {
        let state_proofs = self
            .state_proofs
            .lock()
            .iter()
            .map(|(x, y)| (*x, y.clone()))
            .collect();
        self.state_proofs.lock().clear();
        state_proofs
    }
}

pub enum Command {
    Finish,
    AsyncRead {
        state_key: StateKey,
        version: Version,
    },
}

pub enum StatusCommand {
    /// Used to notify that all the threads finished reading the required proofs.
    Finished,
}

fn main() {
    let (s1, r1) = unbounded();
    let (s2, r2) = (s1.clone(), r1.clone());
    let (s3, r3) = (s2.clone(), r2.clone());

    s1.send(10).unwrap();
    s2.send(20).unwrap();
    s3.send(30).unwrap();

    assert_eq!(r3.recv(), Ok(10));
    assert_eq!(r1.recv(), Ok(20));
    assert_eq!(r2.recv(), Ok(30));
}
