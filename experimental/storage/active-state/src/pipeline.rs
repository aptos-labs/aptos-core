// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    committer::ActionCommitter, executor::ActionExecutor, generator::ActionGenerator,
    utils::BasicProofReader, MAX_ITEMS,
};
use aptos_config::config::{RocksdbConfigs, StorageDirPaths};
use aptos_crypto::hash::SPARSE_MERKLE_PLACEHOLDER_HASH;
use aptos_db::state_merkle_db::StateMerkleDb;
use aptos_logger::info;
use aptos_scratchpad::SparseMerkleTree;
use aptos_types::state_store::{
    state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
};
use std::{
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    thread,
    thread::sleep,
    time::Duration,
};
pub enum Action {
    Read(StateKey),
    Write(StateKey, Option<StateValue>),
}
#[derive(Clone, Copy)]
pub struct ActionConfig {
    // The number of read and write in each batch
    pub count: usize,
    // per million TODO: add read into the batch
    pub read_ratio: u32,
    // per million
    pub delete_ratio: u32,
    // largest write statekey generated to keep track of all keys generated
    pub last_state_key_ind: usize,
}

#[derive(Clone, Copy)]
pub enum ExecutionMode {
    AST,
    StatusQuo,
}

pub struct CommitMessage {
    // The updates to be applied to the state tree
    pub updates: Vec<(StateKey, Option<StateValue>)>,
    pub smt: Option<SparseMerkleTree<StateValue>>,
}

impl CommitMessage {
    pub fn new(
        updates: Vec<(StateKey, Option<StateValue>)>,
        smt: Option<SparseMerkleTree<StateValue>>,
    ) -> Self {
        Self { updates, smt }
    }
}

pub struct PipelineConfig {
    batch_size: usize,
    total_input_size: usize,
    db_path: String,
    execution_mode: ExecutionMode,
}

impl PipelineConfig {
    pub fn new(
        batch_size: usize,
        total_input_size: usize,
        db_path: String,
        execution_mode: ExecutionMode,
    ) -> Self {
        Self {
            batch_size,
            total_input_size,
            db_path,
            execution_mode,
        }
    }
}

pub struct Pipeline {
    config: PipelineConfig,
    sender: Sender<ActionConfig>,
    handle: thread::JoinHandle<()>,
}

impl Pipeline {
    pub fn create_empty_smt() -> SparseMerkleTree<StateValue> {
        SparseMerkleTree::<StateValue>::new(
            *SPARSE_MERKLE_PLACEHOLDER_HASH,
            StateStorageUsage::new_untracked(),
        )
    }

    pub fn new(config: PipelineConfig) -> Self {
        // setup the channel between pipeline and genearator
        let (updates_sender, updates_receiver): (Sender<ActionConfig>, Receiver<ActionConfig>) =
            channel();

        // setup the channel between generate and executor
        let (action_sender, action_receiver): (Sender<Vec<Action>>, Receiver<Vec<Action>>) =
            channel();

        // setup the channel betwen the executor and committer
        let (committer_sender, committer_receiver): (
            Sender<CommitMessage>,
            Receiver<CommitMessage>,
        ) = channel();

        let db_path = config.db_path.clone();
        let handle3 = thread::spawn(move || {
            let base_smt: SparseMerkleTree<StateValue> = Pipeline::create_empty_smt();
            let state_merkle_db = Arc::new(
                StateMerkleDb::new(
                    &StorageDirPaths::from_path(&db_path),
                    RocksdbConfigs::default(),
                    false,
                    1000000usize,
                )
                .unwrap(),
            );
            let mut committer: ActionCommitter =
                ActionCommitter::new(state_merkle_db, committer_receiver, Some(base_smt));
            committer.run();
        });

        let handle2 = thread::spawn(move || {
            let base_smt: SparseMerkleTree<StateValue> = Pipeline::create_empty_smt();
            //TODO(bowu): This is not a good proximation for the status quo since the the proofs are async fetched from the DB
            let proof_reader = BasicProofReader::new();
            let mut executor = match config.execution_mode {
                // the proof reader should be handled differently for AST and StatusQuo
                ExecutionMode::AST => ActionExecutor::new(
                    config.execution_mode,
                    proof_reader,
                    base_smt,
                    action_receiver,
                    committer_sender,
                    MAX_ITEMS,
                ),
                ExecutionMode::StatusQuo => ActionExecutor::new(
                    config.execution_mode,
                    proof_reader,
                    base_smt,
                    action_receiver,
                    committer_sender,
                    MAX_ITEMS,
                ),
            };
            executor.set_committer_handle(handle3);
            executor.run();
        });

        let handle1 = thread::spawn(|| {
            let mut generator = ActionGenerator::new(updates_receiver, action_sender);
            generator.set_executor_handle(handle2);
            generator.run();
        });

        Self {
            config,
            sender: updates_sender,
            handle: handle1,
        }
    }

    pub fn run(self) {
        let action_config = ActionConfig {
            count: self.config.batch_size,
            read_ratio: 0,
            delete_ratio: 0,
            last_state_key_ind: 0,
        };

        let mut input_count = 0;

        loop {
            info!("total input count: {}", input_count);
            if input_count >= self.config.total_input_size {
                // notify to stop
                self.sender
                    .send(ActionConfig {
                        count: 0,
                        read_ratio: 0,
                        delete_ratio: 0,
                        last_state_key_ind: 0,
                    })
                    .unwrap();
                break;
            }
            self.sender.send(action_config).unwrap();
            sleep(Duration::from_secs(1));
            input_count += self.config.batch_size;
        }

        self.handle.join().unwrap();
    }
}
