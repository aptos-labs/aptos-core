// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Interface between Mempool and Network layers.

use crate::{
    counters,
    logging::{LogEntry, LogEvent, LogSchema},
    shared_mempool::{
        tasks,
        types::{
            notify_subscribers, BatchId, PeerSyncState, SharedMempool, SharedMempoolNotification,
        },
    },
};
use aptos_config::{
    config::{MempoolConfig, PeerRole, RoleType},
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_types::{transaction::SignedTransaction, PeerId};
use async_trait::async_trait;
use channel::{aptos_channel, message_queues::QueueStyle};
use fail::fail_point;
use itertools::Itertools;
use netcore::transport::ConnectionOrigin;
use network::{
    application::{
        interface::{MultiNetworkSender, NetworkInterface},
        storage::{LockingHashMap, PeerMetadataStorage},
    },
    error::NetworkError,
    peer_manager::{ConnectionRequestSender, PeerManagerRequestSender},
    protocols::network::{
        AppConfig, ApplicationNetworkSender, NetworkEvents, NetworkSender, NewNetworkSender,
        RpcError,
    },
    transport::ConnectionMetadata,
    ProtocolId,
};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::RandomState;
use std::collections::BTreeSet;
use std::hash::BuildHasher;
use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashMap},
    hash::Hasher,
    ops::Add,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};
use thiserror::Error;
use vm_validator::vm_validator::TransactionValidation;

/// Container for exchanging transactions with other Mempools.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum MempoolSyncMsg {
    /// Broadcast request issued by the sender.
    BroadcastTransactionsRequest {
        /// Unique id of sync request. Can be used by sender for rebroadcast analysis
        request_id: BatchId,
        transactions: Vec<SignedTransaction>,
    },
    /// Broadcast ack issued by the receiver.
    BroadcastTransactionsResponse {
        request_id: BatchId,
        /// Retry signal from recipient if there are txns in corresponding broadcast
        /// that were rejected from mempool but may succeed on resend.
        retry: bool,
        /// A backpressure signal from the recipient when it is overwhelmed (e.g., mempool is full).
        backoff: bool,
    },
}

/// The interface from Network to Mempool layer.
///
/// `MempoolNetworkEvents` is a `Stream` of `PeerManagerNotification` where the
/// raw `Bytes` direct-send and rpc messages are deserialized into
/// `MempoolMessage` types. `MempoolNetworkEvents` is a thin wrapper around an
/// `channel::Receiver<PeerManagerNotification>`.
pub type MempoolNetworkEvents = NetworkEvents<MempoolSyncMsg>;

/// The interface from Mempool to Networking layer.
///
/// This is a thin wrapper around a `NetworkSender<MempoolSyncMsg>`, so it is
/// easy to clone and send off to a separate task. For example, the rpc requests
/// return Futures that encapsulate the whole flow, from sending the request to
/// remote, to finally receiving the response and deserializing. It therefore
/// makes the most sense to make the rpc call on a separate async task, which
/// requires the `MempoolNetworkSender` to be `Clone` and `Send`.
#[derive(Clone, Debug)]
pub struct MempoolNetworkSender {
    inner: NetworkSender<MempoolSyncMsg>,
}

pub fn network_endpoint_config(max_broadcasts_per_peer: usize) -> AppConfig {
    AppConfig::p2p(
        [ProtocolId::MempoolDirectSend],
        aptos_channel::Config::new(max_broadcasts_per_peer)
            .queue_style(QueueStyle::KLAST)
            .counters(&counters::PENDING_MEMPOOL_NETWORK_EVENTS),
    )
}

impl NewNetworkSender for MempoolNetworkSender {
    fn new(
        peer_mgr_reqs_tx: PeerManagerRequestSender,
        connection_reqs_tx: ConnectionRequestSender,
    ) -> Self {
        Self {
            inner: NetworkSender::new(peer_mgr_reqs_tx, connection_reqs_tx),
        }
    }
}

