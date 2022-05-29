// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{proof_fetcher::ProofFetcher, DbReader};

use aptos_crypto::{_once_cell::sync::Lazy, hash::CryptoHash, HashValue};
use aptos_metrics::{register_histogram, Histogram};
use aptos_types::{
    proof::SparseMerkleProof,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use crossbeam_channel::{unbounded, Receiver, Sender};
use parking_lot::Mutex;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread::JoinHandle,
};

pub struct AsyncProofFetcher {
    reader: Arc<dyn DbReader>,
    status_receiver: Mutex<Receiver<DataCommand>>,
    sender: Mutex<Sender<ControlCommand>>,
    /// The worker thread handle, created upon Pruner instance construction and joined upon its
    /// destruction. It only becomes `None` after joined in `drop()`.
    _worker_threads: Vec<Option<JoinHandle<()>>>,
    num_proofs_to_read: AtomicUsize,
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
        let (cmd_sender, cmd_receiver) = unbounded();
        let (data_sender, data_receiver) = unbounded();

        let mut workers = vec![];

        for i in 0..10 {
            let mut worker = Worker::new(data_sender.clone(), cmd_receiver.clone(), reader.clone());
            let worker_thread = std::thread::Builder::new()
                .name(format!("async-proof-fetcher-master-{:?}", i))
                .spawn(move || worker.work())
                .expect("Creating proof fetcher thread should succeed.");
            workers.push(Some(worker_thread))
        }

        Self {
            reader,
            status_receiver: Mutex::new(data_receiver),
            sender: Mutex::new(cmd_sender),
            _worker_threads: workers,
            num_proofs_to_read: AtomicUsize::new(0),
        }
    }

    pub fn read_proof_async(&self, state_key: StateKey, version: Version) -> anyhow::Result<()> {
        self.num_proofs_to_read.fetch_add(1, Ordering::Relaxed);
        self.sender
            .lock()
            .send(ControlCommand::AsyncRead { state_key, version })
            .map_err(anyhow::Error::from)
    }

    pub fn finish_and_read_proofs(&self) -> HashMap<HashValue, SparseMerkleProof> {
        let mut proofs = HashMap::new();
        for _ in 0..self.num_proofs_to_read.load(Ordering::Relaxed) {
            let data = self
                .status_receiver
                .lock()
                .recv()
                .expect("Failed to receive proof on the channel");
            match data {
                DataCommand::Proof { proof } => {
                    proofs.insert(proof.0, proof.1);
                }
            }
        }
        // Reset the number of proofs to read for the next round.
        self.num_proofs_to_read.store(0, Ordering::Relaxed);
        proofs
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
        // Drop the sender so that the receiver threads exit
        drop(self.sender.lock());
        // self.sender
        //     .lock()
        //     .send(ControlCommand::Quit)
        //     .expect("Receiver should not destruct.");
        // self.worker_thread
        //     .take()
        //     .expect("worker thread should exists")
        //     .join()
        //     .expect("Worker thread should join peacefully.");
    }
}

struct Worker {
    status_sender: Mutex<Sender<DataCommand>>,
    receiver: Mutex<Receiver<ControlCommand>>,
    reader: Arc<dyn DbReader>,
}

impl Worker {
    pub fn new(
        status_sender: Sender<DataCommand>,
        receiver: Receiver<ControlCommand>,
        reader: Arc<dyn DbReader>,
    ) -> Self {
        Self {
            status_sender: Mutex::new(status_sender),
            receiver: Mutex::new(receiver),
            reader,
        }
    }

    /// Blocks until we receive a command of batch size or receive a finish command. This is done
    /// to ensure that we have sufficient number of AsyncReads in batch to be able to parallelize
    /// the read.

    pub fn work(&mut self) {
        loop {
            if let Ok(ControlCommand::AsyncRead { state_key, version }) =
                self.receiver.lock().recv()
            {
                self.process_proof_read(state_key, version)
            }
        }
    }

    fn process_proof_read(&self, state_key: StateKey, version: Version) {
        let proof = self
            .reader
            // TODO(skedia): Replace this with a function which reads only the proof
            // and not value to save an extra DB read.
            .get_proof_for_state_key_by_version(&state_key, version)
            .expect("proof reading should succeed");
        self.status_sender
            .lock()
            .send(DataCommand::Proof {
                proof: (state_key.hash(), proof),
            })
            .expect("Sending proof should succeed");
    }
}

pub enum ControlCommand {
    AsyncRead {
        state_key: StateKey,
        version: Version,
    },
}

pub enum DataCommand {
    /// Used to notify that all the threads finished reading the required proofs.
    Proof {
        proof: (HashValue, SparseMerkleProof),
    },
}
