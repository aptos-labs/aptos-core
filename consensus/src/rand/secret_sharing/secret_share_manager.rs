// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    counters::{DEC_QUEUE_SIZE, SECRET_SHARE_RECOVERY_COUNT},
    logging::{LogEvent, LogSchema},
    network::{IncomingSecretShareRequest, NetworkSender, TConsensusMsg},
    pipeline::buffer_manager::{OrderedBlocks, ResetAck, ResetRequest, ResetSignal},
    rand::secret_sharing::{
        block_queue::{BlockQueue, QueueItem},
        network_messages::{SecretShareMessage, SecretShareRpc},
        reliable_broadcast_state::SecretShareAggregateState,
        secret_share_recovery::SecretShareRecovery,
        secret_share_store::{SecretShareAggregationResult, SecretShareStore},
        types::RequestSecretShare,
        verifier::SecretShareVerifier,
    },
};
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::aptos_channel;
use aptos_config::config::ReliableBroadcastConfig;
use aptos_consensus_types::{
    common::{Author, Round},
    pipelined_block::{PipelinedBlock, SecretShareResult, TaskResult},
};
use aptos_infallible::Mutex;
use aptos_logger::{debug, error, info, spawn_named, warn};
use aptos_network::{protocols::network::RpcError, ProtocolId};
use aptos_reliable_broadcast::{DropGuard, ReliableBroadcast};
use aptos_time_service::TimeService;
use aptos_types::{
    epoch_state::EpochState,
    secret_sharing::{SecretShare, SecretShareMetadata},
};
use bytes::Bytes;
use fail::fail_point;
use futures::{
    future::{AbortHandle, Abortable},
    stream::FuturesUnordered,
    FutureExt, StreamExt,
};
use futures_channel::{
    mpsc::{unbounded, UnboundedReceiver, UnboundedSender},
    oneshot,
};
use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc, time::Duration};
use tokio_retry::strategy::ExponentialBackoff;

pub type Sender<T> = UnboundedSender<T>;
pub type Receiver<T> = UnboundedReceiver<T>;

type PendingDeriveFut =
    Pin<Box<dyn Future<Output = (Round, TaskResult<SecretShareResult>)> + Send>>;
type RecoveryResponse = (ProtocolId, oneshot::Sender<Result<Bytes, RpcError>>);
type RecoveryResult = (
    SecretShareMetadata,
    u64,
    anyhow::Result<Option<SecretShare>>,
);

struct PendingRecoveryRequests<T> {
    generation: u64,
    requests: HashMap<SecretShareMetadata, Vec<T>>,
}

impl<T> Default for PendingRecoveryRequests<T> {
    fn default() -> Self {
        Self {
            generation: 0,
            requests: HashMap::new(),
        }
    }
}

impl<T> PendingRecoveryRequests<T> {
    fn append_if_pending(&mut self, metadata: &SecretShareMetadata, request: T) -> Result<(), T> {
        match self.requests.get_mut(metadata) {
            Some(requests) => {
                requests.push(request);
                Ok(())
            },
            None => Err(request),
        }
    }

    fn insert(&mut self, metadata: SecretShareMetadata, request: T) {
        let previous = self.requests.insert(metadata, vec![request]);
        debug_assert!(previous.is_none());
    }

    fn reset(&mut self) {
        self.requests.clear();
        self.generation = self.generation.wrapping_add(1);
    }

    fn take(&mut self, metadata: &SecretShareMetadata, generation: u64) -> Option<Vec<T>> {
        (generation == self.generation)
            .then(|| self.requests.remove(metadata))
            .flatten()
    }
}

pub struct SecretShareManager {
    author: Author,
    epoch_state: Arc<EpochState>,
    stop: bool,
    verifier: Arc<SecretShareVerifier>,
    reliable_broadcast: Arc<ReliableBroadcast<SecretShareMessage, ExponentialBackoff>>,
    network_sender: Arc<NetworkSender>,
    secret_share_request_delay_ms: u64,
    recovery: Arc<dyn SecretShareRecovery>,
    recovery_executor: BoundedExecutor,