#[async_trait]
impl ApplicationNetworkSender<MempoolSyncMsg> for MempoolNetworkSender {
    fn send_to(&self, recipient: PeerId, message: MempoolSyncMsg) -> Result<(), NetworkError> {
        fail_point!("mempool::send_to", |_| {
            Err(anyhow::anyhow!("Injected error in mempool::send_to").into())
        });
        let protocol = ProtocolId::MempoolDirectSend;
        self.inner.send_to(recipient, protocol, message)
    }

    async fn send_rpc(
        &self,
        _recipient: PeerId,
        _req_msg: MempoolSyncMsg,
        _timeout: Duration,
    ) -> Result<MempoolSyncMsg, RpcError> {
        unimplemented!("Shared mempool only supports direct send messages!");
    }
}

#[derive(Debug, Error)]
pub enum BroadcastError {
    #[error("Peer {0} NetworkError: '{1}'")]
    NetworkError(PeerNetworkId, anyhow::Error),
    #[error("Peer {0} has no transactions to broadcast")]
    NoTransactions(PeerNetworkId),
    #[error("Peer {0} not found")]
    PeerNotFound(PeerNetworkId),
    #[error("Peer {0} not prioritized, priority: {1}")]
    PeerNotPrioritized(PeerNetworkId, usize),
    #[error("Peer {0} not scheduled for backoff")]
    PeerNotScheduled(PeerNetworkId),
    #[error("Peer {0} is over the limit for pending broadcasts")]
    TooManyPendingBroadcasts(PeerNetworkId),
}

type MempoolMultiNetworkSender = MultiNetworkSender<MempoolSyncMsg, MempoolNetworkSender>;

#[derive(Clone, Debug)]
pub(crate) struct MempoolNetworkInterface {
    peer_metadata_storage: Arc<PeerMetadataStorage>,
    sender: MempoolMultiNetworkSender,
    sync_states: Arc<LockingHashMap<PeerNetworkId, PeerSyncState>>,
    prioritized_peers: Arc<Mutex<Vec<PeerNetworkId>>>,
    role: RoleType,
    mempool_config: MempoolConfig,
    prioritized_peers_comparator: PrioritizedPeersComparator,
}

impl MempoolNetworkInterface {
    pub(crate) fn new(
        peer_metadata_storage: Arc<PeerMetadataStorage>,
        network_senders: HashMap<NetworkId, MempoolNetworkSender>,
        role: RoleType,
        mempool_config: MempoolConfig,
    ) -> MempoolNetworkInterface {
        MempoolNetworkInterface {
            peer_metadata_storage,
            sender: MultiNetworkSender::new(network_senders),
            sync_states: Arc::new(LockingHashMap::new()),
            prioritized_peers: Arc::new(Mutex::new(Vec::new())),
            role,
            mempool_config,
            prioritized_peers_comparator: PrioritizedPeersComparator::new(),
        }
    }

    /// Add a peer to sync states, and returns `false` if the peer already is in storage
    pub fn add_peer(&self, peer: PeerNetworkId, metadata: ConnectionMetadata) -> bool {
        let mut sync_states = self.sync_states.write_lock();
        let is_new_peer = !sync_states.contains_key(&peer);
        if self.is_upstream_peer(&peer, Some(&metadata)) {
            // If we have a new peer, let's insert new data, otherwise, let's just update the current state
            if is_new_peer {
                counters::active_upstream_peers(&peer.network_id()).inc();
                sync_states.insert(peer, PeerSyncState::new(metadata));
            } else if let Some(peer_state) = sync_states.get_mut(&peer) {
                peer_state.metadata = metadata;
            }
        }
        drop(sync_states);

        // Always need to update the prioritized peers, because of `is_alive` state changes
        self.update_prioritized_peers();
        is_new_peer
    }

    /// Disables a peer if it can be restarted, otherwise removes it
    pub fn disable_peer(&self, peer: PeerNetworkId) {
        // All other nodes have their state immediately restarted anyways, so let's free them
        if self.sync_states.write_lock().remove(&peer).is_some() {
            counters::active_upstream_peers(&peer.network_id()).dec();
        }

        // Always update prioritized peers to be in line with peer states
        self.update_prioritized_peers();
    }

