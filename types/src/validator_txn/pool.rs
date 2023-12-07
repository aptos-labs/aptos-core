// Copyright © Aptos Foundation

use crate::validator_txn::ValidatorTransaction;
use aptos_crypto::HashValue;
use std::{collections::HashSet, time::Duration};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::oracle::OracleTopic;
use aptos_crypto::hash::CryptoHash;

pub enum ValidatorTransactionFilter {
    PendingTxnHashSet(HashSet<HashValue>),
}

impl ValidatorTransactionFilter {
    pub fn should_exclude(&self, txn: &ValidatorTransaction) -> bool {
        match self {
            ValidatorTransactionFilter::PendingTxnHashSet(set) => {
                set.contains(&txn.hash())
            }
        }
    }
}

pub trait ValidatorTransactionPoolWriter: Send + Sync {
    fn put(&self, txn: Option<ValidatorTransaction>) -> Option<ValidatorTransaction>;
}

pub struct ValidatorTransactionPoolTopicWriter {
    pub pool: Arc<ValidatorTransactionPool>,
    pub topic: OracleTopic,
}

impl ValidatorTransactionPoolWriter for ValidatorTransactionPoolTopicWriter {
    fn put(&self, txn: Option<ValidatorTransaction>) -> Option<ValidatorTransaction> {
        let mut txns_by_topic = self.pool.txns_by_topic.write().unwrap();
        let ret = txns_by_topic.remove(&self.topic);
        if let Some(txn) = txn {
            txns_by_topic.insert(self.topic, txn);
        }
        ret
    }
}

pub trait ValidatorTransactionPoolClient: Send + Sync {
    fn pull(
        &self,
        max_time: Duration,
        max_items: u64,
        max_bytes: u64,
        exclude: ValidatorTransactionFilter,
    ) -> Vec<ValidatorTransaction>;
}

pub struct ValidatorTransactionPool {
    txns_by_topic: RwLock<HashMap<OracleTopic, ValidatorTransaction>>,
    topics: Vec<OracleTopic>,
}

impl ValidatorTransactionPool {
    pub fn new(topics: Vec<OracleTopic>) -> Self {
        Self { txns_by_topic: RwLock::new(HashMap::new()), topics }
    }
}

impl Default for ValidatorTransactionPool {
    fn default() -> Self {
        Self::new(vec![])
    }
}

impl ValidatorTransactionPoolClient for ValidatorTransactionPool {
    fn pull(
        &self,
        _max_time: Duration,
        mut max_items: u64,
        mut max_bytes: u64,
        filter: ValidatorTransactionFilter,
    ) -> Vec<ValidatorTransaction> {
        let txns_by_topic = self.txns_by_topic.read().unwrap();
        let mut ret = vec![];

        for txn in self.topics.iter().filter_map(|topic| txns_by_topic.get(topic)).filter(|txn| !filter.should_exclude(txn)) {
            let txn_size = txn.size_in_bytes() as u64;
            if max_items >= 1 && max_bytes >= txn_size {
                ret.push(txn.clone());
                max_items -= 1;
                max_bytes -= txn_size;
            }
        }

        ret
    }
}
