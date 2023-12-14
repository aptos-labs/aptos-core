// Copyright © Aptos Foundation

use aptos_channels::aptos_channel;
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_types::validator_txn::{Topic, ValidatorTransaction};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub enum ValidatorTransactionFilter {
    PendingTxnHashSet(HashSet<HashValue>),
}

impl ValidatorTransactionFilter {
    pub fn should_exclude(&self, txn: &ValidatorTransaction) -> bool {
        match self {
            ValidatorTransactionFilter::PendingTxnHashSet(set) => set.contains(&txn.hash()),
        }
    }
}

/// Create a validator transaction pool for a given list of topics.
/// For each topic, a notification sender can be optionally given and later be used to send notification when later this topic is pulled.
///
/// Return a pull client (typically used by consensus when proposing blocks).
/// Return write clients (typically used by validator transaction producers like DKG).
pub fn new(
    topic_tx_pairs: Vec<(Topic, Option<NotificationSender>)>,
) -> (VTxnPoolPullClient, Vec<WriteClient>) {
    let topics: Vec<Topic> = topic_tx_pairs.iter().map(|(topic, _)| *topic).collect();
    let pool = Arc::new(Mutex::new(ValidatorTransactionPool::new(topic_tx_pairs)));
    let pull_client = VTxnPoolPullClient { pool: pool.clone() };
    let write_clients = topics
        .into_iter()
        .map(|topic| WriteClient {
            pool: pool.clone(),
            topic,
        })
        .collect();
    (pull_client, write_clients)
}

pub struct WriteClient {
    pub pool: Arc<Mutex<ValidatorTransactionPool>>,
    pub topic: Topic,
}

impl WriteClient {
    pub fn put(&self, txn: Option<Arc<ValidatorTransaction>>) -> Option<Arc<ValidatorTransaction>> {
        let mut pool = self.pool.lock().unwrap();
        if let Some(txn) = txn {
            pool.txns.insert(self.topic, txn)
        } else {
            pool.txns.remove(&self.topic)
        }
    }
}

pub trait PullClient: Send + Sync {
    fn pull(
        &self,
        max_time: Duration,
        max_items: u64,
        max_bytes: u64,
        exclude: ValidatorTransactionFilter,
    ) -> Vec<ValidatorTransaction>;
}

pub type NotificationSender = aptos_channel::Sender<(), Arc<ValidatorTransaction>>;
pub type NotificationReceiver = aptos_channel::Receiver<(), Arc<ValidatorTransaction>>;

pub struct ValidatorTransactionPool {
    txns: HashMap<Topic, Arc<ValidatorTransaction>>,
    pull_notification_senders: HashMap<Topic, Mutex<NotificationSender>>,
    topics: Vec<Topic>,
}

impl ValidatorTransactionPool {
    pub fn new(topic_sender_pairs: Vec<(Topic, Option<NotificationSender>)>) -> Self {
        let topics = topic_sender_pairs.iter().map(|(topic, _)| *topic).collect();
        let pull_notification_senders = topic_sender_pairs
            .into_iter()
            .filter_map(|(topic, maybe_sender)| {
                maybe_sender.map(|sender| (topic, Mutex::new(sender)))
            })
            .collect();
        Self {
            txns: HashMap::new(),
            pull_notification_senders,
            topics,
        }
    }
}

impl Default for ValidatorTransactionPool {
    fn default() -> Self {
        Self::new(vec![])
    }
}

pub struct VTxnPoolPullClient {
    pool: Arc<Mutex<ValidatorTransactionPool>>,
}

impl PullClient for VTxnPoolPullClient {
    fn pull(
        &self,
        max_time: Duration,
        mut max_items: u64,
        mut max_bytes: u64,
        filter: ValidatorTransactionFilter,
    ) -> Vec<ValidatorTransaction> {
        let pull_start_time = Instant::now();
        let pool = self.pool.lock().unwrap();
        let mut ret = vec![];
        let mut txn_iterator = pool
            .topics
            .iter()
            .copied()
            .filter_map(|topic| pool.txns.get(&topic).cloned().map(|txn| (topic, txn)));
        while pull_start_time.elapsed() < max_time && max_items >= 1 && max_bytes >= 1 {
            if let Some((topic, txn)) = txn_iterator.next() {
                if filter.should_exclude(txn.as_ref()) {
                    continue;
                }
                let txn_size = txn.size_in_bytes() as u64;
                if txn_size > max_bytes {
                    continue;
                }
                // All checks passed.
                ret.push(txn.as_ref().clone());
                if let Some(tx) = pool.pull_notification_senders.get(&topic) {
                    tx.lock().unwrap().push((), txn).unwrap();
                }
                max_items -= 1;
                max_bytes -= txn_size;
            } else {
                break;
            }
        }

        ret
    }
}