    // local channel received from dec_store
    decision_rx: Receiver<SecretShareAggregationResult>,
    // downstream channels
    outgoing_blocks: Sender<OrderedBlocks>,
    // local state
    secret_share_store: Arc<Mutex<SecretShareStore>>,
    block_queue: BlockQueue,
    pending_derives: FuturesUnordered<PendingDeriveFut>,
    pending_recoveries: PendingRecoveryRequests<RecoveryResponse>,
    recovery_result_tx: Sender<RecoveryResult>,
    recovery_result_rx: Receiver<RecoveryResult>,
}

impl SecretShareManager {
    pub fn new(
        author: Author,
        epoch_state: Arc<EpochState>,
        verifier: Arc<SecretShareVerifier>,
        outgoing_blocks: Sender<OrderedBlocks>,
        network_sender: Arc<NetworkSender>,
        bounded_executor: BoundedExecutor,
        rb_config: &ReliableBroadcastConfig,
        secret_share_request_delay_ms: u64,
        recovery: Arc<dyn SecretShareRecovery>,
    ) -> Self {
        let rb_backoff_policy = ExponentialBackoff::from_millis(rb_config.backoff_policy_base_ms)
            .factor(rb_config.backoff_policy_factor)
            .max_delay(Duration::from_millis(rb_config.backoff_policy_max_delay_ms));
        let reliable_broadcast = Arc::new(ReliableBroadcast::new(
            "secret_share_manager",
            author,
            epoch_state.verifier.get_ordered_account_addresses(),
            network_sender.clone(),
            rb_backoff_policy,
            TimeService::real(),
            Duration::from_millis(rb_config.rpc_timeout_ms),
            bounded_executor.clone(),
        ));
        let (decision_tx, decision_rx) = unbounded();
        let (recovery_result_tx, recovery_result_rx) = unbounded();

        let dec_store = Arc::new(Mutex::new(SecretShareStore::new(
            epoch_state.epoch,
            author,
            verifier.clone(),
            decision_tx,
        )));

        Self {
            author,
            epoch_state,
            stop: false,
            verifier,
            reliable_broadcast,
            network_sender,
            secret_share_request_delay_ms,
            recovery,
            recovery_executor: bounded_executor,

            decision_rx,
            outgoing_blocks,

            secret_share_store: dec_store,
            block_queue: BlockQueue::new(),
            pending_derives: FuturesUnordered::new(),
            pending_recoveries: PendingRecoveryRequests::default(),
            recovery_result_tx,
            recovery_result_rx,
        }
    }

    /// Processes a batch of incoming ordered blocks by registering their rounds
    /// in the store and deferring self-share derivation to `pending_derives`.
    fn process_incoming_blocks(&mut self, blocks: OrderedBlocks) -> anyhow::Result<()> {
        let rounds: Vec<u64> = blocks.ordered_blocks.iter().map(|b| b.round()).collect();
        info!(
            rounds = rounds,
            num_blocks = rounds.len(),
            "Processing incoming blocks."
        );

        for block in blocks.ordered_blocks.iter() {
            self.enqueue_self_derive(block)?;
        }

        self.block_queue.push_back(QueueItem::new(blocks));
        Ok(())
    }

    /// Registers the round in the store so remote shares can accumulate, and
    /// pushes the self-derive future into `pending_derives` for later resolution.
    fn enqueue_self_derive(&mut self, block: &PipelinedBlock) -> anyhow::Result<()> {
        let futures = block.pipeline_futs().ok_or_else(|| {
            anyhow::anyhow!("pipeline futures not set for round {}", block.round())
        })?;

        self.secret_share_store
            .lock()
            .update_highest_known_round(block.round());

        let round = block.round();
        let derive_fut = futures.secret_sharing_derive_self_fut.clone();
        self.pending_derives
            .push(Box::pin(async move { (round, derive_fut.await) }));
        Ok(())
    }

