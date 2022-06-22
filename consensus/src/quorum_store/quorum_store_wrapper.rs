// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::network::NetworkSender;
use crate::network_interface::ConsensusMsg;
use crate::quorum_store::batch_reader::BatchReader;
use crate::quorum_store::quorum_store::QuorumStoreError;
use crate::quorum_store::{counters, quorum_store::QuorumStoreCommand};
use aptos_infallible::Mutex;
use aptos_mempool::{QuorumStoreRequest, QuorumStoreResponse};
use aptos_metrics_core::monitor;
use aptos_types::transaction::SignedTransaction;
use consensus_types::proof_of_store::LogicalTime;
use consensus_types::{
    common::TransactionSummary,
    proof_of_store::{ProofOfStore, SignedDigestInfo},
    request_response::ConsensusRequest,
};
use futures::future::BoxFuture;
use futures::stream::FuturesUnordered;
use futures::{
    channel::{
        mpsc::{Receiver, Sender},
        oneshot,
    },
    SinkExt, StreamExt, TryFutureExt,
};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};
use tokio::time::timeout;

// TODO: how long to keep filtering transactions from a batch? need some kind of notification from consensus?
pub struct QuorumStoreWrapper {
    batch_reader: Arc<BatchReader>,
    consensus_receiver: Receiver<ConsensusRequest>,
    mempool_sender: Sender<QuorumStoreRequest>,
    mempool_txn_pull_timeout_ms: u64,
    quorum_store_sender: tokio::sync::mpsc::Sender<QuorumStoreCommand>,
    network_sender: NetworkSender,
    batches: HashMap<SignedDigestInfo, Vec<TransactionSummary>>,
    batch_in_progress: Vec<TransactionSummary>,
    latest_logical_time: LogicalTime,
    // TODO: store all ProofOfStore (created locally, and received via broadcast)
    // TODO: need to be notified of ProofOfStore's that were committed
}

impl QuorumStoreWrapper {
    pub fn new(
        epoch: u64,
        batch_reader: Arc<BatchReader>,
        consensus_receiver: Receiver<ConsensusRequest>,
        mempool_sender: Sender<QuorumStoreRequest>,
        quorum_store_sender: tokio::sync::mpsc::Sender<QuorumStoreCommand>,
        network_sender: NetworkSender,
        mempool_txn_pull_timeout_ms: u64,
    ) -> Self {
        Self {
            batch_reader,
            consensus_receiver,
            mempool_sender,
            mempool_txn_pull_timeout_ms,
            quorum_store_sender,
            network_sender,
            batches: HashMap::new(),
            batch_in_progress: vec![],
            latest_logical_time: LogicalTime::new(epoch, 0),
        }
    }

    async fn pull_internal(
        &self,
        max_size: u64,
        exclude_txns: Vec<TransactionSummary>,
    ) -> Result<Vec<SignedTransaction>, anyhow::Error> {
        let (callback, callback_rcv) = oneshot::channel();
        let msg = QuorumStoreRequest::GetBatchRequest(max_size, exclude_txns, callback);
        self.mempool_sender
            .clone()
            .try_send(msg)
            .map_err(anyhow::Error::from)?;
        // wait for response
        match monitor!(
            "pull_txn",
            timeout(
                Duration::from_millis(self.mempool_txn_pull_timeout_ms),
                callback_rcv
            )
            .await
        ) {
            Err(_) => Err(anyhow::anyhow!(
                "[direct_mempool_quorum_store] did not receive GetBatchResponse on time"
            )),
            Ok(resp) => match resp.map_err(anyhow::Error::from)?? {
                QuorumStoreResponse::GetBatchResponse(txns) => Ok(txns),
                _ => Err(anyhow::anyhow!(
                    "[direct_mempool_quorum_store] did not receive expected GetBatchResponse"
                )),
            },
        }
    }

    async fn handle_scheduled_pull(
        &mut self,
    ) -> Option<oneshot::Receiver<Result<ProofOfStore, QuorumStoreError>>> {
        // TODO: include batch_in_progress
        let exclude_txns: Vec<_> = self.batches.values().flatten().cloned().collect();
        // TODO: size and unwrap or not?
        let pulled_txns = self.pull_internal(50, exclude_txns).await.unwrap();
        self.batch_in_progress
            .extend(pulled_txns.iter().map(|txn| TransactionSummary {
                sender: txn.sender(),
                sequence_number: txn.sequence_number(),
            }));

        // TODO: also some timer if there are not enough txns
        if self.batch_in_progress.len() <= 100 {
            self.quorum_store_sender
                .send(QuorumStoreCommand::AppendToBatch(pulled_txns))
                .await;
            None
        } else {
            let (proof_tx, proof_rx) = oneshot::channel();
            self.quorum_store_sender
                .send(QuorumStoreCommand::EndBatch(
                    pulled_txns,
                    LogicalTime::new(
                        self.latest_logical_time.epoch(),
                        self.latest_logical_time.round() + 10,
                    ), // TODO
                    proof_tx,
                ))
                .await;
            Some(proof_rx)
        }
    }

