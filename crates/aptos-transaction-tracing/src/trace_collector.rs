// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::trace::{
    PartialTrace, TransactionDestination, TransactionExpirationReason, TransactionSource,
    TransactionTrace,
};
use aptos_config::{
    config::TransactionTracingConfig,
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_consensus_types::common::RejectedTransactionSummary;
use aptos_crypto::HashValue;
use aptos_logger::{info, warn};
use aptos_mempool_notifications::CommittedTransaction;
use aptos_types::{transaction::SignedTransaction, PeerId};
use dashmap::DashMap;
use std::{collections::BTreeMap, sync::Arc};

/// A collector for transaction traces
#[derive(Clone)]
pub struct TransactionTraceCollector {
    // The identities of this trace collector (i.e., this node) across different networks
    collector_identities: Arc<BTreeMap<NetworkId, PeerId>>,

    // The configuration for transaction tracing
    transaction_tracing_config: Arc<TransactionTracingConfig>,

    // A mapping from transaction hashes to transaction traces.
    // Traces are created locally (e.g., in mempool), as well as
    // collected from other peers.
    transaction_traces: DashMap<HashValue, TransactionTrace>,
}

impl TransactionTraceCollector {
    pub fn new(
        collector_identities: BTreeMap<NetworkId, PeerId>,
        transaction_tracing_config: TransactionTracingConfig,
    ) -> Self {
        Self {
            collector_identities: Arc::new(collector_identities),
            transaction_tracing_config: Arc::new(transaction_tracing_config),
            transaction_traces: DashMap::new(),
        }
    }

    /// Creates a new (empty) transaction trace collector
    pub fn new_empty() -> Self {
        Self {
            collector_identities: Arc::new(BTreeMap::new()),
            transaction_tracing_config: Arc::new(TransactionTracingConfig::default()),
            transaction_traces: DashMap::new(),
        }
    }

    /// Adds a new transaction trace received from the specified peer
    pub fn add_transaction_trace_from_peer(
        &self,
        peer_network_id: &PeerNetworkId,
        transaction_hash: HashValue,
        transaction_trace: TransactionTrace,
    ) {
        if let Some(mut existing_transaction_trace_entry) =
            self.transaction_traces.get_mut(&transaction_hash)
        {
            // Check that we forwarded the transaction to the peer that sent us the trace
            let existing_transaction_trace = existing_transaction_trace_entry.value_mut();
            let transaction_forwarded_to = &existing_transaction_trace
                .get_authored_partial_trace()
                .transaction_forwarded_to();
            if let Some(TransactionDestination::RemotePeer(remote_peer_network_id)) =
                transaction_forwarded_to
            {
                if peer_network_id == remote_peer_network_id {
                    info!("Merging transaction trace for transaction hash {:?} received from peer {:?}.",
                        transaction_hash, peer_network_id);

                    // Merge the received trace into the existing transaction trace
                    existing_transaction_trace
                        .merge_peer_trace(*peer_network_id, transaction_trace);
                } else {
                    warn!(
                        "Received trace for transaction hash {:?} from peer {:?}, \
                            but we forwarded the transaction to another peer {:?}. Ignoring trace!",
                        transaction_hash, peer_network_id, remote_peer_network_id
                    );
                }
            } else {
                warn!(
                    "Received trace for transaction hash {:?} from peer {:?}, \
                        but we did not forward the transaction to any peer. Ignoring trace!",
                    transaction_hash, peer_network_id
                );
            }
        } else {
            warn!(
                "Received trace for transaction hash {:?} from peer {:?}, \
                but no trace exists for that transaction!",
                transaction_hash, peer_network_id
            );
        }
    }

    /// Retrieves all complete, expired or garbage collected transaction traces
    /// (and removes them from the collector). The returned tuple contains three
    /// maps: (complete_traces, expired_traces, garbage_collected_traces).
    pub fn get_complete_expired_and_garbage_collected_traces(
        &self,
    ) -> (
        DashMap<HashValue, TransactionTrace>,
        DashMap<HashValue, TransactionTrace>,
        DashMap<HashValue, TransactionTrace>,
    ) {
        // Gather all transaction hashes for traces that are complete or expired
        let mut complete_transaction_hashes = vec![];
        let mut expired_transaction_hashes = vec![];
        let mut garbage_collected_transaction_hashes = vec![];
        for transaction_trace_entry in self.transaction_traces.iter() {
            let transaction_hash = transaction_trace_entry.key();
            let transaction_trace = transaction_trace_entry.value();

            // Check if the transaction trace is complete, expired or garbage collected
            if transaction_trace.is_complete() {
                complete_transaction_hashes.push(*transaction_hash);
            } else if transaction_trace.is_expired() {
                expired_transaction_hashes.push(*transaction_hash);
            } else if transaction_trace.is_garbage_collected() {
                garbage_collected_transaction_hashes.push(*transaction_hash);
            }
        }

        // Remove all complete transaction traces from the collector
        let complete_transaction_traces =
            remove_traces_from_collection(&self.transaction_traces, &complete_transaction_hashes);

        // Remove all expired transaction traces from the collector
        let expired_transaction_traces =
            remove_traces_from_collection(&self.transaction_traces, &expired_transaction_hashes);

        // Remove all garbage collected transaction traces from the collector
        let garbage_collected_transaction_traces = remove_traces_from_collection(
            &self.transaction_traces,
            &garbage_collected_transaction_hashes,
        );

        (
            complete_transaction_traces,
            expired_transaction_traces,
            garbage_collected_transaction_traces,
        )
    }

    /// Retrieves transaction traces for multiple transaction hashes
    pub fn get_traces_for_transaction_hashes(
        &self,
        transaction_hashes: &Vec<HashValue>,
    ) -> BTreeMap<HashValue, TransactionTrace> {
        let mut transaction_traces = BTreeMap::new();
        for transaction_hash in transaction_hashes {
            if let Some(transaction_trace_entry) = self.transaction_traces.get(transaction_hash) {
                let transaction_hash = transaction_trace_entry.key();
                let transaction_trace = transaction_trace_entry.value();

                transaction_traces.insert(*transaction_hash, transaction_trace.clone());
            }
        }
        transaction_traces
    }

    /// Returns all transaction hashes that we received from the specified peer
    pub fn get_transaction_hashes_received_from_peer(
        &self,
        peer_network_id: PeerNetworkId,
    ) -> Vec<HashValue> {
        // TODO: optimize this to avoid scanning all traces

        let mut transaction_hashes = Vec::new();
        for transaction_trace_entry in self.transaction_traces.iter() {
            let transaction_hash = transaction_trace_entry.key();
            let transaction_trace = transaction_trace_entry.value();

            // Check if the transaction was received from the specified peer
            let transaction_received_from = transaction_trace
                .get_authored_partial_trace()
                .transaction_received_from();
            if transaction_received_from == &TransactionSource::RemotePeer(peer_network_id) {
                transaction_hashes.push(*transaction_hash);
            }
        }
        transaction_hashes
    }

    /// Updates the transaction traces to note that we broadcast
    /// the specified transactions via mempool to the given peer.
    pub fn transactions_broadcast_via_mempool(
        &self,
        peer_network_id: PeerNetworkId,
        transaction_hashes: Vec<HashValue>,
    ) {
        for transaction_hash in transaction_hashes {
            // Retrieve the existing transaction trace
            if let Some(mut transaction_trace) = self.transaction_traces.get_mut(&transaction_hash)
            {
                // Mark the transaction as forwarded to the specified peer
                let transaction_destination = TransactionDestination::RemotePeer(peer_network_id);
                transaction_trace
                    .get_mut_authored_partial_trace()
                    .mark_transaction_forwarded(transaction_destination);
            } else {
                warn!(
                    "Cannot mark transaction {:?} as broadcast to peer {:?}, \
                    as no trace exists for that transaction!",
                    transaction_hash, peer_network_id
                );
            }
        }
    }

    /// Updates the transaction traces to note that the specified
    /// transactions have been committed.
    pub fn transactions_committed(&self, committed_transactions: &[CommittedTransaction]) {
        for committed_transaction in committed_transactions {
            // Retrieve the existing transaction trace. Note: it is possible that
            // no trace exists for a committed transaction, e.g., if the transactio
            // was never forwarded through this node.
            let transaction_hash = committed_transaction.committed_hash;
            if let Some(mut transaction_trace) = self.transaction_traces.get_mut(&transaction_hash)
            {
                // Mark the transaction as committed
                transaction_trace
                    .get_mut_authored_partial_trace()
                    .mark_transaction_committed();
            }
        }
    }

    /// Updates the transaction traces to note that the specified
    /// transaction has expired in mempool. If `client_expiration` is true,
    /// the expiration was due to a client-specified expiration time.
    /// Otherwise, it was due to a system time-to-live expiration.
    pub fn transaction_expired_in_mempool(
        &self,
        signed_transaction: &SignedTransaction,
        client_expiration: bool,
    ) {
        // Retrieve the existing transaction trace
        let transaction_hash = signed_transaction.committed_hash();
        if let Some(mut transaction_trace) = self.transaction_traces.get_mut(&transaction_hash) {
            // Mark the transaction as expired in mempool
            let expiration_reason = if client_expiration {
                TransactionExpirationReason::ClientExpiration
            } else {
                TransactionExpirationReason::SystemTTLExpiration
            };
            transaction_trace
                .get_mut_authored_partial_trace()
                .mark_transaction_expired(expiration_reason);
        } else {
            warn!(
                "Cannot mark transaction {:?} as expired in mempool, as no trace exists for that transaction!",
                transaction_hash
            );
        }
    }

    /// Updates the transaction traces to note that the specified
    /// transactions have been pulled by quorum store.
    pub fn transactions_pulled_by_quorum_store(&self, signed_transactions: &[SignedTransaction]) {
        for signed_transaction in signed_transactions {
            // Retrieve the existing transaction trace
            let transaction_hash = signed_transaction.committed_hash();
            if let Some(mut transaction_trace) = self.transaction_traces.get_mut(&transaction_hash)
            {
                // Mark the transaction as pulled by quorum store
                transaction_trace
                    .get_mut_authored_partial_trace()
                    .mark_transaction_pulled_by_quorum_store();
            } else {
                warn!(
                    "Cannot mark transaction {:?} as pulled by quorum store, as no trace exists for that transaction!",
                    transaction_hash
                );
            }
        }
    }

    /// Adds a new partial transaction trace noting that we
    /// received the specified transaction in mempool.
    pub fn transaction_received_in_mempool(
        &self,
        transaction_source: TransactionSource,
        transaction_hash: HashValue,
    ) {
        // Verify that we don't exceed the maximum number of active transaction traces
        let max_num_active_transaction_traces = self
            .transaction_tracing_config
            .max_num_active_transaction_traces;
        if self.transaction_traces.len() >= max_num_active_transaction_traces {
            warn!(
                "Cannot add new transaction trace for transaction {:?}, as we have reached the \
                maximum number of active transaction traces ({}). Ignoring transaction!",
                transaction_hash, max_num_active_transaction_traces
            );
            return;
        }

        // Determine the trace author for the partial trace
        let trace_author = match &transaction_source {
            TransactionSource::RemotePeer(remote_peer_network_id) => {
                let remote_network_id = remote_peer_network_id.network_id();
                match self.collector_identities.get(&remote_network_id) {
                    Some(peer_id) => PeerNetworkId::new(remote_network_id, *peer_id),
                    None => {
                        warn!(
                            "Received transaction from peer {:?}, but we do not have a collector \
                        identity for that network. Ignoring trace!",
                            remote_peer_network_id
                        );
                        return;
                    },
                }
            },
            TransactionSource::RestApi => {
                // Use the first available collector identity for REST API transactions
                if let Some((network_id, peer_id)) = self.collector_identities.iter().next() {
                    PeerNetworkId::new(*network_id, *peer_id)
                } else {
                    warn!(
                        "Received transaction from REST API, but we do not have any collector \
                    identities. Ignoring trace!"
                    );
                    return;
                }
            },
        };

        info!(
            "Creating new transaction trace for transaction {:?}, received from {:?}.",
            transaction_hash, transaction_source
        );

        // Create the new transaction trace
        let authored_partial_trace = PartialTrace::new(trace_author, transaction_source);
        let transaction_trace = TransactionTrace::new(
            self.transaction_tracing_config.clone(),
            authored_partial_trace,
        );

        // Add the transaction trace to the collection
        self.transaction_traces
            .insert(transaction_hash, transaction_trace);
    }

    /// Updates the transaction traces to note that the specified
    /// transactions have been rejected by quorum store.
    pub fn transactions_rejected_by_quorum_store(
        &self,
        rejected_transaction_summaries: &[RejectedTransactionSummary],
    ) {
        for transaction_summary in rejected_transaction_summaries {
            // Retrieve the existing transaction trace
            let transaction_hash = transaction_summary.hash;
            if let Some(mut transaction_trace) = self.transaction_traces.get_mut(&transaction_hash)
            {
                // Mark the transaction as rejected by quorum store
                transaction_trace
                    .get_mut_authored_partial_trace()
                    .mark_transaction_rejected_by_quorum_store(transaction_summary.reason);
            } else {
                warn!(
                    "Cannot mark transaction {:?} as failed, as no trace exists for that transaction!",
                    transaction_hash
                );
            }
        }
    }
}

/// Removes transaction traces from the given collection for the specified transaction hashes
fn remove_traces_from_collection(
    transaction_traces: &DashMap<HashValue, TransactionTrace>,
    hashes_to_remove: &[HashValue],
) -> DashMap<HashValue, TransactionTrace> {
    hashes_to_remove
        .iter()
        .filter_map(|transaction_hash| {
            transaction_traces
                .remove(transaction_hash)
                .map(|(_, transaction_trace)| (*transaction_hash, transaction_trace))
        })
        .collect()
}