    fn update_prioritized_peers(&self) {
        // Only do this if it's not a validator
        if self.role.is_validator() {
            return;
        }

        // Retrieve just what's needed for the peer ordering
        let peers: Vec<_> = {
            let peer_states = self.sync_states.read_all();
            peer_states
                .iter()
                .map(|(peer, state)| (*peer, state.metadata.role))
                .collect()
        };

        // Order peers by network and by type
        // Origin doesn't matter at this point, only inserted ones into peer_states are upstream
        // Validators will always have the full set
        let mut prioritized_peers = self.prioritized_peers.lock();
        let peers: Vec<_> = peers
            .iter()
            .sorted_by(|peer_a, peer_b| self.prioritized_peers_comparator.compare(peer_a, peer_b))
            .map(|(peer, _)| *peer)
            .collect();
        let _ = std::mem::replace(&mut *prioritized_peers, peers);
    }

    pub fn is_validator(&self) -> bool {
        self.role.is_validator()
    }

    pub fn is_upstream_peer(
        &self,
        peer: &PeerNetworkId,
        metadata: Option<&ConnectionMetadata>,
    ) -> bool {
        // P2P networks have everyone be upstream
        if peer.network_id().is_validator_network() {
            return true;
        }

        // Outbound connections are upstream on non-P2P networks
        if let Some(metadata) = metadata {
            metadata.origin == ConnectionOrigin::Outbound
        } else {
            self.sync_states.read(peer).is_some()
        }
    }

    pub fn process_broadcast_ack(
        &self,
        peer: PeerNetworkId,
        batch_id: BatchId,
        retry: bool,
        backoff: bool,
        timestamp: SystemTime,
    ) {
        let mut sync_states = self.sync_states.write_lock();

        let sync_state = if let Some(state) = sync_states.get_mut(&peer) {
            state
        } else {
            counters::invalid_ack_inc(peer.network_id(), counters::UNKNOWN_PEER);
            return;
        };

        if let Some(sent_timestamp) = sync_state.broadcast_info.sent_batches.remove(&batch_id) {
            let rtt = timestamp
                .duration_since(sent_timestamp)
                .expect("failed to calculate mempool broadcast RTT");

            let network_id = peer.network_id();
            counters::SHARED_MEMPOOL_BROADCAST_RTT
                .with_label_values(&[network_id.as_str()])
                .observe(rtt.as_secs_f64());

            counters::shared_mempool_pending_broadcasts(&peer).dec();
        } else {
            trace!(
                LogSchema::new(LogEntry::ReceiveACK)
                    .peer(&peer)
                    .batch_id(&batch_id),
                "batch ID does not exist or expired"
            );
            return;
        }

        trace!(
            LogSchema::new(LogEntry::ReceiveACK)
                .peer(&peer)
                .batch_id(&batch_id)
                .backpressure(backoff),
            retry = retry,
        );
        tasks::update_ack_counter(&peer, counters::RECEIVED_LABEL, retry, backoff);

        if retry {
            sync_state.broadcast_info.retry_batches.insert(batch_id);
        }

        // Backoff mode can only be turned off by executing a broadcast that was scheduled
        // as a backoff broadcast.
        // This ensures backpressure request from remote peer is honored at least once.
        if backoff {
            sync_state.broadcast_info.backoff_mode = true;
        }
    }

    pub fn is_backoff_mode(&self, peer: &PeerNetworkId) -> bool {
        if let Some(state) = self.sync_states.write_lock().get(peer) {
            state.broadcast_info.backoff_mode
        } else {
            // If we don't have sync state, we shouldn't backoff
            false
        }
    }

