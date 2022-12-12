// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{TransactionCommitter, TransactionExecutor};
use aptos_executor::block_executor::BlockExecutor;
use aptos_executor_types::BlockExecutorTrait;
use aptos_logger::info;
use aptos_types::transaction::{Transaction, Version};
use aptos_vm::AptosVM;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread::JoinHandle,
    time::Duration,
};

pub struct Pipeline {
    join_handles: Vec<JoinHandle<()>>,
    start_commit: Arc<AtomicBool>,
}

impl Pipeline {
    pub fn new(
        executor: BlockExecutor<AptosVM>,
        version: Version,
        split_stages: bool,
    ) -> (Self, mpsc::SyncSender<Vec<Transaction>>) {
        let parent_block_id = executor.committed_block_id();
        let executor_1 = Arc::new(executor);
        let executor_2 = executor_1.clone();

        let (block_sender, block_receiver) =
            mpsc::sync_channel::<Vec<Transaction>>(50 /* bound */);
        let (commit_sender, commit_receiver) =
            mpsc::sync_channel(if split_stages { 1000 } else { 3 } /* bound */);

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

        let start_commit_1: Arc<AtomicBool> =
            Arc::new(AtomicBool::new(if split_stages { false } else { true }));
        let start_commit_2: Arc<AtomicBool> = start_commit_1.clone();

        let commit_thread = std::thread::Builder::new()
            .name("txn_committer".to_string())
            .spawn(move || {
                while !start_commit_2.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(100));
                }
                info!("Starting commit thread");
                let mut committer = TransactionCommitter::new(executor_2, version, commit_receiver);
                committer.run();
            })
            .expect("Failed to spawn transaction committer thread.");
        let join_handles = vec![exe_thread, commit_thread];

        (
            Self {
                join_handles,
                start_commit: start_commit_1,
            },
            block_sender,
        )
    }

    pub fn join(self) {
        self.start_commit.store(true, Ordering::Relaxed);
        for handle in self.join_handles {
            handle.join().unwrap()
        }
    }
}
