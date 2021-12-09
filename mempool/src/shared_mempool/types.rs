// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

//! Objects used by/related to shared mempool
use crate::{
    core_mempool::CoreMempool, network::MempoolNetworkInterface,
    shared_mempool::network::MempoolNetworkSender,
};
use anyhow::Result;
use diem_config::{
    config::{MempoolConfig, RoleType},
    network_id::{NetworkId, PeerNetworkId},
};
use diem_crypto::HashValue;
use diem_infallible::{Mutex, RwLock};
use diem_types::{
    account_address::AccountAddress, mempool_status::MempoolStatus, transaction::SignedTransaction,
    vm_status::DiscardedVMStatus,
};
use futures::{
    channel::{mpsc, mpsc::UnboundedSender, oneshot},
    future::Future,
    task::{Context, Poll},
};
use network::{application::storage::PeerMetadataStorage, transport::ConnectionMetadata};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt,
    pin::Pin,
    sync::Arc,
    task::Waker,
    time::{Instant, SystemTime},
};
use storage_interface::DbReader;
use tokio::runtime::Handle;
use vm_validator::vm_validator::TransactionValidation;

/// Struct that owns all dependencies required by shared mempool routines.
#[derive(Clone)]
pub(crate) struct SharedMempool<V>
where
    V: TransactionValidation + 'static,
{
    pub mempool: Arc<Mutex<CoreMempool>>,
    pub config: MempoolConfig,
    pub(crate) network_interface: MempoolNetworkInterface,
    pub db: Arc<dyn DbReader>,
    pub validator: Arc<RwLock<V>>,
    pub subscribers: Vec<UnboundedSender<SharedMempoolNotification>>,
}

impl<V: TransactionValidation + 'static> SharedMempool<V> {
    pub fn new(
        mempool: Arc<Mutex<CoreMempool>>,
        config: MempoolConfig,
        network_senders: HashMap<NetworkId, MempoolNetworkSender>,
        db: Arc<dyn DbReader>,
        validator: Arc<RwLock<V>>,
        subscribers: Vec<UnboundedSender<SharedMempoolNotification>>,
        role: RoleType,
        peer_metadata_storage: Arc<PeerMetadataStorage>,
    ) -> Self {
        let network_interface = MempoolNetworkInterface::new(
            peer_metadata_storage,
            network_senders,
            role,
            config.clone(),
        );
        SharedMempool {
            mempool,
            config,
            network_interface,
            db,
            validator,
            subscribers,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SharedMempoolNotification {
    PeerStateChange,
    NewTransactions,
    ACK,
    Broadcast,
}

pub(crate) fn notify_subscribers(
    event: SharedMempoolNotification,
    subscribers: &[UnboundedSender<SharedMempoolNotification>],
) {
    for subscriber in subscribers {
        let _ = subscriber.unbounded_send(event);
    }
}

/// A future that represents a scheduled mempool txn broadcast
pub(crate) struct ScheduledBroadcast {
    /// Time of scheduled broadcast
    deadline: Instant,
    peer: PeerNetworkId,
    backoff: bool,
    waker: Arc<Mutex<Option<Waker>>>,
}

impl ScheduledBroadcast {
    pub fn new(deadline: Instant, peer: PeerNetworkId, backoff: bool, executor: Handle) -> Self {
        let waker: Arc<Mutex<Option<Waker>>> = Arc::new(Mutex::new(None));
        let waker_clone = waker.clone();

        if deadline > Instant::now() {
            let tokio_instant = tokio::time::Instant::from_std(deadline);
            executor.spawn(async move {
                tokio::time::sleep_until(tokio_instant).await;
                let mut waker = waker_clone.lock();
                if let Some(waker) = waker.take() {
                    waker.wake()
                }
            });
        }

        Self {
            deadline,
            peer,
            backoff,
            waker,
        }
    }
}

impl Future for ScheduledBroadcast {
    type Output = (PeerNetworkId, bool); // (peer, whether this broadcast was scheduled as a backoff broadcast)

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        if Instant::now() < self.deadline {
            let waker_clone = context.waker().clone();
            let mut waker = self.waker.lock();
            *waker = Some(waker_clone);

            Poll::Pending
        } else {
            Poll::Ready((self.peer, self.backoff))
        }
    }
}

/// Message sent from consensus to mempool.
pub enum ConsensusRequest {
    /// Request to pull block to submit to consensus.
    GetBlockRequest(
        // max block size
        u64,
        // transactions to exclude from the requested block
        Vec<TransactionSummary>,
        // callback to respond to
        oneshot::Sender<Result<ConsensusResponse>>,
    ),
    /// Notifications about *rejected* committed txns.
    RejectNotification(
        // rejected transactions from consensus
        Vec<TransactionSummary>,
        // callback to respond to
        oneshot::Sender<Result<ConsensusResponse>>,
    ),
}

impl fmt::Display for ConsensusRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let payload = match self {
            ConsensusRequest::GetBlockRequest(block_size, excluded_txns, _) => {
                let mut txns_str = "".to_string();
                for tx in excluded_txns.iter() {
                    txns_str += &format!("{} ", tx);
                }
                format!(
                    "GetBlockRequest [block_size: {}, excluded_txns: {}]",
                    block_size, txns_str
                )
            }
            ConsensusRequest::RejectNotification(rejected_txns, _) => {
                let mut txns_str = "".to_string();
                for tx in rejected_txns.iter() {
                    txns_str += &format!("{} ", tx);
                }
                format!("RejectNotification [rejected_txns: {}]", txns_str)
            }
        };
        write!(f, "{}", payload)
    }
}