    /// Peers are prioritized when the local is a validator, or it's within the default failovers.
    /// One is added for the primary peer
    fn check_peer_prioritized(&self, peer: PeerNetworkId) -> Result<(), BroadcastError> {
        if !self.role.is_validator() {
            let priority = self
                .prioritized_peers
                .lock()
                .iter()
                .find_position(|peer_network_id| *peer_network_id == &peer)
                .map_or(usize::MAX, |(pos, _)| pos);
            if priority > self.mempool_config.default_failovers {
                return Err(BroadcastError::PeerNotPrioritized(peer, priority));
            }
        }
        Ok(())
    }

    /// Determines the broadcast batch.  There are three types of batches:
    /// * Expired -> This timed out waiting for a response and needs to be resent
    /// * Retry -> This received a response telling it to retry later
    /// * New -> There are no Expired or Retry broadcasts currently waiting
    fn determine_broadcast_batch<V>(
        &self,
        peer: PeerNetworkId,
        scheduled_backoff: bool,
        smp: &mut SharedMempool<V>,
    ) -> Result<(BatchId, Vec<SignedTransaction>, Option<&str>), BroadcastError>
    where
        V: TransactionValidation,
    {
        let mut sync_states = self.sync_states.write_lock();
        // If we don't have any info about the node, we shouldn't broadcast to it
        let state = sync_states
            .get_mut(&peer)
            .ok_or(BroadcastError::PeerNotFound(peer))?;

        // If the peer isn't prioritized, lets not broadcast
        self.check_peer_prioritized(peer)?;

        // If backoff mode is on for this peer, only execute broadcasts that were scheduled as a backoff broadcast.
        // This is to ensure the backoff mode is actually honored (there is a chance a broadcast was scheduled
        // in non-backoff mode before backoff mode was turned on - ignore such scheduled broadcasts).
        if state.broadcast_info.backoff_mode && !scheduled_backoff {
            return Err(BroadcastError::PeerNotScheduled(peer));
        }

        // Sync peer's pending broadcasts with latest mempool state.
        // A pending or retry broadcast might become empty if the corresponding txns were committed through
        // another peer, so don't track broadcasts for committed txns.
        let mempool = smp.mempool.lock();
        state.broadcast_info.sent_batches = state
            .broadcast_info
            .sent_batches
            .clone()
            .into_iter()
            .filter(|(id, _batch)| !mempool.timeline_range(id.0, id.1).is_empty())
            .collect::<BTreeMap<BatchId, SystemTime>>();
        state.broadcast_info.retry_batches = state
            .broadcast_info
            .retry_batches
            .clone()
            .into_iter()
            .filter(|id| !mempool.timeline_range(id.0, id.1).is_empty())
            .collect::<BTreeSet<BatchId>>();

        // Check for batch to rebroadcast:
        // 1. Batch that did not receive ACK in configured window of time
        // 2. Batch that an earlier ACK marked as retriable
        let mut pending_broadcasts = 0;
        let mut expired_batch_id = None;

        // Find earliest batch in timeline index that expired.
        // Note that state.broadcast_info.sent_batches is ordered in decreasing order in the timeline index
        for (batch, sent_time) in state.broadcast_info.sent_batches.iter() {
            let deadline = sent_time.add(Duration::from_millis(
                self.mempool_config.shared_mempool_ack_timeout_ms,
            ));
            if SystemTime::now().duration_since(deadline).is_ok() {
                expired_batch_id = Some(batch);
            } else {
                pending_broadcasts += 1;
            }

            // The maximum number of broadcasts sent to a single peer that are pending a response ACK at any point.
            // If the number of un-ACK'ed un-expired broadcasts reaches this threshold, we do not broadcast anymore
            // and wait until an ACK is received or a sent broadcast expires.
            // This helps rate-limit egress network bandwidth and not overload a remote peer or this
            // node's network sender.
            if pending_broadcasts >= self.mempool_config.max_broadcasts_per_peer {
                return Err(BroadcastError::TooManyPendingBroadcasts(peer));
            }
        }
        let retry_batch_id = state.broadcast_info.retry_batches.iter().rev().next();

        let (batch_id, transactions, metric_label) =
            match std::cmp::max(expired_batch_id, retry_batch_id) {
                Some(id) => {
                    let metric_label = if Some(id) == expired_batch_id {
                        Some(counters::EXPIRED_BROADCAST_LABEL)
                    } else {
                        Some(counters::RETRY_BROADCAST_LABEL)
                    };

                    let txns = mempool.timeline_range(id.0, id.1);
                    (*id, txns, metric_label)
                }
                None => {
                    // Fresh broadcast
                    let (txns, new_timeline_id) = mempool.read_timeline(
                        state.timeline_id,
                        self.mempool_config.shared_mempool_batch_size,
                    );
                    (BatchId(state.timeline_id, new_timeline_id), txns, None)
                }
            };

        if transactions.is_empty() {
            return Err(BroadcastError::NoTransactions(peer));
        }

        Ok((batch_id, transactions, metric_label))
    }

