// Copyright © Aptos Foundation

use aptos_channels::aptos_channel;
#[cfg(test)]
use aptos_channels::message_queues::QueueStyle;
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_types::validator_txn::{Topic, ValidatorTransaction};
#[cfg(test)]
use futures_util::StreamExt;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
#[cfg(test)]
use tokio::time::timeout;

pub enum TransactionFilter {
    PendingTxnHashSet(HashSet<HashValue>),
}

impl TransactionFilter {
    pub fn should_exclude(&self, txn: &ValidatorTransaction) -> bool {
        match self {
            TransactionFilter::PendingTxnHashSet(set) => set.contains(&txn.hash()),
        }
    }
}

pub type PullNotificationSender = aptos_channel::Sender<(), Arc<ValidatorTransaction>>;
pub type PullNotificationReceiver = aptos_channel::Receiver<(), Arc<ValidatorTransaction>>;

/// Create a validator txn pool for a given list of topics.
/// For each topic, an optional notification sender can be specified,
/// which is used to send a notification later when a pull on the topic happens.
///
/// Return a read client (typically used by consensus when proposing blocks).
/// Return the write clients (typically used by validator transaction producers like DKG).
pub fn new(
    topic_tx_pairs: Vec<(Topic, Option<PullNotificationSender>)>,
) -> (ReadClient, Vec<SingleTopicWriteClient>) {
    let topics: Vec<Topic> = topic_tx_pairs.iter().map(|(topic, _)| *topic).collect();
    let pool_state = Arc::new(Mutex::new(PoolState::new(topic_tx_pairs)));
    let read_client = ReadClient {
        pool: pool_state.clone(),
    };
    let write_clients = topics
        .into_iter()
        .map(|topic| SingleTopicWriteClient {
            pool: pool_state.clone(),
            topic,
        })
        .collect();
    (read_client, write_clients)
}

struct PoolState {
    /// sorted by priority (high to low).
    topics: Vec<Topic>,

    /// Currently only support 1 txn per topic.
    txns: HashMap<Topic, Arc<ValidatorTransaction>>,

    pull_notification_senders: HashMap<Topic, Mutex<PullNotificationSender>>,
}

