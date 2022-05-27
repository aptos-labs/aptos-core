// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{proof_fetcher::ProofFetcher, DbReader};

use aptos_crypto::_once_cell::sync::Lazy;
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_metrics::{register_histogram, Histogram};
use aptos_types::{
    proof::SparseMerkleProof,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use parking_lot::Mutex;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{
    collections::HashMap,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    thread::JoinHandle,
};

pub struct AsyncProofFetcher {
    reader: Arc<dyn DbReader>,
    status_receiver: Mutex<Receiver<StatusCommand>>,
    sender: Mutex<Sender<Command>>,
    /// The worker thread handle, created upon Pruner instance construction and joined upon its
    /// destruction. It only becomes `None` after joined in `drop()`.
    worker_thread: Option<JoinHandle<()>>,
}

pub static FETCH_STATE_VALUE: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "fetch_state_value",
        // metric description
        "The total time spent in seconds of block execution in the block executor."
    )
    .unwrap()
});

impl AsyncProofFetcher {
    pub fn new(reader: Arc<dyn DbReader>) -> Self {
        let (cmd_sender, cmd_receiver) = channel();
        let (status_sender, status_receiver) = channel();

        let mut worker = Worker::new(status_sender, cmd_receiver, reader.clone());
        let worker_thread = std::thread::Builder::new()
            .name("async-proof-fetcher-master".to_string())
            .spawn(move || worker.work())
            .expect("Creating proof fetcher thread should succeed.");

        Self {
            reader,
            status_receiver: Mutex::new(status_receiver),
            sender: Mutex::new(cmd_sender),
            worker_thread: Some(worker_thread),
        }
    }

    pub fn read_proof_async(&self, state_key: StateKey, version: Version) -> anyhow::Result<()> {
        self.sender
            .lock()
            .send(Command::AsyncRead { state_key, version })
            .map_err(anyhow::Error::from)
    }

    pub fn finish_and_read_proofs(&self) -> HashMap<HashValue, SparseMerkleProof> {
        // Send the finish marker to the worker threads, once all the work is
        self.sender
            .lock()
            .send(Command::FinishBatch)
            .expect("Receiver should not destruct prematurely");

        // Wait for the master thread to finish reading the proofs. This can be done by waiting for the
        // finish marker in the status channel.
        return match self
            .status_receiver
            .lock()
            .recv()
            .expect("Sender should not destruct prematurely")
        {
            StatusCommand::Finished { proofs } => proofs,
        };
    }
}

impl ProofFetcher for AsyncProofFetcher {
    fn fetch_state_value_and_proof(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> anyhow::Result<(Option<StateValue>, Option<SparseMerkleProof>)> {
        // Send command to the async proof fetcher thread to read the proof in async.
        self.read_proof_async(state_key.clone(), version)?;
        Ok((
            self.reader.get_state_value_by_version(state_key, version)?,
            None,
        ))
    }

    fn get_proof_cache(&self) -> HashMap<HashValue, SparseMerkleProof> {
        self.finish_and_read_proofs()
    }
}

impl Drop for AsyncProofFetcher {
    fn drop(&mut self) {
        self.sender
            .lock()
            .send(Command::Quit)
            .expect("Receiver should not destruct.");
        self.worker_thread
            .take()
            .expect("worker thread should exists")
            .join()
            .expect("Worker thread should join peacefully.");
    }
}

struct Worker {
    status_sender: Mutex<Sender<StatusCommand>>,
    receiver: Mutex<Receiver<Command>>,
    reader: Arc<dyn DbReader>,
    state_proofs: HashMap<HashValue, SparseMerkleProof>,
}

impl Worker {
    pub fn new(
        status_sender: Sender<StatusCommand>,
        receiver: Receiver<Command>,
        reader: Arc<dyn DbReader>,
    ) -> Self {
        Self {
            status_sender: Mutex::new(status_sender),
            receiver: Mutex::new(receiver),
            reader,
            state_proofs: HashMap::new(),
        }
    }

    /// Blocks until we receive a command of batch size or receive a finish command. This is done
    /// to ensure that we have sufficient number of AsyncReads in batch to be able to parallelize
    /// the read.
    pub fn receive_command_batch(&self, batch_size: usize) -> (Vec<Command>, bool, bool) {
        let mut commands = vec![];
        loop {
            match self
                .receiver
                .lock()
                .recv()
                .expect("Sender should not destruct prematurely")
            {
                Command::FinishBatch => {
                    // Received a finish command, no need to wait for more batches further.
                    return (commands, true, false);
                }
                Command::AsyncRead { state_key, version } => {
                    commands.push(Command::AsyncRead { state_key, version });
                    if commands.len() >= batch_size {
                        return (commands, false, false);
                    }
                }
                Command::Quit => {
                    return (commands, false, true);
                }
            }
        }
    }

    pub fn work(&mut self) {
        loop {
            let (commands, finish, quit) = self.receive_command_batch(100);
            // Read the proofs in parallel to speed up the proof reading
            let proofs: HashMap<HashValue, SparseMerkleProof> = commands
                .into_par_iter()
                .map(|command| match command {
                    Command::AsyncRead { state_key, version } => {
                        self.read_proof(state_key, version)
                    }
                    _ => panic!("Unexpected command encountered during proof reading"),
                })
                .collect();
            self.state_proofs.extend(proofs);
            if finish {
                // Finish command acts as a marker for finishing a batch of async proof reads.
                // Upon receiving this command, the thread just returns all the proofs read so far.
                let proofs = self.get_and_clear_state_proofs();
                self.status_sender
                    .lock()
                    .send(StatusCommand::Finished { proofs })
                    .expect("Failed to send Finished status on the status channel");
            }
            if quit {
                break;
            }
        }
    }

    fn read_proof(&self, state_key: StateKey, version: Version) -> (HashValue, SparseMerkleProof) {
        let (_, proof) = self
            .reader
            // TODO(skedia): Replace this with a function which reads only the proof
            // and not value to save an extra DB read.
            .get_state_value_with_proof_by_version(&state_key, version)
            .expect("proof reading should succeed");
        (state_key.hash(), proof)
        // self.state_proofs.lock().insert(state_key.hash(), proof);
    }

    // copy the snapshot of the state proofs into a separate hashmap
    pub fn get_and_clear_state_proofs(&mut self) -> HashMap<HashValue, SparseMerkleProof> {
        let mut state_proofs = HashMap::new();
        self.state_proofs.iter().for_each(|(x, y)| {
            state_proofs.insert(*x, y.clone());
        });
        self.state_proofs.clear();
        state_proofs
    }
}

pub enum Command {
    FinishBatch,
    AsyncRead {
        state_key: StateKey,
        version: Version,
    },
    // Quits the thread
    Quit,
}

pub enum StatusCommand {
    /// Used to notify that all the threads finished reading the required proofs.
    Finished {
        proofs: HashMap<HashValue, SparseMerkleProof>,
    },
}
