// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Condvar, Mutex};
use once_cell::sync::OnceCell;
use aptos_block_executor::transaction_provider::TxnProvider;
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;
use aptos_types::transaction::signature_verified_transaction::SignatureVerifiedTransaction;
use execution_metrics::REMOTE_EXECUTOR_TIMER_V2;

static SHARD_ID: OnceCell<usize> = OnceCell::new();

pub fn static_set_shard_id(shard_id: usize) {
    SHARD_ID.set(shard_id).unwrap();
}

pub struct StreamedTransactionsProvider {
    txns: Vec<Arc<SignatureVerifiedTransaction>>
}

impl StreamedTransactionsProvider {
    pub fn new(txns: Vec<SignatureVerifiedTransaction>) -> Self {
        let arc_txns = txns.into_iter().map(|txn| Arc::new(txn)).collect();
        Self {
            txns: arc_txns
        }
    }

    pub fn from_slice(txns: &[SignatureVerifiedTransaction]) -> Self {
        let arc_txns = txns.iter().map(|txn| Arc::new(txn.clone())).collect();
        Self {
            txns: arc_txns
        }
    }
}

impl TxnProvider<SignatureVerifiedTransaction> for StreamedTransactionsProvider {
    fn get_txn(&self, idx: usize) -> Arc<SignatureVerifiedTransaction> {
        self.txns[idx].clone()
    }

    fn num_txns(&self) -> usize {
        self.txns.len()
    }

    fn iter(&self) -> Box<dyn Iterator<Item = Arc<SignatureVerifiedTransaction>> + '_> {
        Box::new(self.txns.iter().cloned())
    }
}

pub struct BlockingTransactionsProvider {
    txns: Vec<(Mutex<CommandValue>, Condvar)>,
}

impl BlockingTransactionsProvider {
    pub fn new(num_txns: usize) -> Self {
        let mut txns = Vec::new();
        for _ in 0..num_txns {
            txns.push((Mutex::new(CommandValue::Waiting), Condvar::new()));
        }
        Self {
            txns
        }
    }

    pub fn set_txn(&self, idx: usize, txn: AnalyzedTransaction) {
        let (lock, cvar) = &self.txns[idx];
        let mut status = lock.lock().unwrap();
        *status = CommandValue::Ready(Arc::new(txn.into_txn()));
        cvar.notify_all();
    }
}

impl TxnProvider<SignatureVerifiedTransaction> for BlockingTransactionsProvider {
    fn get_txn(&self, idx: usize) -> Arc<SignatureVerifiedTransaction> {
        let (lock, cvar) = &self.txns[idx];
        let mut status = lock.lock().unwrap();
        let timer = REMOTE_EXECUTOR_TIMER_V2
            .with_label_values(&[&SHARD_ID.get().unwrap().to_string(), "get_txn_avg_waiting_time"])
            .start_timer();
        while let CommandValue::Waiting = *status {
            status = cvar.wait(status).unwrap();
        }
        drop(timer);
        match &*status {
            CommandValue::Ready(txn) => txn.clone(),
            CommandValue::Waiting => unreachable!(),
        }
    }

    fn num_txns(&self) -> usize {
        self.txns.len()
    }

    fn iter(&self) -> Box<dyn Iterator<Item = Arc<SignatureVerifiedTransaction>> + '_> {
        //Box::new(self.txns.iter().cloned())
        Box::new(self.txns.iter().map(|(lock, _)| {
            let status = lock.lock().unwrap();
            match &*status {
                CommandValue::Ready(txn) => txn.clone(),
                CommandValue::Waiting => unreachable!(),
            }
        }))
    }
}

pub enum CommandValue {
    /// The state value is available as a result of cross shard execution
    Ready(Arc<SignatureVerifiedTransaction>),
    /// We are still waiting for remote shard to push the state value
    Waiting,
}