impl PoolState {
    pub fn new(topic_sender_pairs: Vec<(Topic, Option<PullNotificationSender>)>) -> Self {
        let topics: Vec<Topic> = topic_sender_pairs.iter().map(|(topic, _)| *topic).collect();
        let topic_set: HashSet<Topic> = HashSet::from_iter(topics.clone());
        assert_eq!(topics.len(), topic_set.len());
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

impl Default for PoolState {
    fn default() -> Self {
        Self::new(vec![])
    }
}

pub struct ReadClient {
    pool: Arc<Mutex<PoolState>>,
}

impl ReadClient {
    pub async fn pull(
        &self,
        max_time: Duration,
        mut max_items: u64,
        mut max_bytes: u64,
        filter: TransactionFilter,
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

pub struct SingleTopicWriteClient {
    pool: Arc<Mutex<PoolState>>,
    topic: Topic,
}

impl SingleTopicWriteClient {
    pub fn put(&self, txn: Option<Arc<ValidatorTransaction>>) -> Option<Arc<ValidatorTransaction>> {
        let mut pool = self.pool.lock().unwrap();
        if let Some(txn) = txn {
            pool.txns.insert(self.topic, txn)
        } else {
            pool.txns.remove(&self.topic)
        }
    }
}

#[cfg(test)]
#[tokio::test]
async fn test_validator_txn_pool() {
    // Create the pool with 2 topics: `dummy1` and `dummy2`. Also subscribe "pulled" notification for topic `dummy2`.
    let (dummy2_notification_tx, mut dummy2_notification_rx) =
        aptos_channel::new(QueueStyle::FIFO, 999, None);
    let (read_client, mut write_clients) = new(vec![
        (Topic::DUMMY1, None),
        (Topic::DUMMY2, Some(dummy2_notification_tx)),
    ]);
    let dummy2_write_client = write_clients.pop().unwrap();
    let dummy1_write_client = write_clients.pop().unwrap();

    // Initially nothing should be available on the read client side.
    let pulled = read_client
        .pull(
            Duration::from_secs(3600),
            999,
            2048,
            TransactionFilter::PendingTxnHashSet(HashSet::new()),
        )
        .await;
    assert!(pulled.is_empty());

    // Write a dummy2 txn in.
    let dummy2_txn = ValidatorTransaction::dummy2(b"dummy2_txn".to_vec());
    dummy2_write_client.put(Some(Arc::new(dummy2_txn.clone())));

    // The dummy2 txn should be available on the read client side.
    // Pull notification can be delivered.
    let pulled = read_client
        .pull(
            Duration::from_secs(3600),
            999,
            2048,
            TransactionFilter::PendingTxnHashSet(HashSet::new()),
        )
        .await;
    assert_eq!(vec![dummy2_txn.clone()], pulled);
    let notification = dummy2_notification_rx.next().await;
    assert_eq!(&dummy2_txn, notification.unwrap().as_ref());

    // Write a dummy1 txn in.
    let dummy1_txn = ValidatorTransaction::dummy1(b"dummy1_txn".to_vec());
    dummy1_write_client.put(Some(Arc::new(dummy1_txn.clone())));

    // Both txn should be available. For topic `dummy2` a pull notification can be delivered again.
    let pulled = read_client
        .pull(
            Duration::from_secs(3600),
            999,
            2048,
            TransactionFilter::PendingTxnHashSet(HashSet::new()),
        )
        .await;
    assert_eq!(vec![dummy1_txn.clone(), dummy2_txn.clone()], pulled);
    let notification = timeout(Duration::from_secs(1), dummy2_notification_rx.next()).await;
    assert_eq!(&dummy2_txn, notification.unwrap().unwrap().as_ref());

    // In a `pull()`, limit `max_items` should be respected, and lower-priority topic should be the victim.
    let pulled = read_client
        .pull(
            Duration::from_secs(3600),
            1,
            2048,
            TransactionFilter::PendingTxnHashSet(HashSet::new()),
        )
        .await;
    assert_eq!(vec![dummy1_txn.clone()], pulled);

    // In a `pull()`, limit `max_size` should be respected, and lower-priority topic should be the victim.
    let dummy1_txn_size = dummy1_txn.size_in_bytes() as u64;
    let pulled = read_client
        .pull(
            Duration::from_secs(3600),
            999,
            dummy1_txn_size,
            TransactionFilter::PendingTxnHashSet(HashSet::new()),
        )
        .await;
    assert_eq!(vec![dummy1_txn.clone()], pulled);

    // In a `pull()`,  txn filter should be respected.
    let dummy1_txn_hash = dummy1_txn.hash();
    let pulled = read_client
        .pull(
            Duration::from_secs(3600),
            999,
            2048,
            TransactionFilter::PendingTxnHashSet(HashSet::from([dummy1_txn_hash])),
        )
        .await;
    assert_eq!(vec![dummy2_txn.clone()], pulled);
    let notification = timeout(Duration::from_secs(1), dummy2_notification_rx.next()).await;
    assert_eq!(&dummy2_txn, notification.unwrap().unwrap().as_ref());

    // Write clients should be able to update/delete their proposals.
    dummy1_write_client.put(None);
    let dummy2_txn_ver_b = ValidatorTransaction::dummy1(b"dummy2_txn_ver_b".to_vec());
    dummy2_write_client.put(Some(Arc::new(dummy2_txn_ver_b.clone())));
    let pulled = read_client
        .pull(
            Duration::from_secs(3600),
            999,
            2048,
            TransactionFilter::PendingTxnHashSet(HashSet::from([dummy1_txn_hash])),
        )
        .await;
    assert_eq!(vec![dummy2_txn_ver_b.clone()], pulled);
    let notification = timeout(Duration::from_secs(1), dummy2_notification_rx.next()).await;
    assert_eq!(&dummy2_txn_ver_b, notification.unwrap().unwrap().as_ref());
}