    /// Handles a completed self-share derivation: updates the store, broadcasts
    /// the share, and spawns the share requester task.
    fn process_completed_derive(&mut self, round: Round, result: TaskResult<SecretShareResult>) {
        let share = match result {
            Ok(Some(share)) => share,
            Ok(None) => {
                info!(
                    round = round,
                    "Self-share derive returned None (no encrypted txns), resolving round"
                );
                if let Some(item) = self.block_queue.item_mut(round) {
                    item.resolve_round_without_key(round);
                }
                self.secret_share_store.lock().mark_round_skipped(round);
                return;
            },
            Err(e) if e.is_cancellation() => {
                // Expected teardown: the block was dropped or a buffer_manager
                // reset aborted the pipeline, cancelling the derive future. Not
                // a failure, so log at debug and don't surface it as an error.
                debug!(round = round, "Self-share derive cancelled: {:?}", e);
                return;
            },
            Err(e) => {
                error!(round = round, "Self-share derive failed: {:?}", e);
                return;
            },
        };

        let metadata = share.metadata().clone();
        let drop_delivery = Self::drop_derived_share_delivery();
        if !drop_delivery {
            let mut store = self.secret_share_store.lock();
            if let Err(e) = store.add_self_share(share.clone()) {
                error!(round = round, "Failed to add self share to store: {:?}", e);
                return;
            }
        } else {
            warn!(
                round = round,
                "Dropping derived self-share delivery due to failpoint"
            );
        }

        info!(LogSchema::new(LogEvent::BroadcastSecretShare)
            .epoch(self.epoch_state.epoch)
            .author(self.author)
            .round(round));
        self.network_sender
            .broadcast_secret_share(SecretShareMessage::Share(share).into_network_message());

        let guard = self.spawn_share_requester_task(metadata);
        if let Some(item) = self.block_queue.item_mut(round) {
            item.push_share_requester_handle(guard);
        } else {
            warn!(
                round = round,
                "Secret share item not found for round {}", round
            );
        }
    }

    fn drop_derived_share_delivery() -> bool {
        fail_point!(
            "consensus::secret_share_manager::drop_derived_share_delivery",
            |_| true
        );
        false
    }

    fn process_ready_blocks(&mut self, ready_blocks: Vec<OrderedBlocks>) {
        let rounds: Vec<u64> = ready_blocks
            .iter()
            .flat_map(|b| b.ordered_blocks.iter().map(|b3| b3.round()))
            .collect();
        info!(rounds = rounds, "Processing secret share ready blocks.");

        for blocks in ready_blocks {
            if let Err(e) = self.outgoing_blocks.unbounded_send(blocks) {
                error!(
                    "[SecretShareManager] Failed to send ready blocks downstream: {}",
                    e
                );
            }
        }
    }

    fn process_reset(&mut self, request: ResetRequest) {
        let ResetRequest { tx, signal } = request;
        let target_round = match signal {
            ResetSignal::Stop => 0,
            ResetSignal::TargetRound(round) => round,
        };
        self.block_queue = BlockQueue::new();
        self.pending_derives = FuturesUnordered::new();
        self.pending_recoveries.reset();
        self.secret_share_store.lock().reset(target_round);
        self.stop = matches!(signal, ResetSignal::Stop);
        let _ = tx.send(ResetAck::default());
    }

    fn process_aggregation_result(&mut self, result: SecretShareAggregationResult) {
        match result {
            SecretShareAggregationResult::Success(secret_share_key) => {
                let round = secret_share_key.metadata.round;
                self.secret_share_store
                    .lock()
                    .handle_aggregation_success(round);
                if let Some(item) = self.block_queue.item_mut(round) {
                    item.set_secret_shared_key(round, secret_share_key);
                }
            },
            SecretShareAggregationResult::Failure {
                round,
                epoch,
                metadata,
                surviving_shares,
            } => {
                warn!(
                    epoch = epoch,
                    round = round,
                    "Background aggregation failed, retrying with {} surviving shares",
                    surviving_shares.len()
                );
                let existing_authors = self
                    .secret_share_store
                    .lock()
                    .handle_aggregation_failure(round, surviving_shares);

                if let Some(existing) = existing_authors {
                    let targets: Vec<Author> = self
                        .epoch_state
                        .verifier
                        .get_ordered_account_addresses_iter()
                        .filter(|author| !existing.contains(author))
                        .collect();
                    if !targets.is_empty() {
                        let guard =
                            self.spawn_share_requester_for_targets(metadata, Some(targets), 0);
                        if let Some(item) = self.block_queue.item_mut(round) {
                            item.push_share_requester_handle(guard);
                        }
                    }
                }
            },
        }
    }

