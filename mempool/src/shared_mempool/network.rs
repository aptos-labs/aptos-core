// Copyright (c) The Diem Core Contributors
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
use async_trait::async_trait;
use channel::{diem_channel, message_queues::QueueStyle};
use diem_config::{
    config::{MempoolConfig, PeerRole, RoleType},
    network_id::{NetworkId, PeerNetworkId},
};
use diem_infallible::Mutex;
use diem_logger::prelude::*;
use diem_types::{transaction::SignedTransaction, PeerId};
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
use short_hex_str::AsShortHexStr;
use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashMap},
    ops::Add,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};
use vm_validator::vm_validator::TransactionValidation;

/// Container for exchanging transactions with other Mempools.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum MempoolSyncMsg {
    /// Broadcast request issued by the sender.
    BroadcastTransactionsRequest {
        /// Unique id of sync request. Can be used by sender for rebroadcast analysis
        request_id: Vec<u8>,
        transactions: Vec<SignedTransaction>,
    },
    /// Broadcast ack issued by the receiver.
    BroadcastTransactionsResponse {
        request_id: Vec<u8>,
        /// Retry signal from recipient if there are txns in corresponding broadcast
        /// that were rejected from mempool but may succeed on resend.
        retry: bool,
        /// A backpressure signal from the recipient when it is overwhelmed (e.g., mempool is full).
        backoff: bool,
    },
}

/// Protocol id for mempool direct-send calls.
pub const MEMPOOL_DIRECT_SEND_PROTOCOL: &[u8] = b"/diem/direct-send/0.1.0/mempool/0.1.0";

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