/// Response sent from mempool to consensus.
pub enum ConsensusResponse {
    /// Block to submit to consensus
    GetBlockResponse(Vec<SignedTransaction>),
    CommitResponse(),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionSummary {
    pub sender: AccountAddress,
    pub sequence_number: u64,
}

impl fmt::Display for TransactionSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.sender, self.sequence_number,)
    }
}

pub type SubmissionStatus = (MempoolStatus, Option<DiscardedVMStatus>);

pub type SubmissionStatusBundle = (SignedTransaction, SubmissionStatus);

pub enum MempoolClientRequest {
    SubmitTransaction(SignedTransaction, oneshot::Sender<Result<SubmissionStatus>>),
    GetTransactionByHash(HashValue, oneshot::Sender<Option<SignedTransaction>>),
}

pub type MempoolClientSender = mpsc::Sender<MempoolClientRequest>;
pub type MempoolEventsReceiver = mpsc::Receiver<MempoolClientRequest>;

/// State of last sync with peer:
/// `timeline_id` is position in log of ready transactions
/// `is_alive` - is connection healthy
#[derive(Clone, Debug)]
pub(crate) struct PeerSyncState {
    pub timeline_id: u64,
    pub broadcast_info: BroadcastInfo,
    pub metadata: ConnectionMetadata,
}

impl PeerSyncState {
    pub fn new(metadata: ConnectionMetadata) -> Self {
        PeerSyncState {
            timeline_id: 0,
            broadcast_info: BroadcastInfo::new(),
            metadata,
        }
    }
}

/// Identifier for a broadcasted batch of txns.
/// For BatchId(`start_id`, `end_id`), (`start_id`, `end_id`) is the range of timeline IDs read from
/// the core mempool timeline index that produced the txns in this batch.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct BatchId(pub u64, pub u64);

impl PartialOrd for BatchId {
    fn partial_cmp(&self, other: &BatchId) -> Option<std::cmp::Ordering> {
        Some((other.0, other.1).cmp(&(self.0, self.1)))
    }
}

impl Ord for BatchId {
    fn cmp(&self, other: &BatchId) -> std::cmp::Ordering {
        (other.0, other.1).cmp(&(self.0, self.1))
    }
}

/// Txn broadcast-related info for a given remote peer.
#[derive(Clone, Debug)]
pub struct BroadcastInfo {
    // Sent broadcasts that have not yet received an ack.
    pub sent_batches: BTreeMap<BatchId, SystemTime>,
    // Broadcasts that have received a retry ack and are pending a resend.
    pub retry_batches: BTreeSet<BatchId>,
    // Whether broadcasting to this peer is in backoff mode, e.g. broadcasting at longer intervals.
    pub backoff_mode: bool,
}

impl BroadcastInfo {
    fn new() -> Self {
        Self {
            sent_batches: BTreeMap::new(),
            retry_batches: BTreeSet::new(),
            backoff_mode: false,
        }
    }
}