    fn process_response(
        &self,
        protocol: ProtocolId,
        sender: oneshot::Sender<Result<Bytes, RpcError>>,
        message: SecretShareMessage,
    ) {
        let msg = message.into_network_message();
        let _ = sender.send(Ok(protocol
            .to_bytes(&msg)
            .expect("Message should be serializable into protocol")
            .into()));
    }

    fn schedule_recovery(&mut self, metadata: SecretShareMetadata, response: RecoveryResponse) {
        let response = match self
            .pending_recoveries
            .append_if_pending(&metadata, response)
        {
            Ok(()) => {
                SECRET_SHARE_RECOVERY_COUNT
                    .with_label_values(&["deduplicated"])
                    .inc();
                return;
            },
            Err(response) => response,
        };

        let recovery = self.recovery.clone();
        let result_tx = self.recovery_result_tx.clone();
        let generation = self.pending_recoveries.generation;
        let result_metadata = metadata.clone();
        let task = async move {
            let result = recovery.recover(result_metadata.clone()).await;
            let _ = result_tx.unbounded_send((result_metadata, generation, result));
        };
        match self.recovery_executor.try_spawn(task) {
            Ok(_) => {
                self.pending_recoveries.insert(metadata, response);
                SECRET_SHARE_RECOVERY_COUNT
                    .with_label_values(&["scheduled"])
                    .inc();
            },
            Err(_) => {
                warn!(
                    epoch = metadata.epoch,
                    round = metadata.round,
                    "Secret-share recovery executor is at capacity"
                );
                SECRET_SHARE_RECOVERY_COUNT
                    .with_label_values(&["at_capacity"])
                    .inc();
            },
        }
    }

    fn process_recovery_result(&mut self, result: RecoveryResult) {
        let (metadata, generation, result) = result;
        if generation != self.pending_recoveries.generation {
            SECRET_SHARE_RECOVERY_COUNT
                .with_label_values(&["stale"])
                .inc();
            return;
        }
        let Some(requesters) = self.pending_recoveries.take(&metadata, generation) else {
            return;
        };

        let recovered_share = match result {
            Ok(Some(share)) if share.metadata() == &metadata => share,
            Ok(Some(_)) => {
                warn!(
                    epoch = metadata.epoch,
                    round = metadata.round,
                    "Recovered secret share has mismatched metadata"
                );
                SECRET_SHARE_RECOVERY_COUNT
                    .with_label_values(&["metadata_mismatch"])
                    .inc();
                return;
            },
            Ok(None) => {
                SECRET_SHARE_RECOVERY_COUNT
                    .with_label_values(&["unavailable"])
                    .inc();
                return;
            },
            Err(error) => {
                warn!(
                    epoch = metadata.epoch,
                    round = metadata.round,
                    "Failed to recover secret share: {error:#}"
                );
                SECRET_SHARE_RECOVERY_COUNT
                    .with_label_values(&["error"])
                    .inc();
                return;
            },
        };

        let (share_to_send, inserted) = {
            let mut store = self.secret_share_store.lock();
            let already_present = matches!(store.get_self_share(&metadata), Ok(Some(_)));
            let inserted = if already_present {
                false
            } else {
                match store.add_self_share(recovered_share) {
                    Ok(()) => true,
                    Err(error) => {
                        warn!(
                            epoch = metadata.epoch,
                            round = metadata.round,
                            "Failed to add recovered self share: {error:#}"
                        );
                        false
                    },
                }
            };
            let share = match store.get_self_share(&metadata) {
                Ok(share) => share,
                Err(error) => {
                    warn!(
                        epoch = metadata.epoch,
                        round = metadata.round,
                        "Failed to read recovered self share: {error:#}"
                    );
                    None
                },
            };
            (share, inserted)
        };
        let Some(share) = share_to_send else {
            SECRET_SHARE_RECOVERY_COUNT
                .with_label_values(&["store_conflict"])
                .inc();
            return;
        };

        for (protocol, response_sender) in requesters {
            self.process_response(
                protocol,
                response_sender,
                SecretShareMessage::Share(share.clone()),
            );
        }
        if inserted {
            let guard = self.spawn_share_requester_task(metadata.clone());
            if let Some(item) = self.block_queue.item_mut(metadata.round) {
                item.push_share_requester_handle(guard);
            } else {
                warn!(
                    round = metadata.round,
                    "Recovered secret share item not found in block queue"
                );
            }
        }
        SECRET_SHARE_RECOVERY_COUNT
            .with_label_values(&["success"])
            .inc();
    }