    /// Sends a batch to the given `Peer`
    async fn send_batch(
        &self,
        peer: PeerNetworkId,
        batch_id: BatchId,
        transactions: Vec<SignedTransaction>,
    ) -> Result<(), BroadcastError> {
        let request = MempoolSyncMsg::BroadcastTransactionsRequest {
            request_id: batch_id,
            transactions,
        };

        if let Err(e) = self.sender.send_to(peer, request) {
            counters::network_send_fail_inc(counters::BROADCAST_TXNS);
            return Err(BroadcastError::NetworkError(peer, e.into()));
        }
        Ok(())
    }

    /// Updates the local tracker for a broadcast.  This is used to handle `DirectSend` tracking of
    /// responses
    fn update_broadcast_state(
        &self,
        peer: PeerNetworkId,
        batch_id: BatchId,
        send_time: SystemTime,
    ) -> Result<usize, BroadcastError> {
        let mut sync_states = self.sync_states.write_lock();
        let state = sync_states
            .get_mut(&peer)
            .ok_or(BroadcastError::PeerNotFound(peer))?;

        // Update peer sync state with info from above broadcast.
        state.timeline_id = std::cmp::max(state.timeline_id, batch_id.1);
        // Turn off backoff mode after every broadcast.
        state.broadcast_info.backoff_mode = false;
        state
            .broadcast_info
            .sent_batches
            .insert(batch_id, send_time);
        state.broadcast_info.retry_batches.remove(&batch_id);
        Ok(state.broadcast_info.sent_batches.len())
    }

    pub async fn execute_broadcast<V>(
        &self,
        peer: PeerNetworkId,
        scheduled_backoff: bool,
        smp: &mut SharedMempool<V>,
    ) -> Result<(), BroadcastError>
    where
        V: TransactionValidation,
    {
        // Start timer for tracking broadcast latency.
        let start_time = Instant::now();
        let (batch_id, transactions, metric_label) =
            self.determine_broadcast_batch(peer, scheduled_backoff, smp)?;

        let num_txns = transactions.len();
        let send_time = SystemTime::now();
        self.send_batch(peer, batch_id, transactions).await?;
        let num_pending_broadcasts = self.update_broadcast_state(peer, batch_id, send_time)?;
        notify_subscribers(SharedMempoolNotification::Broadcast, &smp.subscribers);

        // Log all the metrics
        let latency = start_time.elapsed();
        trace!(
            LogSchema::event_log(LogEntry::BroadcastTransaction, LogEvent::Success)
                .peer(&peer)
                .batch_id(&batch_id)
                .backpressure(scheduled_backoff)
        );
        let network_id = peer.network_id();
        counters::shared_mempool_broadcast_size(network_id, num_txns);
        // TODO: Rethink if this metric is useful
        counters::shared_mempool_pending_broadcasts(&peer).set(num_pending_broadcasts as i64);
        counters::shared_mempool_broadcast_latency(network_id, latency);
        if let Some(label) = metric_label {
            counters::shared_mempool_broadcast_type_inc(network_id, label);
        }
        if scheduled_backoff {
            counters::shared_mempool_broadcast_type_inc(
                network_id,
                counters::BACKPRESSURE_BROADCAST_LABEL,
            );
        }
        Ok(())
    }
}

