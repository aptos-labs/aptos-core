// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::error::QuorumStoreError;
use crate::monitor;
use anyhow::{format_err, Result};
use aptos_infallible::Mutex;
use consensus_types::{common::Round, request_response::ConsensusRequest};
use futures::channel::{mpsc, mpsc::Sender, oneshot};
use std::time::Duration;
use tokio::time::timeout;

/// Notification of execution committed logical time for QuorumStore to clean.
#[async_trait::async_trait]
pub trait CommitNotifier: Send + Sync {
    /// Notification of committed logical time
    async fn notify_commit(&self, epoch: u64, round: Round) -> Result<(), QuorumStoreError>;

    fn new_epoch(&self, quorum_store_commit_sender: mpsc::Sender<ConsensusRequest>);
}

/// Execution -> QuorumStore notification of commits.
pub struct QuorumStoreCommitNotifier {
    quorum_store_commit_sender: Mutex<mpsc::Sender<ConsensusRequest>>,
    /// Timeout for QuorumStore clean ack
    quorum_store_commit_timeout_ms: u64,
}

impl QuorumStoreCommitNotifier {
    /// new
    pub fn new(quorum_store_commit_timeout_ms: u64) -> Self {
        let (dummy_sender, _) = mpsc::channel(1);
        Self {
            quorum_store_commit_sender: Mutex::new(dummy_sender),
            quorum_store_commit_timeout_ms,
        }
    }
}

#[async_trait::async_trait]
impl CommitNotifier for QuorumStoreCommitNotifier {
    async fn notify_commit(&self, epoch: u64, round: Round) -> Result<(), QuorumStoreError> {
        let (callback, callback_rcv) = oneshot::channel();
        let req = ConsensusRequest::CleanRequest(epoch, round, callback);

        self.quorum_store_commit_sender
            .lock()
            .clone()
            .try_send(req)
            .map_err(anyhow::Error::from)?;

        if let Err(e) = monitor!(
            "notify_commit",
            timeout(
                Duration::from_millis(self.quorum_store_commit_timeout_ms),
                callback_rcv
            )
            .await
        ) {
            Err(format_err!(
                "[consensus] quorum store commit notifier did not receive ACK on time: {:?}",
                e
            )
            .into())
        } else {
            Ok(())
        }
    }

    fn new_epoch(&self, quorum_store_commit_sender: Sender<ConsensusRequest>) {
        *self.quorum_store_commit_sender.lock() = quorum_store_commit_sender;
    }
}
