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
            notify_subscribers, MultiBatchId, PeerSyncState, SharedMempool,
            SharedMempoolNotification,
        },
    },
};
use aptos_config::{
    config::{MempoolConfig, RoleType},
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
    ops::Add,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};
use thiserror::Error;

/// Container for exchanging transactions with other Mempools.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum MempoolSyncMsg {
    /// Broadcast request issued by the sender.
    BroadcastTransactionsRequest {
        /// Unique id of sync request. Can be used by sender for rebroadcast analysis
        request_id: MultiBatchId,
        transactions: Vec<SignedTransaction>,
    },
    /// Broadcast ack issued by the receiver.
    BroadcastTransactionsResponse {
        request_id: MultiBatchId,
        /// Retry signal from recipient if there are txns in corresponding broadcast
        /// that were rejected from mempool but may succeed on resend.
        retry: bool,
        /// A backpressure signal from the recipient when it is overwhelmed (e.g., mempool is full).
        backoff: bool,
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

pub enum BroadcastPeerPriority {
    Primary,
    Failover,
}

#[derive(Clone, Debug)]
pub(crate) struct MempoolNetworkInterface<NetworkClient> {
    network_client: NetworkClient,
    sync_states: Arc<RwLock<HashMap<PeerNetworkId, PeerSyncState>>>,
    role: RoleType,
    mempool_config: MempoolConfig,
    prioritized_peers_state: PrioritizedPeersState,
}

impl<NetworkClient: NetworkClientInterface<MempoolSyncMsg>> MempoolNetworkInterface<NetworkClient> {
    pub(crate) fn new(
        network_client: NetworkClient,
        role: RoleType,
        mempool_config: MempoolConfig,
    ) -> MempoolNetworkInterface<NetworkClient> {
        let prioritized_peers_state =
            PrioritizedPeersState::new(mempool_config.clone(), TimeService::real());
        Self {
            network_client,
            sync_states: Arc::new(RwLock::new(HashMap::new())),
            role,
            mempool_config,
            prioritized_peers_state,
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
                PeerSyncState::new(self.mempool_config.broadcast_buckets.len()),
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
        if self.role.is_validator() {
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
        self.prioritized_peers_state
            .update_prioritized_peers(peers_and_metadata);
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
            self.sync_states_exists(peer)
        }
    }

    pub fn process_broadcast_ack(
        &self,
        peer: PeerNetworkId,
        batch_id: MultiBatchId,
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
        if let Some(state) = self.sync_states.write().get(peer) {
            state.broadcast_info.backoff_mode
        } else {
            // If we don't have sync state, we shouldn't backoff
            false
        }
    }

    /// Peers are prioritized when the local is a validator, or it's within the default failovers.
    /// One is added for the primary peer
    fn check_peer_prioritized(
        &self,
        peer: PeerNetworkId,
    ) -> Result<BroadcastPeerPriority, BroadcastError> {
        if self.role.is_validator() {
            return Ok(BroadcastPeerPriority::Primary);
        }

        let peer_priority = self.prioritized_peers_state.get_peer_priority(&peer);
        if peer_priority == 0 {
            Ok(BroadcastPeerPriority::Primary)
        } else if peer_priority <= self.mempool_config.default_failovers {
            Ok(BroadcastPeerPriority::Failover)
        } else {
            Err(BroadcastError::PeerNotPrioritized(peer, peer_priority))
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
    ) -> Result<(MultiBatchId, Vec<SignedTransaction>, Option<&str>), BroadcastError> {
        let mut sync_states = self.sync_states.write();
        // If we don't have any info about the node, we shouldn't broadcast to it
        let state = sync_states
            .get_mut(&peer)
            .ok_or(BroadcastError::PeerNotFound(peer))?;

        // If the peer isn't prioritized, lets not broadcast
        let peer_priority = self.check_peer_prioritized(peer)?;

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
            .filter(|(id, _batch)| !mempool.timeline_range(&id.0).is_empty())
            .collect::<BTreeMap<MultiBatchId, SystemTime>>();
        state.broadcast_info.retry_batches = state
            .broadcast_info
            .retry_batches
            .clone()
            .into_iter()
            .filter(|id| !mempool.timeline_range(&id.0).is_empty())
            .collect::<BTreeSet<MultiBatchId>>();

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
        let retry_batch_id = state.broadcast_info.retry_batches.iter().next_back();

        let (batch_id, transactions, metric_label) =
            match std::cmp::max(expired_batch_id, retry_batch_id) {
                Some(id) => {
                    let metric_label = if Some(id) == expired_batch_id {
                        Some(counters::EXPIRED_BROADCAST_LABEL)
                    } else {
                        Some(counters::RETRY_BROADCAST_LABEL)
                    };

                    let txns = mempool.timeline_range(&id.0);
                    (id.clone(), txns, metric_label)
                },
                None => {
                    // Fresh broadcast
                    let before = match peer_priority {
                        BroadcastPeerPriority::Primary => None,
                        BroadcastPeerPriority::Failover => Some(
                            Instant::now()
                                - Duration::from_millis(
                                    self.mempool_config.shared_mempool_failover_delay_ms,
                                ),
                        ),
                    };
                    let (txns, new_timeline_id) = mempool.read_timeline(
                        &state.timeline_id,
                        self.mempool_config.shared_mempool_batch_size,
                        before,
                    );
                    (
                        MultiBatchId::from_timeline_ids(&state.timeline_id, &new_timeline_id),
                        txns,
                        None,
                    )
                },
            };

        if transactions.is_empty() {
            return Err(BroadcastError::NoTransactions(peer));
        }

        Ok((batch_id, transactions, metric_label))
    }

    /// Sends a batch to the given peer
    async fn send_batch_to_peer(
        &self,
        peer: PeerNetworkId,
        batch_id: MultiBatchId,
        transactions: Vec<SignedTransaction>,
    ) -> Result<(), BroadcastError> {
        let request = MempoolSyncMsg::BroadcastTransactionsRequest {
            request_id: batch_id,
            transactions,
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
        batch_id: MultiBatchId,
        send_time: SystemTime,
    ) -> Result<usize, BroadcastError> {
        let mut sync_states = self.sync_states.write();
        let state = sync_states
            .get_mut(&peer)
            .ok_or(BroadcastError::PeerNotFound(peer))?;

        // Update peer sync state with info from above broadcast.
        state.timeline_id.update(&batch_id);
        // Turn off backoff mode after every broadcast.
        state.broadcast_info.backoff_mode = false;
        state.broadcast_info.retry_batches.remove(&batch_id);
        state
            .broadcast_info
            .sent_batches
            .insert(batch_id, send_time);
        Ok(state.broadcast_info.sent_batches.len())
    }

    pub async fn execute_broadcast<TransactionValidator: TransactionValidation>(
        &self,
        peer: PeerNetworkId,
        scheduled_backoff: bool,
        smp: &mut SharedMempool<NetworkClient, TransactionValidator>,
    ) -> Result<(), BroadcastError> {
        // Start timer for tracking broadcast latency.
        let start_time = Instant::now();
        let (batch_id, transactions, metric_label) =
            self.determine_broadcast_batch(peer, scheduled_backoff, smp)?;

        let num_txns = transactions.len();
        let send_time = SystemTime::now();
        self.send_batch_to_peer(peer, batch_id.clone(), transactions)
            .await?;
        let num_pending_broadcasts =
            self.update_broadcast_state(peer, batch_id.clone(), send_time)?;
        notify_subscribers(SharedMempoolNotification::Broadcast, &smp.subscribers);

        // Log all the metrics
        let latency = start_time.elapsed();
        trace!(
            LogSchema::event_log(LogEntry::BroadcastTransaction, LogEvent::Success)
                .peer(&peer)
                .batch_id(&batch_id)
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