/// Create a new Sender that only sends for the `MEMPOOL_DIRECT_SEND_PROTOCOL` ProtocolId and a
/// Receiver (Events) that explicitly returns only said ProtocolId..
pub fn network_endpoint_config(max_broadcasts_per_peer: usize) -> AppConfig {
    AppConfig::p2p(
        [ProtocolId::MempoolDirectSend],
        diem_channel::Config::new(max_broadcasts_per_peer)
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
        unimplemented!()
    }
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
                if !peer_state.is_alive {
                    counters::active_upstream_peers(&peer.network_id()).inc();
                }
                peer_state.is_alive = true;
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
        // Validators can be restarted ata  later time
        // TODO: Determine why there's this optimization
        // TODO: What about garbage collection of validators
        if peer.network_id().is_validator_network() {
            if let Some(state) = self.sync_states.write_lock().get_mut(&peer) {
                counters::active_upstream_peers(&peer.network_id()).dec();
                state.is_alive = false;
            }
        } else {
            // All other nodes have their state immediately restarted anyways, so let's free them
            // TODO: Why is the Validator optimization not applied here
            if self.sync_states.write_lock().remove(&peer).is_some() {
                counters::active_upstream_peers(&peer.network_id()).dec();
            }
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
            let peer_states = self.sync_states.read_filtered(|(_, state)| state.is_alive);
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
            .sorted_by(|peer_a, peer_b| compare_prioritized_peers(peer_a, peer_b))
            .map(|(peer, _)| *peer)
            .collect();
        let _ = std::mem::replace(&mut *prioritized_peers, peers);
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
        request_id_bytes: Vec<u8>,
        retry: bool,
        backoff: bool,
        timestamp: SystemTime,
    ) {
        let batch_id = if let Ok(id) = bcs::from_bytes::<BatchId>(&request_id_bytes) {
            id
        } else {
            counters::invalid_ack_inc(&peer, counters::INVALID_REQUEST_ID);
            return;
        };

        let mut sync_states = self.sync_states.write_lock();

        let sync_state = if let Some(state) = sync_states.get_mut(&peer) {
            state
        } else {
            counters::invalid_ack_inc(&peer, counters::UNKNOWN_PEER);
            return;
        };

        if let Some(sent_timestamp) = sync_state.broadcast_info.sent_batches.remove(&batch_id) {
            let rtt = timestamp
                .duration_since(sent_timestamp)
                .expect("failed to calculate mempool broadcast RTT");

            let network_id = peer.network_id();
            let peer_id = peer.peer_id().short_str();
            counters::SHARED_MEMPOOL_BROADCAST_RTT
                .with_label_values(&[network_id.as_str(), peer_id.as_str()])
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

    pub fn execute_broadcast<V>(
        &self,
        peer: PeerNetworkId,
        scheduled_backoff: bool,
        smp: &mut SharedMempool<V>,
    ) where
        V: TransactionValidation,
    {
        // Start timer for tracking broadcast latency.
        let start_time = Instant::now();

        let mut sync_states = self.sync_states.write_lock();
        let state = if let Some(state) = sync_states.get_mut(&peer) {
            state
        } else {
            // If we don't have any info about the node, we shouldn't broadcast to it
            return;
        };

        // Only broadcast to peers that are alive.
        if !state.is_alive {
            return;
        }

        // When not a validator, only broadcast to `default_failovers`
        if !self.role.is_validator() {
            let priority = self
                .prioritized_peers
                .lock()
                .iter()
                .find_position(|peer_network_id| *peer_network_id == &peer)
                .map_or(usize::MAX, |(pos, _)| pos);
            if priority > self.mempool_config.default_failovers {
                return;
            }
        }

        // If backoff mode is on for this peer, only execute broadcasts that were scheduled as a backoff broadcast.
        // This is to ensure the backoff mode is actually honored (there is a chance a broadcast was scheduled
        // in non-backoff mode before backoff mode was turned on - ignore such scheduled broadcasts).
        if state.broadcast_info.backoff_mode && !scheduled_backoff {
            return;
        }

        let batch_id: BatchId;
        let transactions: Vec<SignedTransaction>;
        let mut metric_label = None;
        {
            let mut mempool = smp.mempool.lock();

            // Sync peer's pending broadcasts with latest mempool state.
            // A pending broadcast might become empty if the corresponding txns were committed through
            // another peer, so don't track broadcasts for committed txns.
            state.broadcast_info.sent_batches = state
                .broadcast_info
                .sent_batches
                .clone()
                .into_iter()
                .filter(|(id, _batch)| !mempool.timeline_range(id.0, id.1).is_empty())
                .collect::<BTreeMap<BatchId, SystemTime>>();

            // Check for batch to rebroadcast:
            // 1. Batch that did not receive ACK in configured window of time
            // 2. Batch that an earlier ACK marked as retriable
            let mut pending_broadcasts = 0;
            let mut expired = None;

            // Find earliest batch in timeline index that expired.
            // Note that state.broadcast_info.sent_batches is ordered in decreasing order in the timeline index
            for (batch, sent_time) in state.broadcast_info.sent_batches.iter() {
                let deadline = sent_time.add(Duration::from_millis(
                    self.mempool_config.shared_mempool_ack_timeout_ms,
                ));
                if SystemTime::now().duration_since(deadline).is_ok() {
                    expired = Some(batch);
                } else {
                    pending_broadcasts += 1;
                }

                // The maximum number of broadcasts sent to a single peer that are pending a response ACK at any point.
                // If the number of un-ACK'ed un-expired broadcasts reaches this threshold, we do not broadcast anymore
                // and wait until an ACK is received or a sent broadcast expires.
                // This helps rate-limit egress network bandwidth and not overload a remote peer or this
                // node's Diem network sender.
                if pending_broadcasts >= self.mempool_config.max_broadcasts_per_peer {
                    return;
                }
            }
            let retry = state.broadcast_info.retry_batches.iter().rev().next();

            let (new_batch_id, new_transactions) = match std::cmp::max(expired, retry) {
                Some(id) => {
                    metric_label = if Some(id) == expired {
                        Some(counters::EXPIRED_BROADCAST_LABEL)
                    } else {
                        Some(counters::RETRY_BROADCAST_LABEL)
                    };

                    let txns = mempool.timeline_range(id.0, id.1);
                    (*id, txns)
                }
                None => {
                    // Fresh broadcast
                    let (txns, new_timeline_id) = mempool.read_timeline(
                        state.timeline_id,
                        self.mempool_config.shared_mempool_batch_size,
                    );
                    (BatchId(state.timeline_id, new_timeline_id), txns)
                }
            };

            batch_id = new_batch_id;
            transactions = new_transactions;
        }

        if transactions.is_empty() {
            return;
        }

        let network_sender = smp.network_interface.sender();

        let num_txns = transactions.len();
        if let Err(e) = network_sender.send_to(
            peer,
            MempoolSyncMsg::BroadcastTransactionsRequest {
                request_id: bcs::to_bytes(&batch_id).expect("failed BCS serialization of batch ID"),
                transactions,
            },
        ) {
            counters::network_send_fail_inc(counters::BROADCAST_TXNS);
            error!(
                LogSchema::event_log(LogEntry::BroadcastTransaction, LogEvent::NetworkSendFail)
                    .peer(&peer)
                    .error(&e.into())
            );
            return;
        }
        // Update peer sync state with info from above broadcast.
        state.timeline_id = std::cmp::max(state.timeline_id, batch_id.1);
        // Turn off backoff mode after every broadcast.
        state.broadcast_info.backoff_mode = false;
        state
            .broadcast_info
            .sent_batches
            .insert(batch_id, SystemTime::now());
        state.broadcast_info.retry_batches.remove(&batch_id);
        notify_subscribers(SharedMempoolNotification::Broadcast, &smp.subscribers);

        let latency = start_time.elapsed();
        trace!(
            LogSchema::event_log(LogEntry::BroadcastTransaction, LogEvent::Success)
                .peer(&peer)
                .batch_id(&batch_id)
                .backpressure(scheduled_backoff)
        );
        let peer_id = peer.peer_id().short_str();
        let network_id = peer.network_id();
        counters::SHARED_MEMPOOL_TRANSACTION_BROADCAST_SIZE
            .with_label_values(&[network_id.as_str(), peer_id.as_str()])
            .observe(num_txns as f64);
        counters::shared_mempool_pending_broadcasts(&peer)
            .set(state.broadcast_info.sent_batches.len() as i64);
        counters::SHARED_MEMPOOL_BROADCAST_LATENCY
            .with_label_values(&[network_id.as_str(), peer_id.as_str()])
            .observe(latency.as_secs_f64());
        if let Some(label) = metric_label {
            counters::SHARED_MEMPOOL_BROADCAST_TYPE_COUNT
                .with_label_values(&[network_id.as_str(), peer_id.as_str(), label])
                .inc();
        }
        if scheduled_backoff {
            counters::SHARED_MEMPOOL_BROADCAST_TYPE_COUNT
                .with_label_values(&[
                    network_id.as_str(),
                    peer_id.as_str(),
                    counters::BACKPRESSURE_BROADCAST_LABEL,
                ])
                .inc();
        }
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

/// Provides ordering for peers to send transactions to
fn compare_prioritized_peers(
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
                // Then tiebreak by PeerId for stability
                Ordering::Equal => {
                    let peer_id_a = peer_network_id_a.peer_id();
                    let peer_id_b = peer_network_id_b.peer_id();
                    peer_id_a.cmp(&peer_id_b)
                }
                ordering => ordering,
            }
        }
        ordering => ordering,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use diem_config::network_id::NetworkId;
    use diem_types::PeerId;

    #[test]
    fn check_peer_prioritization() {
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
        assert_eq!(Ordering::Greater, compare_prioritized_peers(&vfn_1, &val_1));
        assert_eq!(Ordering::Less, compare_prioritized_peers(&val_1, &vfn_1));

        // PeerRole ordering
        assert_eq!(
            Ordering::Greater,
            compare_prioritized_peers(&vfn_1, &preferred_1)
        );
        assert_eq!(
            Ordering::Less,
            compare_prioritized_peers(&preferred_1, &vfn_1)
        );

        // Tiebreaker on peer_id
        assert_eq!(Ordering::Greater, compare_prioritized_peers(&val_2, &val_1));
        assert_eq!(Ordering::Less, compare_prioritized_peers(&val_1, &val_2));

        // Same the only equal case
        assert_eq!(Ordering::Equal, compare_prioritized_peers(&val_1, &val_1));
    }
}
