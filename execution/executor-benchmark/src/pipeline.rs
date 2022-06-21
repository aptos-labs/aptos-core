// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    state_committer::StateCommitter, StateCommitMaker, TransactionCommitter, TransactionExecutor,
};
use aptos_logger::info;
use aptos_types::transaction::{Transaction, Version};
use aptos_vm::AptosVM;
use aptosdb::state_store::StateStore;
use executor::block_executor::BlockExecutor;
use executor_types::BlockExecutorTrait;
use std::{
    sync::{mpsc, Arc},
    thread::JoinHandle,
};

pub struct Pipeline {
    join_handles: Vec<JoinHandle<()>>,
}

impl Pipeline {
    pub fn new(
        store: Arc<StateStore>,
        executor: BlockExecutor<AptosVM>,
        version: Version,
    ) -> (Self, mpsc::SyncSender<Vec<Transaction>>) {
        let parent_block_id = executor.committed_block_id();
        let base_smt = executor.root_smt();
        let executor_1 = Arc::new(executor);
        let executor_2 = executor_1.clone();

        let (block_sender, block_receiver) =
            mpsc::sync_channel::<Vec<Transaction>>(50 /* bound */);
        let (commit_sender, commit_receiver) = mpsc::sync_channel(3 /* bound */);
        let (state_delta_sender, state_delta_receiver) = mpsc::sync_channel(5 /* bound */);
        let (state_commit_sender, state_commit_receiver) = mpsc::sync_channel(2 /* bound */);

        let exe_thread = std::thread::Builder::new()
            .name("txn_executor".to_string())
            .spawn(move || {
                let mut exe = TransactionExecutor::new(
                    executor_1,
                    parent_block_id,
                    version,
                    Some(commit_sender),
                );
                while let Ok(transactions) = block_receiver.recv() {
                    info!("Received block of size {:?} to execute", transactions.len());
                    exe.execute_block(transactions);
                }
            })
            .expect("Failed to spawn transaction executor thread.");
        let commit_thread = std::thread::Builder::new()
            .name("txn_committer".to_string())
            .spawn(move || {
                let mut committer = TransactionCommitter::new(
                    executor_2,
                    version,
                    commit_receiver,
                    state_delta_sender,
                );
                committer.run();
            })
            .expect("Failed to spawn transaction committer thread.");
        let store_clone = store.clone();
        let state_commit_maker_thread = std::thread::Builder::new()
            .name("state_commit_maker".to_string())
            .spawn(move || {
                let maker = StateCommitMaker::new(
                    state_delta_receiver,
                    state_commit_sender,
                    store_clone,
                    base_smt,
                    Some(version),
                );
                maker.run();
            })
            .expect("Failed to spawn state commit maker thread.");
        let state_commit_thread = std::thread::Builder::new()
            .name("state_committer".to_string())
            .spawn(move || {
                let committer = StateCommitter::new(state_commit_receiver, store);
                committer.run();
            })
            .expect("Failed to spawn state committer thread.");

        let join_handles = vec![
            exe_thread,
            commit_thread,
            state_commit_maker_thread,
            state_commit_thread,
        ];

        (Self { join_handles }, block_sender)
    }

    pub fn join(self) {
        for handle in self.join_handles {
            handle.join().unwrap()
        }
    }
}