    async fn verification_task(
        epoch_state: Arc<EpochState>,
        mut incoming_rpc_request: aptos_channel::Receiver<Author, IncomingSecretShareRequest>,
        verified_msg_tx: UnboundedSender<SecretShareRpc>,
        verifier: Arc<SecretShareVerifier>,
        bounded_executor: BoundedExecutor,
    ) {
        while let Some(dec_msg) = incoming_rpc_request.next().await {
            let tx = verified_msg_tx.clone();
            let epoch_state_clone = epoch_state.clone();
            let verifier_clone = verifier.clone();
            bounded_executor
                .spawn_blocking(move || {
                    match bcs::from_bytes::<SecretShareMessage>(dec_msg.req.data()) {
                        Ok(msg) => {
                            if msg
                                .verify(&epoch_state_clone, &verifier_clone, &dec_msg.sender)
                                .is_ok()
                            {
                                let _ = tx.unbounded_send(SecretShareRpc {
                                    msg,
                                    protocol: dec_msg.protocol,
                                    response_sender: dec_msg.response_sender,
                                });
                            }
                        },
                        Err(e) => {
                            warn!("Invalid dec message: {}", e);
                        },
                    }
                })
                .await;
        }
    }

    fn spawn_share_requester_task(&self, metadata: SecretShareMetadata) -> DropGuard {
        self.spawn_share_requester_for_targets(metadata, None, self.secret_share_request_delay_ms)
    }

