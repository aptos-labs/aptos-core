// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{error::MempoolError, monitor};
use anyhow::{format_err, Result};
use aptos_consensus_types::common::RejectedTransactionSummary;
use aptos_mempool::QuorumStoreRequest;
use aptos_types::transaction::{SignedTransaction, TransactionStatus};
use futures::channel::{mpsc, oneshot};
use itertools::Itertools;
use std::time::Duration;
use tokio::time::timeout;

/// Notification of failed transactions.
#[async_trait::async_trait]
pub trait TxnNotifier: Send + Sync {
    /// Notification of txns which failed execution. (Committed txns is notified by
    /// state sync.)
    async fn notify_failed_txn(
        &self,
        txns: &[SignedTransaction],
        statuses: &[TransactionStatus],
    ) -> Result<(), MempoolError>;
}

/// Execution -> Mempool notification of failed transactions.
pub struct MempoolNotifier {
    consensus_to_mempool_sender: mpsc::Sender<QuorumStoreRequest>,
    /// Timeout for consensus to get an ack from mempool for executed transactions (in milliseconds)
    mempool_executed_txn_timeout_ms: u64,
}

impl MempoolNotifier {
    /// new
    pub fn new(
        consensus_to_mempool_sender: mpsc::Sender<QuorumStoreRequest>,
        mempool_executed_txn_timeout_ms: u64,
    ) -> Self {
        Self {
            consensus_to_mempool_sender,
            mempool_executed_txn_timeout_ms,
        }
    }
}

#[async_trait::async_trait]
impl TxnNotifier for MempoolNotifier {
    async fn notify_failed_txn(
        &self,
        user_txns: &[SignedTransaction],
        user_txn_statuses: &[TransactionStatus],
    ) -> Result<(), MempoolError> {
        let mut rejected_txns = vec![];

        for (txn, status) in user_txns.iter().zip_eq(user_txn_statuses) {
            if let TransactionStatus::Discard(reason) = status {
                rejected_txns.push(RejectedTransactionSummary {
                    sender: txn.sender(),
                    replay_protector: txn.replay_protector(),
                    hash: txn.committed_hash(),
                    reason: *reason,
                });
            }
        }

        if rejected_txns.is_empty() {
            return Ok(());
        }

        let (callback, callback_rcv) = oneshot::channel();
        let req = QuorumStoreRequest::RejectNotification(rejected_txns, callback);

        // send to shared mempool
        self.consensus_to_mempool_sender
            .clone()
            .try_send(req)
            .map_err(anyhow::Error::from)?;

        if let Err(e) = monitor!(
            "notify_mempool",
            timeout(
                Duration::from_millis(self.mempool_executed_txn_timeout_ms),
                callback_rcv
            )
            .await
        ) {
            Err(format_err!("[consensus] txn notifier did not receive ACK for commit notification sent to mempool on time: {:?}", e).into())
        } else {
            Ok(())
        }
    }
}