    async fn handle_proof_completed(&mut self, msg: Result<ProofOfStore, QuorumStoreError>) {
        match msg {
            Ok(proof) => {
                self.network_sender
                    .broadcast_without_self(ConsensusMsg::ProofOfStoreBroadcastMsg(Box::new(proof)))
                    .await
            }
            Err(_) => {
                // TODO: cast to anyhow?
            }
        }
    }

    async fn handle_consensus_request(&mut self, msg: ConsensusRequest) {
        match msg {
            ConsensusRequest::GetBlockRequest(_max_block_size, _filter, _callback) => {
                // TODO: Fill up from the seen ProofOfStores, after applying the filter.
                // TODO: Pass along to batch_store
            }
            ConsensusRequest::CleanRequest(epoch, round, _callback) => {
                self.latest_logical_time = LogicalTime::new(epoch, round);
                self.batch_reader
                    .update_certified_round(self.latest_logical_time)
                    .await;
            }
        }
    }

    pub async fn start(mut self) {
        let mut scheduled_pulls: FuturesUnordered<ScheduledPull> = FuturesUnordered::new();
        scheduled_pulls.push(ScheduledPull::new(
            Instant::now() + Duration::from_millis(50),
            false,
        ));
        let mut proofs_in_progress: FuturesUnordered<BoxFuture<'_, _>> = FuturesUnordered::new();

        loop {
            let _timer = counters::MAIN_LOOP.start_timer();
            ::futures::select! {
                _backoff = scheduled_pulls.next() => {
                    if let Some(proof_rx) = self.handle_scheduled_pull().await {
                        proofs_in_progress.push(Box::pin(proof_rx));
                    }
                    scheduled_pulls.push(ScheduledPull::new(
                        Instant::now() + Duration::from_millis(50),
                        false
                    ));
                },
                next = proofs_in_progress.next() => {
                    // TODO: handle failures
                    if let Some(Ok(msg)) = next {
                        self.handle_proof_completed(msg).await;
                    }
                },
                msg = self.consensus_receiver.select_next_some() => {
                    self.handle_consensus_request(msg).await;
                }
                complete => break,
            }
        }

        // Periodically:
        // 1. Pull from mempool.
        // 2. a. Start a batch with these txns if batch is not active
        //    b. Continue batch with these txns if batch is active
        // 3. Close batch if criteria is met.

        // State needed:
        // 1. txn summaries that are part of all pending batches: map<batch_id, vec<txn>>
        //    - pending batches: batches, including those in progress, that have not yet been cleaned.
        //    - batch_id: needs to include epoch, round info.
        // 2. all completed digests that have not yet been cleaned: map<batch_id, digest>
        //    -- is this really needed? pull_payload filters anyway. maybe all that's needed
        //    is a broadcast queue?
    }
}

/// From: Mempool ScheduledBroadcast
pub(crate) struct ScheduledPull {
    /// Time of scheduled pull
    deadline: Instant,
    backoff: bool,
    waker: Arc<Mutex<Option<Waker>>>,
}

impl ScheduledPull {
    pub fn new(deadline: Instant, backoff: bool) -> Self {
        let waker: Arc<Mutex<Option<Waker>>> = Arc::new(Mutex::new(None));
        let waker_clone = waker.clone();

        if deadline > Instant::now() {
            let tokio_instant = tokio::time::Instant::from_std(deadline);
            // TODO: something more general?
            tokio::spawn(async move {
                tokio::time::sleep_until(tokio_instant).await;
                let mut waker = waker_clone.lock();
                if let Some(waker) = waker.take() {
                    waker.wake()
                }
            });
        }

        Self {
            deadline,
            backoff,
            waker,
        }
    }
}

impl Future for ScheduledPull {
    type Output = bool; // whether this pull was scheduled as a backoff

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        if Instant::now() < self.deadline {
            let waker_clone = context.waker().clone();
            let mut waker = self.waker.lock();
            *waker = Some(waker_clone);

            Poll::Pending
        } else {
            Poll::Ready(self.backoff)
        }
    }
}