impl NetworkInterface<MempoolSyncMsg, MempoolMultiNetworkSender> for MempoolNetworkInterface {
    type AppDataKey = PeerNetworkId;
    type AppData = PeerSyncState;

    fn peer_metadata_storage(&self) -> &PeerMetadataStorage {
        &self.peer_metadata_storage
    }

    fn sender(&self) -> MempoolMultiNetworkSender {
        self.sender.clone()
    }

    fn app_data(&self) -> &LockingHashMap<PeerNetworkId, PeerSyncState> {
        &self.sync_states
    }
}

#[derive(Clone, Debug)]
struct PrioritizedPeersComparator {
    random_state: RandomState,
}

impl PrioritizedPeersComparator {
    fn new() -> Self {
        Self {
            random_state: RandomState::new(),
        }
    }

    /// Provides ordering for peers to send transactions to
    fn compare(
        &self,
        peer_a: &(PeerNetworkId, PeerRole),
        peer_b: &(PeerNetworkId, PeerRole),
    ) -> Ordering {
        let peer_network_id_a = peer_a.0;
        let peer_network_id_b = peer_b.0;

        // Sort by NetworkId
        match peer_network_id_a
            .network_id()
            .cmp(&peer_network_id_b.network_id())
        {
            Ordering::Equal => {
                // Then sort by Role
                let role_a = peer_a.1;
                let role_b = peer_b.1;
                match role_a.cmp(&role_b) {
                    // Tiebreak by hash_peer_id.
                    Ordering::Equal => {
                        let hash_a = self.hash_peer_id(&peer_network_id_a.peer_id());
                        let hash_b = self.hash_peer_id(&peer_network_id_b.peer_id());

                        hash_a.cmp(&hash_b)
                    }
                    ordering => ordering,
                }
            }
            ordering => ordering,
        }
    }

    /// Stable within a mempool instance but random between instances.
    fn hash_peer_id(&self, peer_id: &PeerId) -> u64 {
        let mut hasher = self.random_state.build_hasher();
        hasher.write(peer_id.as_ref());
        hasher.finish()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_config::network_id::NetworkId;
    use aptos_types::PeerId;

    #[test]
    fn check_peer_prioritization() {
        let comparator = PrioritizedPeersComparator::new();

        let peer_id_1 = PeerId::from_hex_literal("0x1").unwrap();
        let peer_id_2 = PeerId::from_hex_literal("0x2").unwrap();
        let val_1 = (
            PeerNetworkId::new(NetworkId::Vfn, peer_id_1),
            PeerRole::Validator,
        );
        let val_2 = (
            PeerNetworkId::new(NetworkId::Vfn, peer_id_2),
            PeerRole::Validator,
        );
        let vfn_1 = (
            PeerNetworkId::new(NetworkId::Public, peer_id_1),
            PeerRole::ValidatorFullNode,
        );
        let preferred_1 = (
            PeerNetworkId::new(NetworkId::Public, peer_id_1),
            PeerRole::PreferredUpstream,
        );

        // NetworkId ordering
        assert_eq!(Ordering::Greater, comparator.compare(&vfn_1, &val_1));
        assert_eq!(Ordering::Less, comparator.compare(&val_1, &vfn_1));

        // PeerRole ordering
        assert_eq!(Ordering::Greater, comparator.compare(&vfn_1, &preferred_1));
        assert_eq!(Ordering::Less, comparator.compare(&preferred_1, &vfn_1));

        // Tiebreaker on peer_id
        let hash_1 = comparator.hash_peer_id(&val_1.0.peer_id());
        let hash_2 = comparator.hash_peer_id(&val_2.0.peer_id());

        assert_eq!(hash_2.cmp(&hash_1), comparator.compare(&val_2, &val_1));
        assert_eq!(hash_1.cmp(&hash_2), comparator.compare(&val_1, &val_2));

        // Same the only equal case
        assert_eq!(Ordering::Equal, comparator.compare(&val_1, &val_1));
    }
}