    fn spawn_share_requester_for_targets(
        &self,
        metadata: SecretShareMetadata,
        targets: Option<Vec<Author>>,
        delay_ms: u64,
    ) -> DropGuard {
        let secret_share_store = self.secret_share_store.clone();
        let epoch_state = self.epoch_state.clone();
        let rb = self.reliable_broadcast.clone();
        let aggregate_state = Arc::new(SecretShareAggregateState::new(
            self.secret_share_store.clone(),
            metadata.clone(),
            self.verifier.clone(),
        ));
        let epoch = self.epoch_state.epoch;
        let task = async move {
            if delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
            let targets: Vec<Author> = match targets {
                Some(t) => t,
                None => {
                    let existing_shares =
                        secret_share_store.lock().get_all_shares_authors(&metadata);
                    match existing_shares {
                        Some(existing) => epoch_state
                            .verifier
                            .get_ordered_account_addresses_iter()
                            .filter(|author| !existing.contains(author))
                            .collect(),
                        None => return,
                    }
                },
            };
            if targets.is_empty() {
                return;
            }
            info!(
                epoch = epoch,
                round = metadata.round,
                "[SecretShareManager] Start broadcasting share request for {}",
                targets.len(),
            );
            if let Err(e) = rb
                .multicast(
                    RequestSecretShare::new(metadata.clone()),
                    aggregate_state,
                    targets,
                )
                .await
            {
                warn!(
                    epoch = epoch,
                    round = metadata.round,
                    "[SecretShareManager] Share request broadcast failed: {}",
                    e,
                );
                return;
            }
            info!(
                epoch = epoch,
                round = metadata.round,
                "[SecretShareManager] Finish broadcasting share request",
            );
        };
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(task, abort_registration));
        DropGuard::new(abort_handle)
    }

    fn handle_incoming_msg(&mut self, rpc: SecretShareRpc) {
        let SecretShareRpc {
            msg,
            protocol,
            response_sender,
        } = rpc;
        match msg {
            SecretShareMessage::RequestShare(request) => {
                let result = self
                    .secret_share_store
                    .lock()
                    .get_self_share(request.metadata());
                match result {
                    Ok(Some(share)) => {
                        self.process_response(
                            protocol,
                            response_sender,
                            SecretShareMessage::Share(share),
                        );
                    },
                    Ok(None) => {
                        warn!(
                            "Self secret share could not be found for RPC request {}",
                            request.metadata().round
                        );
                        self.schedule_recovery(
                            request.metadata().clone(),
                            (protocol, response_sender),
                        );
                    },
                    Err(e) => {
                        warn!("[SecretShareManager] Failed to get share: {}", e);
                    },
                }
            },
            SecretShareMessage::Share(share) => {
                info!(LogSchema::new(LogEvent::ReceiveSecretShare)
                    .author(self.author)
                    .epoch(share.epoch())
                    .round(share.metadata().round)
                    .remote_peer(*share.author()));

                if let Err(e) = self.secret_share_store.lock().add_share(share) {
                    warn!("[SecretShareManager] Failed to add share: {}", e);
                }
            },
        }
    }

    pub async fn start(
        mut self,
        mut incoming_blocks: Receiver<OrderedBlocks>,
        incoming_rpc_request: aptos_channel::Receiver<Author, IncomingSecretShareRequest>,
        mut reset_rx: Receiver<ResetRequest>,
        bounded_executor: BoundedExecutor,
        highest_known_round: Round,
    ) {
        info!("SecretShareManager started");
        let (verified_msg_tx, mut verified_msg_rx) = unbounded();
        let epoch_state = self.epoch_state.clone();
        let verifier = self.verifier.clone();
        {
            self.secret_share_store
                .lock()
                .update_highest_known_round(highest_known_round);
        }
        spawn_named!(
            "Secret Share Manager Verification Task",
            Self::verification_task(
                epoch_state,
                incoming_rpc_request,
                verified_msg_tx,
                verifier,
                bounded_executor,
            )
        );

        let mut interval = tokio::time::interval(Duration::from_millis(5000));
        while !self.stop {
            tokio::select! {
                Some(blocks) = incoming_blocks.next() => {
                    if let Err(e) = self.process_incoming_blocks(blocks) {
                        error!("error processing incoming blocks: {:?}", e);
                    }
                }
                Some((round, result)) = self.pending_derives.next() => {
                    self.process_completed_derive(round, result);
                }
                Some(reset) = reset_rx.next() => {
                    let mut dropped = 0;
                    while matches!(incoming_blocks.try_next(), Ok(Some(_))) {
                        dropped += 1;
                    }
                    if dropped > 0 {
                        info!("[SecretShareManager] Dropped {} incoming block batches during reset", dropped);
                    }
                    self.process_reset(reset);
                }
                Some(result) = self.decision_rx.next() => {
                    self.process_aggregation_result(result);
                }
                Some(request) = verified_msg_rx.next() => {
                    self.handle_incoming_msg(request);
                }
                Some(result) = self.recovery_result_rx.next() => {
                    self.process_recovery_result(result);
                }
                _ = interval.tick().fuse() => {
                    self.observe_queue();
                },
            }
            let maybe_ready_blocks = self.block_queue.dequeue_ready_prefix();
            if !maybe_ready_blocks.is_empty() {
                self.process_ready_blocks(maybe_ready_blocks);
            }
        }
        info!("SecretShareManager stopped");
    }

    pub fn observe_queue(&self) {
        let queue = &self.block_queue.queue();
        DEC_QUEUE_SIZE.set(queue.len() as i64);
    }
}

#[cfg(test)]
mod tests {
    use super::PendingRecoveryRequests;
    use crate::rand::secret_sharing::test_utils::create_metadata;

    #[test]
    fn pending_recovery_deduplicates_and_fans_out() {
        let metadata = create_metadata(1, 10);
        let mut pending = PendingRecoveryRequests::default();
        assert_eq!(pending.append_if_pending(&metadata, 1), Err(1));
        pending.insert(metadata.clone(), 1);
        assert_eq!(pending.append_if_pending(&metadata, 2), Ok(()));
        assert_eq!(pending.take(&metadata, 0), Some(vec![1, 2]));
    }

    #[test]
    fn pending_recovery_reset_discards_stale_results() {
        let metadata = create_metadata(1, 10);
        let mut pending = PendingRecoveryRequests::default();
        pending.insert(metadata.clone(), 1);
        let stale_generation = pending.generation;
        pending.reset();
        assert!(pending.take(&metadata, stale_generation).is_none());
        assert_eq!(pending.generation, stale_generation + 1);
    }
}
