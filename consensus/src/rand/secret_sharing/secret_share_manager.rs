// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    counters::DEC_QUEUE_SIZE,
    logging::{LogEvent, LogSchema},
    network::{IncomingSecretShareRequest, NetworkSender, TConsensusMsg},
    pipeline::buffer_manager::{OrderedBlocks, ResetAck, ResetRequest, ResetSignal},
    rand::secret_sharing::{
        block_queue::{BlockQueue, QueueItem},
        network_messages::{SecretShareMessage, SecretShareRpc},
        reliable_broadcast_state::SecretShareAggregateState,
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
use aptos_logger::{error, info, spawn_named, warn};
use aptos_network::{protocols::network::RpcError, ProtocolId};
use aptos_reliable_broadcast::{DropGuard, ReliableBroadcast};
use aptos_time_service::TimeService;
use aptos_types::{epoch_state::EpochState, secret_sharing::SecretShareMetadata};
use bytes::Bytes;
use futures::{
    future::{AbortHandle, Abortable},
    stream::FuturesUnordered,
    FutureExt, StreamExt,
};
use futures_channel::{
    mpsc::{unbounded, UnboundedReceiver, UnboundedSender},
    oneshot,
};
use std::{future::Future, pin::Pin, sync::Arc, time::Duration};
use tokio_retry::strategy::ExponentialBackoff;

pub type Sender<T> = UnboundedSender<T>;
pub type Receiver<T> = UnboundedReceiver<T>;

type PendingDeriveFut =
    Pin<Box<dyn Future<Output = (Round, TaskResult<SecretShareResult>)> + Send>>;

pub struct SecretShareManager {
    author: Author,
    epoch_state: Arc<EpochState>,
    stop: bool,
    verifier: Arc<SecretShareVerifier>,
    reliable_broadcast: Arc<ReliableBroadcast<SecretShareMessage, ExponentialBackoff>>,
    network_sender: Arc<NetworkSender>,
    secret_share_request_delay_ms: u64,

    // local channel received from dec_store
    decision_rx: Receiver<SecretShareAggregationResult>,
    // downstream channels
    outgoing_blocks: Sender<OrderedBlocks>,
    // local state
    secret_share_store: Arc<Mutex<SecretShareStore>>,
    block_queue: BlockQueue,
    pending_derives: FuturesUnordered<PendingDeriveFut>,
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
            bounded_executor,
        ));
        let (decision_tx, decision_rx) = unbounded();

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

            decision_rx,
            outgoing_blocks,

            secret_share_store: dec_store,
            block_queue: BlockQueue::new(),
            pending_derives: FuturesUnordered::new(),
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
                return;
            },
            Err(e) => {
                error!(round = round, "Self-share derive failed: {:?}", e);
                return;
            },
        };

        let metadata = share.metadata().clone();
        {
            let mut store = self.secret_share_store.lock();
            if let Err(e) = store.add_self_share(share.clone()) {
                error!(round = round, "Failed to add self share to store: {:?}", e);
                return;
            }
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
                        let guard = self.spawn_share_requester_for_targets(metadata, targets, 0);
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
        let secret_share_store = self.secret_share_store.clone();
        let existing_shares = secret_share_store.lock().get_all_shares_authors(&metadata);
        let targets: Vec<Author> = match existing_shares {
            Some(existing) => self
                .epoch_state
                .verifier
                .get_ordered_account_addresses_iter()
                .filter(|author| !existing.contains(author))
                .collect(),
            None => return DropGuard::new(AbortHandle::new_pair().0),
        };
        self.spawn_share_requester_for_targets(
            metadata,
            targets,
            self.secret_share_request_delay_ms,
        )
    }

    fn spawn_share_requester_for_targets(
        &self,
        metadata: SecretShareMetadata,
        targets: Vec<Author>,
        delay_ms: u64,
    ) -> DropGuard {
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

    fn handle_incoming_msg(&self, rpc: SecretShareRpc) {
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
