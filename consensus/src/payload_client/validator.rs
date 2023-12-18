// Copyright © Aptos Foundation

use aptos_types::validator_txn::ValidatorTransaction;
use aptos_validator_transaction_pool as vtxn_pool;
use std::{
    thread::sleep,
    time::{Duration, Instant},
};

#[async_trait::async_trait]
pub trait ValidatorTxnPayloadClient: Send + Sync {
    async fn pull(
        &self,
        max_time: Duration,
        max_items: u64,
        max_bytes: u64,
        exclude: vtxn_pool::TransactionFilter,
    ) -> Vec<ValidatorTransaction>;
}

pub struct DummyValidatorTxnClient {
    txns: Vec<ValidatorTransaction>,
}

impl DummyValidatorTxnClient {
    pub fn new(txns: Vec<ValidatorTransaction>) -> Self {
        Self { txns }
    }
}

#[async_trait::async_trait]
impl ValidatorTxnPayloadClient for DummyValidatorTxnClient {
    async fn pull(
        &self,
        max_time: Duration,
        mut max_items: u64,
        mut max_bytes: u64,
        _exclude: vtxn_pool::TransactionFilter,
    ) -> Vec<ValidatorTransaction> {
        let timer = Instant::now();
        let mut nxt_txn_idx = 0;
        let mut ret = vec![];
        while timer.elapsed() < max_time
            && max_items > 0
            && max_bytes > 0
            && nxt_txn_idx < self.txns.len()
        {
            sleep(Duration::from_millis(1));
            let txn = self.txns[nxt_txn_idx].clone();
            let txn_size = txn.size_in_bytes() as u64;
            if txn_size > max_bytes {
                break;
            }
            ret.push(txn);
            max_items -= 1;
            max_bytes -= txn_size;
            nxt_txn_idx += 1;
        }
        ret
    }
}

#[async_trait::async_trait]
impl ValidatorTxnPayloadClient for vtxn_pool::ReadClient {
    async fn pull(
        &self,
        max_time: Duration,
        max_items: u64,
        max_bytes: u64,
        filter: vtxn_pool::TransactionFilter,
    ) -> Vec<ValidatorTransaction> {
        vtxn_pool::ReadClient::pull(self, max_time, max_items, max_bytes, filter).await
    }
}
