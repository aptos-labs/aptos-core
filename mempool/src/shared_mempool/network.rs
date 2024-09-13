// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Interface between Mempool and Network layers.

use crate::{
    counters,
    logging::{LogEntry, LogEvent, LogSchema},
    shared_mempool::{
        priority::PrioritizedPeersState,
        tasks,
        types::{
            notify_subscribers, MempoolMessageId, MempoolSenderBucket, PeerSyncState,
            SharedMempool, SharedMempoolNotification,
        },
    },
};
use aptos_config::{
    config::{MempoolConfig, NodeType},
    network_id::PeerNetworkId,
};
use aptos_infallible::RwLock;
use aptos_logger::prelude::*;
use aptos_netcore::transport::ConnectionOrigin;
use aptos_network::{
    application::{error::Error, interface::NetworkClientInterface, metadata::PeerMetadata},
    transport::ConnectionMetadata,
};
use aptos_time_service::TimeService;
use aptos_types::transaction::SignedTransaction;
use aptos_vm_validator::vm_validator::TransactionValidation;
use fail::fail_point;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt::Display,
    ops::Add,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant, SystemTime},
};
use thiserror::Error;

/// Container for exchanging transactions with other Mempools.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum MempoolSyncMsg {
    /// Broadcast request issued by the sender.
    BroadcastTransactionsRequest {
        /// Unique id of sync request. Can be used by sender for rebroadcast analysis
        message_id: MempoolMessageId,
        transactions: Vec<SignedTransaction>,
    },
    /// Broadcast ack issued by the receiver.
    BroadcastTransactionsResponse {
        message_id: MempoolMessageId,
        /// Retry signal from recipient if there are txns in corresponding broadcast
        /// that were rejected from mempool but may succeed on resend.
        retry: bool,
        /// A backpressure signal from the recipient when it is overwhelmed (e.g., mempool is full).
        backoff: bool,
    },
    /// Broadcast request issued by the sender.
    BroadcastTransactionsRequestWithReadyTime {
        /// Unique id of sync request. Can be used by sender for rebroadcast analysis
        message_id: MempoolMessageId,
        /// For each transaction, we also include the time at which the transaction is ready
        /// in the current node in millis since epoch. The upstream node can then calculate
        /// (SystemTime::now() - ready_time) to calculate the time it took for the transaction
        /// to reach the upstream node.
        transactions: Vec<(SignedTransaction, u64, BroadcastPeerPriority)>,
    },
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum BroadcastPeerPriority {
    Primary,
    Failover,
}

impl Display for BroadcastPeerPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BroadcastPeerPriority::Primary => write!(f, "Primary"),
            BroadcastPeerPriority::Failover => write!(f, "Failover"),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct MempoolNetworkInterface<NetworkClient> {
    network_client: NetworkClient,
    sync_states: Arc<RwLock<HashMap<PeerNetworkId, PeerSyncState>>>,
    node_type: NodeType,
    mempool_config: MempoolConfig,
    prioritized_peers_state: PrioritizedPeersState,
    pub num_mempool_txns_received_since_peers_updated: u64,
    pub num_committed_txns_received_since_peers_updated: Arc<AtomicU64>,
}

