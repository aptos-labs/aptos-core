// Copyright © Aptos Foundation

use crate::pipeline::ParToExeMsg;
use aptos_block_partitioner::sharded_block_partitioner::ShardedBlockPartitioner;
use aptos_crypto::HashValue;
use aptos_logger::info;
use aptos_types::{
    block_executor::partitioner::{
        CrossShardDependencies, ExecutableBlock, ExecutableTransactions,
        TransactionWithDependencies,
    },
    transaction::Transaction,
};
use std::{
    sync::mpsc::Receiver,
    time::Instant,
};
use std::sync::mpsc::SyncSender;

pub(crate) struct BlockPartitioningStage {
    num_iterations: usize,
    executable_block_sender: SyncSender<ParToExeMsg>,
    maybe_exe_fin_receiver: Option<Receiver<()>>,
    maybe_partitioner: Option<ShardedBlockPartitioner>,
}

impl BlockPartitioningStage {
    pub fn new(
        executable_block_sender: SyncSender<ParToExeMsg>,
        maybe_exe_fin_receiver: Option<Receiver<()>>,
        num_shards: usize,
    ) -> Self {
        let maybe_partitioner = if num_shards <= 1 {
            None
        } else {
            let partitioner = ShardedBlockPartitioner::new(num_shards);
            Some(partitioner)
        };

        Self {
            num_iterations: 0,
            executable_block_sender,
            maybe_exe_fin_receiver,
            maybe_partitioner,
        }
    }

    pub fn process(&mut self, mut txns: Vec<Transaction>) {
        let current_block_start_time = Instant::now();
        info!(
            "In iteration {}, received {:?} transactions.",
            self.num_iterations,
            txns.len()
        );
        let block_id = HashValue::random();
        let block: ExecutableBlock<Transaction> = match &self.maybe_partitioner {
            None => (block_id, txns).into(),
            Some(partitioner) => {
                let last_txn = txns.pop().unwrap();
                assert!(matches!(last_txn, Transaction::StateCheckpoint(_)));
                let analyzed_transactions = txns.into_iter().map(|t| t.into()).collect();
                let mut sub_blocks = partitioner.partition(analyzed_transactions, 2);
                sub_blocks
                    .last_mut()
                    .unwrap()
                    .sub_blocks
                    .last_mut()
                    .unwrap()
                    .transactions
                    .push(TransactionWithDependencies::new(
                        last_txn,
                        CrossShardDependencies::default(),
                    ));
                ExecutableBlock::new(block_id, ExecutableTransactions::Sharded(sub_blocks))
            },
        };
        let msg = ParToExeMsg {
            current_block_start_time,
            block,
        };
        self.executable_block_sender.send(msg).unwrap();
        if let Some(rx) = &self.maybe_exe_fin_receiver {
            rx.recv().unwrap();
        }
        self.num_iterations += 1;
    }
}
