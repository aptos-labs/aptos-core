// Copyright © Aptos Foundation

use aptos_types::validator_txn::{
    pool::{ValidatorTransactionFilter, ValidatorTransactionPoolClient},
    ValidatorTransaction,
};
use std::{
    thread::sleep,
    time::{Duration, Instant},
};

#[cfg(test)]
pub struct DummyValidatorTxnClient {
    txns: Vec<ValidatorTransaction>,
}

#[cfg(test)]
impl DummyValidatorTxnClient {
    pub fn new(txns: Vec<ValidatorTransaction>) -> Self {
        Self { txns }
    }
}

#[cfg(test)]
impl ValidatorTransactionPoolClient for DummyValidatorTxnClient {
    fn pull(
        &self,
        max_time: Duration,
        mut max_items: u64,
        mut max_bytes: u64,
        _exclude: ValidatorTransactionFilter,
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