impl<NetworkClient: NetworkClientInterface<MempoolSyncMsg>> MempoolNetworkInterface<NetworkClient> {
    pub(crate) fn new(
        network_client: NetworkClient,
        node_type: NodeType,
        mempool_config: MempoolConfig,
    ) -> MempoolNetworkInterface<NetworkClient> {
        let prioritized_peers_state =
            PrioritizedPeersState::new(mempool_config.clone(), node_type, TimeService::real());
        Self {
            network_client,
            sync_states: Arc::new(RwLock::new(HashMap::new())),
            node_type,
            mempool_config,
            prioritized_peers_state,
            num_mempool_txns_received_since_peers_updated: 0,
            num_committed_txns_received_since_peers_updated: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Returns peers to add (with metadata) and peers to disable
    fn get_upstream_peers_to_add_and_disable(
        &self,
        updated_peers: &HashMap<PeerNetworkId, PeerMetadata>,
    ) -> (Vec<(PeerNetworkId, ConnectionMetadata)>, Vec<PeerNetworkId>) {
        let sync_states = self.sync_states.read();
        let to_disable: Vec<_> = sync_states
            .keys()
            .filter(|previous_peer| !updated_peers.contains_key(previous_peer))
            .copied()
            .collect();
        let to_add: Vec<_> = updated_peers
            .iter()
            .filter(|(peer, _)| !sync_states.contains_key(peer))
            .map(|(peer, metadata)| (*peer, metadata.get_connection_metadata()))
            .filter(|(peer, metadata)| self.is_upstream_peer(peer, Some(metadata)))
            .collect();
        (to_add, to_disable)
    }

    /// Returns newly added peers
    fn add_and_disable_upstream_peers(
        &self,
        to_add: &[(PeerNetworkId, ConnectionMetadata)],
        to_disable: &[PeerNetworkId],
    ) {
        // Return early if there are no updates
        if to_add.is_empty() && to_disable.is_empty() {
            return;
        }

        // Otherwise, update the sync states
        let mut sync_states = self.sync_states.write();
        for (peer, _) in to_add.iter().cloned() {
            counters::active_upstream_peers(&peer.network_id()).inc();
            sync_states.insert(
                peer,
                PeerSyncState::new(
                    self.mempool_config.broadcast_buckets.len(),
                    self.mempool_config.num_sender_buckets,
                ),
            );
        }
        for peer in to_disable {
            // All other nodes have their state immediately restarted anyways, so let's free them
            if sync_states.remove(peer).is_some() {
                counters::active_upstream_peers(&peer.network_id()).dec();
            }
        }
    }

    /// Update peers based on updated view of connected peers. Return (peers newly added that need
    /// to start broadcasts, peers that will be disabled from broadcasts).
    pub fn update_peers(
        &mut self,
        all_connected_peers: &HashMap<PeerNetworkId, PeerMetadata>,
    ) -> (Vec<PeerNetworkId>, Vec<PeerNetworkId>) {
        // Get the upstream peers to add or disable, using a read lock
        let (to_add, to_disable) = self.get_upstream_peers_to_add_and_disable(all_connected_peers);

        if !to_add.is_empty() || !to_disable.is_empty() {
            info!(
                "Mempool peers added: {:?}, Mempool peers disabled: {:?}",
                to_add.iter().map(|(peer, _)| peer).collect::<Vec<_>>(),
                to_disable
            );
        }

        // If there are updates, apply using a write lock
        self.add_and_disable_upstream_peers(&to_add, &to_disable);

        // Update the prioritized peers list using the prioritized peer comparator.
        // This should be called even if there are no changes to the peers, as the
        // peer metadata may have changed (e.g., ping latencies).
        let peers_changed = !to_add.is_empty() || !to_disable.is_empty();
        self.update_prioritized_peers(all_connected_peers, peers_changed);

        (to_add.iter().map(|(peer, _)| *peer).collect(), to_disable)
    }

    /// Updates the prioritized peers list
    fn update_prioritized_peers(
        &mut self,
        all_connected_peers: &HashMap<PeerNetworkId, PeerMetadata>,
        peers_changed: bool,
    ) {
        // Only fullnodes should prioritize peers (e.g., VFNs and PFNs)
        if self.node_type.is_validator() {
            return;
        }

        // If the prioritized peers list is not ready for an update, return early
        if !self.prioritized_peers_state.ready_for_update(peers_changed) {
            return;
        }

        // Fetch the peers and monitoring metadata
        let peer_network_ids: Vec<_> = self.sync_states.read().keys().cloned().collect();
        let peers_and_metadata: Vec<_> = peer_network_ids
            .iter()
            .map(|peer| {
                // Get the peer monitoring metadata for the peer
                let monitoring_metadata = all_connected_peers
                    .get(peer)
                    .map(|metadata| metadata.get_peer_monitoring_metadata());

                // Return the peer and monitoring metadata
                (*peer, monitoring_metadata)
            })
            .collect();

        // Update the prioritized peers list
        self.prioritized_peers_state.update_prioritized_peers(
            peers_and_metadata,
            self.num_mempool_txns_received_since_peers_updated,
            self.num_committed_txns_received_since_peers_updated
                .load(Ordering::Relaxed),
        );
        // Resetting the counter
        self.num_mempool_txns_received_since_peers_updated = 0;
        self.num_committed_txns_received_since_peers_updated
            .store(0, Ordering::SeqCst);
    }

    pub fn is_validator(&self) -> bool {
        self.node_type.is_validator()
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
            self.sync_states_exists(peer)
        }
    }

    pub fn process_broadcast_ack(
        &self,
        peer: PeerNetworkId,
        message_id: MempoolMessageId,
        retry: bool,
        backoff: bool,
        timestamp: SystemTime,
    ) {
        let mut sync_states = self.sync_states.write();

        let sync_state = if let Some(state) = sync_states.get_mut(&peer) {
            state
        } else {
            counters::invalid_ack_inc(peer.network_id(), counters::UNKNOWN_PEER);
            return;
        };

        if let Some(sent_timestamp) = sync_state.broadcast_info.sent_messages.remove(&message_id) {
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
                    .message_id(&message_id),
                "request ID does not exist or expired"
            );
            return;
        }

        trace!(
            LogSchema::new(LogEntry::ReceiveACK)
                .peer(&peer)
                .message_id(&message_id)
                .backpressure(backoff),
            retry = retry,
        );
        tasks::update_ack_counter(&peer, counters::RECEIVED_LABEL, retry, backoff);

        if retry {
            sync_state.broadcast_info.retry_messages.insert(message_id);
        }

        // Backoff mode can only be turned off by executing a broadcast that was scheduled
        // as a backoff broadcast.
        // This ensures backpressure request from remote peer is honored at least once.
        if backoff {
            sync_state.broadcast_info.backoff_mode = true;
        }
    }

    pub fn is_backoff_mode(&self, peer: &PeerNetworkId) -> bool {
        if let Some(state) = self.sync_states.write().get(peer) {
            state.broadcast_info.backoff_mode
        } else {
            // If we don't have sync state, we shouldn't backoff
            false
        }
    }

    /// Determines the broadcast batch.  There are three types of batches:
    /// * Expired -> This timed out waiting for a response and needs to be resent
    /// * Retry -> This received a response telling it to retry later
    /// * New -> There are no Expired or Retry broadcasts currently waiting
    fn determine_broadcast_batch<TransactionValidator: TransactionValidation>(
        &self,
        peer: PeerNetworkId,
        scheduled_backoff: bool,
        smp: &mut SharedMempool<NetworkClient, TransactionValidator>,
    ) -> Result<
        (
            MempoolMessageId,
            Vec<(SignedTransaction, u64, BroadcastPeerPriority)>,
            Option<&str>,
        ),
        BroadcastError,
    > {
        let mut sync_states = self.sync_states.write();
        // If we don't have any info about the node, we shouldn't broadcast to it
        let state = sync_states
            .get_mut(&peer)
            .ok_or(BroadcastError::PeerNotFound(peer))?;

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
        state.broadcast_info.sent_messages = state
            .broadcast_info
            .sent_messages
            .clone()
            .into_iter()
            .filter(|(message_id, _batch)| {
                !mempool
                    .timeline_range_of_message(message_id.decode())
                    .is_empty()
            })
            .collect::<BTreeMap<MempoolMessageId, SystemTime>>();
        state.broadcast_info.retry_messages = state
            .broadcast_info
            .retry_messages
            .clone()
            .into_iter()
            .filter(|message_id| {
                !mempool
                    .timeline_range_of_message(message_id.decode())
                    .is_empty()
            })
            .collect::<BTreeSet<MempoolMessageId>>();

        // Check for batch to rebroadcast:
        // 1. Batch that did not receive ACK in configured window of time
        // 2. Batch that an earlier ACK marked as retriable
        let mut pending_broadcasts = 0;
        let mut expired_message_id = None;

        // Find earliest message in timeline index that expired.
        // Note that state.broadcast_info.sent_messages is ordered in decreasing order in the timeline index
        for (message, sent_time) in state.broadcast_info.sent_messages.iter() {
            let deadline = sent_time.add(Duration::from_millis(
                self.mempool_config.shared_mempool_ack_timeout_ms,
            ));
            if SystemTime::now().duration_since(deadline).is_ok() {
                expired_message_id = Some(message);
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
        let retry_message_id = state.broadcast_info.retry_messages.iter().next_back();

        let (message_id, transactions, metric_label) =
            match std::cmp::max(expired_message_id, retry_message_id) {
                Some(message_id) => {
                    let metric_label = if Some(message_id) == expired_message_id {
                        Some(counters::EXPIRED_BROADCAST_LABEL)
                    } else {
                        Some(counters::RETRY_BROADCAST_LABEL)
                    };

                    let txns = message_id
                        .decode()
                        .into_iter()
                        .flat_map(|(sender_bucket, start_end_pairs)| {
                            if self.node_type.is_validator() {
                                mempool
                                    .timeline_range(sender_bucket, start_end_pairs)
                                    .into_iter()
                                    .map(|(txn, ready_time)| {
                                        (txn, ready_time, BroadcastPeerPriority::Primary)
                                    })
                                    .collect::<Vec<_>>()
                            } else {
                                self.prioritized_peers_state
                                    .get_sender_bucket_priority_for_peer(&peer, sender_bucket)
                                    .map_or_else(Vec::new, |priority| {
                                        mempool
                                            .timeline_range(sender_bucket, start_end_pairs)
                                            .into_iter()
                                            .map(|(txn, ready_time)| {
                                                (txn, ready_time, priority.clone())
                                            })
                                            .collect::<Vec<_>>()
                                    })
                            }
                        })
                        .collect::<Vec<_>>();
                    (message_id.clone(), txns, metric_label)
                },
                None => {
                    // Fresh broadcast

                    // If the peer doesn't have any sender_buckets assigned, let's not broadcast to the peer
                    let mut sender_buckets: Vec<(MempoolSenderBucket, BroadcastPeerPriority)> =
                        if self.node_type.is_validator() {
                            (0..self.mempool_config.num_sender_buckets)
                                .map(|sender_bucket| {
                                    (sender_bucket, BroadcastPeerPriority::Primary)
                                })
                                .collect()
                        } else {
                            self.prioritized_peers_state
                                .get_sender_buckets_for_peer(&peer)
                                .ok_or_else(|| {
                                    BroadcastError::PeerNotPrioritized(
                                        peer,
                                        self.prioritized_peers_state.get_peer_priority(&peer),
                                    )
                                })?
                                .clone()
                                .into_iter()
                                .collect()
                        };
                    // Sort sender_buckets based on priority. Primary peer should be first.
                    sender_buckets.sort_by(|(_, priority_a), (_, priority_b)| {
                        if priority_a == priority_b {
                            std::cmp::Ordering::Equal
                        } else if *priority_a == BroadcastPeerPriority::Primary {
                            std::cmp::Ordering::Less
                        } else {
                            std::cmp::Ordering::Greater
                        }
                    });

                    let max_txns = self.mempool_config.shared_mempool_batch_size;
                    let mut output_txns = vec![];
                    let mut output_updates = vec![];
                    for (sender_bucket, peer_priority) in sender_buckets {
                        let before = match peer_priority {
                            BroadcastPeerPriority::Primary => None,
                            BroadcastPeerPriority::Failover => Some(
                                Instant::now()
                                    - Duration::from_millis(
                                        self.mempool_config.shared_mempool_failover_delay_ms,
                                    ),
                            ),
                        };
                        if max_txns > 0 {
                            let old_timeline_id = state.timelines.get(&sender_bucket).unwrap();
                            let (txns, new_timeline_id) = mempool.read_timeline(
                                sender_bucket,
                                old_timeline_id,
                                max_txns,
                                before,
                                peer_priority.clone(),
                            );
                            output_txns.extend(
                                txns.into_iter()
                                    .map(|(txn, ready_time)| {
                                        (txn, ready_time, peer_priority.clone())
                                    })
                                    .collect::<Vec<_>>(),
                            );
                            output_updates
                                .push((sender_bucket, (old_timeline_id.clone(), new_timeline_id)));
                        }
                    }

                    (
                        MempoolMessageId::from_timeline_ids(output_updates),
                        output_txns,
                        None,
                    )
                },
            };

        if transactions.is_empty() {
            return Err(BroadcastError::NoTransactions(peer));
        }

        Ok((message_id, transactions, metric_label))
    }

    /// Sends a batch to the given peer
    async fn send_batch_to_peer(
        &self,
        peer: PeerNetworkId,
        message_id: MempoolMessageId,
        // For each transaction, we include the ready time in millis since epoch
        transactions: Vec<(SignedTransaction, u64, BroadcastPeerPriority)>,
    ) -> Result<(), BroadcastError> {
        let request = if self.mempool_config.include_ready_time_in_broadcast {
            MempoolSyncMsg::BroadcastTransactionsRequestWithReadyTime {
                message_id,
                transactions,
            }
        } else {
            MempoolSyncMsg::BroadcastTransactionsRequest {
                message_id,
                transactions: transactions.into_iter().map(|(txn, _, _)| txn).collect(),
            }
        };

        if let Err(e) = self.network_client.send_to_peer(request, peer) {
            counters::network_send_fail_inc(counters::BROADCAST_TXNS);
            return Err(BroadcastError::NetworkError(peer, e.into()));
        }
        Ok(())
    }

    /// Sends a message to the given peer
    pub fn send_message_to_peer(
        &self,
        peer: PeerNetworkId,
        message: MempoolSyncMsg,
    ) -> Result<(), Error> {
        fail_point!("mempool::send_to", |_| {
            Err(anyhow::anyhow!("Injected error in mempool::send_to").into())
        });
        self.network_client.send_to_peer(message, peer)
    }

    /// Updates the local tracker for a broadcast.  This is used to handle `DirectSend` tracking of
    /// responses
    fn update_broadcast_state(
        &self,
        peer: PeerNetworkId,
        message_id: MempoolMessageId,
        send_time: SystemTime,
    ) -> Result<usize, BroadcastError> {
        let mut sync_states = self.sync_states.write();
        let state = sync_states
            .get_mut(&peer)
            .ok_or(BroadcastError::PeerNotFound(peer))?;

        // Update peer sync state with info from above broadcast.
        state.update(&message_id);
        // Turn off backoff mode after every broadcast.
        state.broadcast_info.backoff_mode = false;
        state.broadcast_info.retry_messages.remove(&message_id);
        state
            .broadcast_info
            .sent_messages
            .insert(message_id, send_time);
        Ok(state.broadcast_info.sent_messages.len())
    }

    pub async fn execute_broadcast<TransactionValidator: TransactionValidation>(
        &self,
        peer: PeerNetworkId,
        scheduled_backoff: bool,
        smp: &mut SharedMempool<NetworkClient, TransactionValidator>,
    ) -> Result<(), BroadcastError> {
        // Start timer for tracking broadcast latency.
        let start_time = Instant::now();
        let (message_id, transactions, metric_label) =
            self.determine_broadcast_batch(peer, scheduled_backoff, smp)?;
        let num_txns = transactions.len();
        let send_time = SystemTime::now();
        self.send_batch_to_peer(peer, message_id.clone(), transactions)
            .await?;
        let num_pending_broadcasts =
            self.update_broadcast_state(peer, message_id.clone(), send_time)?;
        notify_subscribers(SharedMempoolNotification::Broadcast, &smp.subscribers);

        // Log all the metrics
        let latency = start_time.elapsed();
        trace!(
            LogSchema::event_log(LogEntry::BroadcastTransaction, LogEvent::Success)
                .peer(&peer)
                .message_id(&message_id)
                .backpressure(scheduled_backoff)
                .num_txns(num_txns)
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

    pub fn sync_states_exists(&self, peer: &PeerNetworkId) -> bool {
        self.sync_states.read().get(peer).is_some()
    }
}
