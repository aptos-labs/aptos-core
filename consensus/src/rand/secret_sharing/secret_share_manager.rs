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
        storage::{storage_key, SecretShareKey, SecretShareStorage},
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

struct RecoveredSelfShare {
    share: SecretShare,
    verified: bool,
}

pub struct SecretShareManager {
    author: Author,
    epoch_state: Arc<EpochState>,
    stop: bool,
    verifier: Arc<SecretShareVerifier>,
    reliable_broadcast: Arc<ReliableBroadcast<SecretShareMessage, ExponentialBackoff>>,
    network_sender: Arc<NetworkSender>,
    secret_share_storage: Arc<dyn SecretShareStorage>,
    retention_rounds: Round,
    secret_share_request_delay_ms: u64,

    // local channel received from dec_store
    decision_rx: Receiver<SecretShareAggregationResult>,
    // downstream channels
    outgoing_blocks: Sender<OrderedBlocks>,
    // local state
    secret_share_store: Arc<Mutex<SecretShareStore>>,
    recovered_self_shares: HashMap<SecretShareKey, RecoveredSelfShare>,
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
        secret_share_storage: Arc<dyn SecretShareStorage>,
        highest_committed_round: Round,
        retention_rounds: Round,
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
            bounded_executor.clone(),
        ));
        let (decision_tx, decision_rx) = unbounded();

        let dec_store = Arc::new(Mutex::new(SecretShareStore::new(
            epoch_state.epoch,
            author,
            verifier.clone(),
            decision_tx,
        )));
        if let Err(error) = secret_share_storage.prune_before_epoch(epoch_state.epoch) {
            error!(
                epoch = epoch_state.epoch,
                "Failed to prune old secret shares at epoch start: {error}"
            );
        }
        let oldest_retained_round = highest_committed_round.saturating_sub(retention_rounds);
        if let Err(error) =
            secret_share_storage.prune_before_round(epoch_state.epoch, oldest_retained_round)
        {
            error!(
                epoch = epoch_state.epoch,
                oldest_retained_round = oldest_retained_round,
                "Failed to prune expired secret shares at epoch start: {error}"
            );
        }
        let loaded_self_shares = secret_share_storage
            .load_self_shares(epoch_state.epoch)
            .unwrap_or_else(|error| panic!("Failed to load secret shares at epoch start: {error}"));
        let mut recovered_self_shares = HashMap::new();
        for loaded_share in loaded_self_shares {
            let share = match loaded_share {
                Ok(share) => share,
                Err(error) => {
                    error!("Ignoring invalid persisted secret share: {error}");
                    continue;
                },
            };
            if share.epoch() != epoch_state.epoch || share.author() != &author {
                error!(
                    expected_epoch = epoch_state.epoch,
                    share_epoch = share.epoch(),
                    expected_author = author,
                    share_author = share.author(),
                    "Ignoring persisted secret share with invalid identity"
                );
                continue;
            }
            recovered_self_shares.insert(storage_key(share.metadata()), RecoveredSelfShare {
                share,
                verified: false,
            });
        }

        Self {
            author,
            epoch_state,
            stop: false,
            verifier,
            reliable_broadcast,
            network_sender,
            secret_share_storage,
            retention_rounds,
            secret_share_request_delay_ms,

            decision_rx,
            outgoing_blocks,

            secret_share_store: dec_store,
            recovered_self_shares,
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

    /// Handles a completed self-share derivation. The synchronous database write
    /// must succeed before the share enters either memory cache or the network.
    fn process_completed_derive(&mut self, round: Round, result: TaskResult<SecretShareResult>) {
        let share = match result {
            Ok(Some(share)) => share,
            Ok(None) => {
                info!(
                    round = round,
                    "Self-share derive returned None (no encrypted txns), resolving round"
                );
                self.prune_expired_self_shares(round);
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

        if share.author() != &self.author
            || share.epoch() != self.epoch_state.epoch
            || share.round() != round
        {
            error!(
                epoch = self.epoch_state.epoch,
                round = round,
                share_epoch = share.epoch(),
                share_round = share.round(),
                share_author = share.author(),
                "Derived self share has invalid identity or metadata"
            );
            return;
        }

        if let Err(error) = self.secret_share_storage.save_self_share(&share) {
            error!(
                epoch = share.epoch(),
                round = round,
                block_id = share.metadata().block_id,
                "CRITICAL: Failed to persist self secret share: {error}"
            );
            return;
        }
        self.recovered_self_shares
            .insert(storage_key(share.metadata()), RecoveredSelfShare {
                share: share.clone(),
                verified: false,
            });
        self.prune_expired_self_shares(round);

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

    fn prune_expired_self_shares(&mut self, latest_round: Round) {
        let oldest_retained_round = latest_round.saturating_sub(self.retention_rounds);
        let previous_len = self.recovered_self_shares.len();
        self.recovered_self_shares
            .retain(|_, recovered| recovered.share.round() >= oldest_retained_round);
        if self.recovered_self_shares.len() == previous_len {
            return;
        }
        if let Err(error) = self
            .secret_share_storage
            .prune_before_round(self.epoch_state.epoch, oldest_retained_round)
        {
            error!(
                epoch = self.epoch_state.epoch,
                oldest_retained_round = oldest_retained_round,
                "Failed to prune expired secret shares: {error}"
            );
        }
    }

    fn get_recovered_self_share(&mut self, metadata: &SecretShareMetadata) -> Option<SecretShare> {
        let key = storage_key(metadata);
        let verification_error = {
            let recovered = self.recovered_self_shares.get_mut(&key)?;
            if recovered.share.metadata() != metadata {
                return None;
            }
            if recovered.verified {
                return Some(recovered.share.clone());
            }
            match self.verifier.verify(&recovered.share, &self.author) {
                Ok(()) => {
                    recovered.verified = true;
                    return Some(recovered.share.clone());
                },
                Err(error) => error,
            }
        };

        self.recovered_self_shares.remove(&key);
        error!(
            epoch = metadata.epoch,
            round = metadata.round,
            block_id = metadata.block_id,
            "Rejecting cryptographically invalid persisted secret share: {verification_error}"
        );
        None
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
        let (target_round, stop) = match signal {
            ResetSignal::Stop => (0, true),
            ResetSignal::TargetRound(round) => (round, false),
        };
        if !stop {
            self.prune_expired_self_shares(target_round);
        }
        self.block_queue = BlockQueue::new();
        self.pending_derives = FuturesUnordered::new();
        self.secret_share_store.lock().reset(target_round);
        self.stop = stop;
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
                if request.metadata().epoch != self.epoch_state.epoch {
                    warn!(
                        requested_epoch = request.metadata().epoch,
                        current_epoch = self.epoch_state.epoch,
                        "Rejecting secret share request from old or future epoch"
                    );
                    return;
                }
                let result = self
                    .secret_share_store
                    .lock()
                    .get_self_share(request.metadata());
                let active_share = match result {
                    Ok(Some(share)) => Some(share),
                    Ok(None) => None,
                    Err(error) => {
                        debug!("Self share not available in active store: {error}");
                        None
                    },
                };
                if let Some(share) = active_share {
                    self.process_response(
                        protocol,
                        response_sender,
                        SecretShareMessage::Share(share),
                    );
                    return;
                }

                let recovered_share = self.get_recovered_self_share(request.metadata());
                if let Some(share) = recovered_share {
                    self.process_response(
                        protocol,
                        response_sender,
                        SecretShareMessage::Share(share),
                    );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        network_interface::{ConsensusMsg, ConsensusNetworkClient, DIRECT_SEND, RPC},
        rand::secret_sharing::{
            storage::{InMemorySecretShareStorage, LoadedSecretShare, SecretShareDb},
            test_utils::{
                create_bad_secret_share, create_metadata, create_secret_share, TestContext,
            },
        },
    };
    use aptos_channels::{aptos_channel, message_queues::QueueStyle};
    use aptos_config::{
        config::ReliableBroadcastConfig,
        network_id::{NetworkId, PeerNetworkId},
    };
    use aptos_network::{
        application::{interface::NetworkClient, storage::PeersAndMetadata},
        peer_manager::{ConnectionRequestSender, PeerManagerRequest, PeerManagerRequestSender},
        protocols::{network::NewNetworkSender, wire::handshake::v1::ProtocolIdSet},
        transport::ConnectionMetadata,
    };
    use aptos_temppath::TempPath;
    use maplit::hashmap;
    use std::iter::FromIterator;

    struct FailingStorage;

    impl SecretShareStorage for FailingStorage {
        fn save_self_share(&self, _share: &SecretShare) -> anyhow::Result<()> {
            anyhow::bail!("injected failure")
        }

        fn load_self_shares(&self, _epoch: u64) -> anyhow::Result<Vec<LoadedSecretShare>> {
            Ok(Vec::new())
        }

        fn prune_before_epoch(&self, _epoch: u64) -> anyhow::Result<()> {
            Ok(())
        }

        fn prune_before_round(&self, _epoch: u64, _round: Round) -> anyhow::Result<()> {
            Ok(())
        }
    }

    fn make_manager(
        ctx: &TestContext,
        local_index: usize,
        storage: Arc<dyn SecretShareStorage>,
    ) -> (
        SecretShareManager,
        aptos_channel::Receiver<(Author, ProtocolId), PeerManagerRequest>,
    ) {
        make_manager_with_retention(ctx, local_index, storage, 0, 120)
    }

    fn make_manager_with_retention(
        ctx: &TestContext,
        local_index: usize,
        storage: Arc<dyn SecretShareStorage>,
        highest_committed_round: Round,
        retention_rounds: Round,
    ) -> (
        SecretShareManager,
        aptos_channel::Receiver<(Author, ProtocolId), PeerManagerRequest>,
    ) {
        let peers_and_metadata = PeersAndMetadata::new(&[NetworkId::Validator]);
        for peer in &ctx.authors {
            let peer_network_id = PeerNetworkId::new(NetworkId::Validator, *peer);
            let mut connection_metadata = ConnectionMetadata::mock(*peer);
            connection_metadata.application_protocols = ProtocolIdSet::from_iter(DIRECT_SEND);
            peers_and_metadata
                .insert_connection_metadata(peer_network_id, connection_metadata)
                .unwrap();
        }

        let (network_reqs_tx, network_reqs_rx) = aptos_channel::new(QueueStyle::FIFO, 16, None);
        let (connection_reqs_tx, _) = aptos_channel::new(QueueStyle::FIFO, 16, None);
        let network_sender = aptos_network::protocols::network::NetworkSender::new(
            PeerManagerRequestSender::new(network_reqs_tx),
            ConnectionRequestSender::new(connection_reqs_tx),
        );
        let network_client = NetworkClient::new(
            DIRECT_SEND.into(),
            RPC.into(),
            hashmap! { NetworkId::Validator => network_sender },
            peers_and_metadata,
        );
        let consensus_network_client = ConsensusNetworkClient::new(network_client);
        let (self_sender, _) = aptos_channels::new_unbounded_test();
        let network_sender = Arc::new(NetworkSender::new(
            ctx.authors[local_index],
            consensus_network_client,
            self_sender,
            ctx.validator_verifier.clone(),
        ));
        let (outgoing_blocks, _) = unbounded();
        let bounded_executor = BoundedExecutor::new(4, tokio::runtime::Handle::current());
        let manager = SecretShareManager::new(
            ctx.authors[local_index],
            Arc::new(EpochState {
                epoch: ctx.epoch,
                verifier: ctx.validator_verifier.clone(),
            }),
            Arc::new(SecretShareVerifier::new(
                ctx.secret_share_config.clone(),
                true,
            )),
            outgoing_blocks,
            network_sender,
            storage,
            highest_committed_round,
            retention_rounds,
            bounded_executor,
            &ReliableBroadcastConfig::default(),
            1_000_000,
        );
        (manager, network_reqs_rx)
    }

    fn request_rpc(
        metadata: SecretShareMetadata,
    ) -> (SecretShareRpc, oneshot::Receiver<Result<Bytes, RpcError>>) {
        let (response_sender, response_rx) = oneshot::channel();
        (
            SecretShareRpc {
                msg: SecretShareMessage::RequestShare(RequestSecretShare::new(metadata)),
                protocol: ProtocolId::ConsensusRpcBcs,
                response_sender,
            },
            response_rx,
        )
    }

    #[tokio::test]
    async fn test_write_completes_before_cache_and_broadcast() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let storage = Arc::new(InMemorySecretShareStorage::new());
        let (mut manager, mut network_rx) = make_manager(&ctx, 0, storage.clone());
        let metadata = create_metadata(ctx.epoch, 10);
        let share = create_secret_share(&ctx, 0, &metadata);
        manager
            .secret_share_store
            .lock()
            .update_highest_known_round(metadata.round);

        manager.process_completed_derive(metadata.round, Ok(Some(share.clone())));

        let persisted = storage
            .load_self_shares(ctx.epoch)
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
            .unwrap();
        assert_eq!(persisted.metadata(), &metadata);
        assert_eq!(
            manager
                .recovered_self_shares
                .get(&storage_key(&metadata))
                .unwrap()
                .share
                .metadata(),
            &metadata
        );
        assert!(
            !manager
                .recovered_self_shares
                .get(&storage_key(&metadata))
                .unwrap()
                .verified
        );
        assert!(manager
            .secret_share_store
            .lock()
            .get_self_share(&metadata)
            .unwrap()
            .is_some());
        assert!(network_rx.next().await.is_some());
    }

    #[tokio::test]
    async fn test_write_failure_prevents_publication() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let (mut manager, mut network_rx) = make_manager(&ctx, 0, Arc::new(FailingStorage));
        let metadata = create_metadata(ctx.epoch, 10);
        let share = create_secret_share(&ctx, 0, &metadata);
        manager
            .secret_share_store
            .lock()
            .update_highest_known_round(metadata.round);

        manager.process_completed_derive(metadata.round, Ok(Some(share)));

        assert!(manager
            .secret_share_store
            .lock()
            .get_self_share(&metadata)
            .unwrap()
            .is_none());
        assert!(manager.recovered_self_shares.is_empty());
        assert!(
            tokio::time::timeout(Duration::from_millis(10), network_rx.next())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_retention_prunes_storage_and_recovery_cache() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let storage = Arc::new(InMemorySecretShareStorage::new());
        let old_metadata = create_metadata(ctx.epoch, 10);
        let boundary_metadata = create_metadata(ctx.epoch, 20);
        storage
            .save_self_share(&create_secret_share(&ctx, 0, &old_metadata))
            .unwrap();
        storage
            .save_self_share(&create_secret_share(&ctx, 0, &boundary_metadata))
            .unwrap();

        let (manager, _) = make_manager_with_retention(&ctx, 0, storage.clone(), 30, 10);

        let recovered = storage
            .load_self_shares(ctx.epoch)
            .unwrap()
            .into_iter()
            .collect::<anyhow::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].metadata(), &boundary_metadata);
        assert!(!manager
            .recovered_self_shares
            .contains_key(&storage_key(&old_metadata)));
        assert!(manager
            .recovered_self_shares
            .contains_key(&storage_key(&boundary_metadata)));
    }

    #[tokio::test]
    async fn test_no_share_round_advances_retention() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let storage = Arc::new(InMemorySecretShareStorage::new());
        let metadata = create_metadata(ctx.epoch, 10);
        storage
            .save_self_share(&create_secret_share(&ctx, 0, &metadata))
            .unwrap();
        let (mut manager, _) = make_manager_with_retention(&ctx, 0, storage.clone(), 10, 10);

        manager.process_completed_derive(21, Ok(None));

        assert!(storage.load_self_shares(ctx.epoch).unwrap().is_empty());
        assert!(!manager
            .recovered_self_shares
            .contains_key(&storage_key(&metadata)));
    }

    #[tokio::test]
    async fn test_target_round_reset_advances_retention() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let storage = Arc::new(InMemorySecretShareStorage::new());
        let metadata = create_metadata(ctx.epoch, 10);
        storage
            .save_self_share(&create_secret_share(&ctx, 0, &metadata))
            .unwrap();
        let (mut manager, _) = make_manager_with_retention(&ctx, 0, storage.clone(), 10, 10);
        let (tx, rx) = oneshot::channel();

        manager.process_reset(ResetRequest {
            tx,
            signal: ResetSignal::TargetRound(21),
        });
        rx.await.unwrap();

        assert!(storage.load_self_shares(ctx.epoch).unwrap().is_empty());
        assert!(!manager
            .recovered_self_shares
            .contains_key(&storage_key(&metadata)));
    }

    #[tokio::test]
    async fn test_reset_preserves_recovered_self_shares() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let storage = Arc::new(InMemorySecretShareStorage::new());
        let (mut manager, mut network_rx) = make_manager(&ctx, 0, storage.clone());
        let metadata = create_metadata(ctx.epoch, 10);
        let share = create_secret_share(&ctx, 0, &metadata);
        manager
            .secret_share_store
            .lock()
            .update_highest_known_round(metadata.round);
        manager.process_completed_derive(metadata.round, Ok(Some(share)));
        assert!(network_rx.next().await.is_some());

        let (tx, rx) = oneshot::channel();
        manager.process_reset(ResetRequest {
            tx,
            signal: ResetSignal::TargetRound(metadata.round),
        });
        rx.await.unwrap();

        assert_eq!(manager.recovered_self_shares.len(), 1);
        assert!(manager
            .secret_share_store
            .lock()
            .get_self_share(&metadata)
            .unwrap()
            .is_none());

        let (request, response) = request_rpc(metadata.clone());
        manager.handle_incoming_msg(request);
        assert!(response.await.unwrap().is_ok());
        assert!(
            manager
                .recovered_self_shares
                .get(&storage_key(&metadata))
                .unwrap()
                .verified
        );
    }

    #[tokio::test]
    async fn test_cache_miss_uses_preloaded_share() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let storage = Arc::new(InMemorySecretShareStorage::new());
        let metadata = create_metadata(ctx.epoch, 10);
        let share = create_secret_share(&ctx, 0, &metadata);
        storage.save_self_share(&share).unwrap();
        let (mut manager, _) = make_manager(&ctx, 0, storage.clone());
        manager
            .secret_share_store
            .lock()
            .update_highest_known_round(metadata.round);
        assert!(
            !manager
                .recovered_self_shares
                .get(&storage_key(&metadata))
                .unwrap()
                .verified
        );
        let (request_1, response_1) = request_rpc(metadata.clone());
        manager.handle_incoming_msg(request_1);
        assert!(
            manager
                .recovered_self_shares
                .get(&storage_key(&metadata))
                .unwrap()
                .verified
        );

        let (request_2, response_2) = request_rpc(metadata.clone());
        manager.handle_incoming_msg(request_2);

        for response in [response_1, response_2] {
            let bytes = response.await.unwrap().unwrap();
            let message: ConsensusMsg = ProtocolId::ConsensusRpcBcs.from_bytes(&bytes).unwrap();
            let SecretShareMessage::Share(recovered) =
                SecretShareMessage::from_network_message(message).unwrap()
            else {
                panic!("expected a secret share response")
            };
            assert_eq!(recovered.metadata(), &metadata);
            assert_eq!(recovered.author(), &ctx.authors[0]);
        }
    }

    #[tokio::test]
    async fn test_invalid_recovered_records_and_requests_are_rejected() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let metadata = create_metadata(ctx.epoch, 10);

        let wrong_author_storage = Arc::new(InMemorySecretShareStorage::new());
        wrong_author_storage
            .save_self_share(&create_secret_share(&ctx, 1, &metadata))
            .unwrap();
        let (mut manager, _) = make_manager(&ctx, 0, wrong_author_storage);
        manager
            .secret_share_store
            .lock()
            .update_highest_known_round(metadata.round);
        let (request, response) = request_rpc(metadata.clone());
        manager.handle_incoming_msg(request);
        assert!(response.await.is_err());

        let bad_crypto_storage = Arc::new(InMemorySecretShareStorage::new());
        bad_crypto_storage
            .save_self_share(&create_bad_secret_share(&ctx, 0, &metadata))
            .unwrap();
        let (mut manager, _) = make_manager(&ctx, 0, bad_crypto_storage);
        manager
            .secret_share_store
            .lock()
            .update_highest_known_round(metadata.round);
        assert!(manager
            .recovered_self_shares
            .contains_key(&storage_key(&metadata)));
        let (request, response) = request_rpc(metadata.clone());
        manager.handle_incoming_msg(request);
        assert!(response.await.is_err());
        assert!(!manager
            .recovered_self_shares
            .contains_key(&storage_key(&metadata)));

        let forged_storage = Arc::new(InMemorySecretShareStorage::new());
        forged_storage
            .save_self_share(&create_secret_share(&ctx, 0, &metadata))
            .unwrap();
        let (mut manager, _) = make_manager(&ctx, 0, forged_storage.clone());
        manager
            .secret_share_store
            .lock()
            .update_highest_known_round(metadata.round);
        let mut forged_metadata = metadata.clone();
        forged_metadata.timestamp += 1;
        let (request, response) = request_rpc(forged_metadata);
        manager.handle_incoming_msg(request);
        assert!(response.await.is_err());

        let old_epoch_metadata = create_metadata(ctx.epoch - 1, metadata.round);
        let (request, response) = request_rpc(old_epoch_metadata);
        manager.handle_incoming_msg(request);
        assert!(response.await.is_err());

        let corrupt_storage = Arc::new(InMemorySecretShareStorage::new());
        corrupt_storage.insert_raw(storage_key(&metadata), vec![0xFF, 0xFF]);
        let (mut manager, _) = make_manager(&ctx, 0, corrupt_storage);
        manager
            .secret_share_store
            .lock()
            .update_highest_known_round(metadata.round);
        let (request, response) = request_rpc(metadata);
        manager.handle_incoming_msg(request);
        assert!(response.await.is_err());
    }

    #[tokio::test]
    async fn test_persisted_threshold_recovers_after_restart_and_lost_handoff() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let metadata = create_metadata(ctx.epoch, 10);
        let mut peer_paths = Vec::new();

        // These three authors formed the threshold that made the original
        // decryption possible. Persist their shares, then drop the databases to
        // model full process shutdown.
        for author_index in 0..3 {
            let path = TempPath::new();
            {
                let db = SecretShareDb::new(&path);
                db.save_self_share(&create_secret_share(&ctx, author_index, &metadata))
                    .unwrap();
            }
            peer_paths.push(path);
        }

        // The fourth validator lost its original pipeline handoff. After its
        // restart/replay, its own derivation succeeds and it requests the
        // threshold authors' shares from their freshly restarted managers.
        let target_storage = Arc::new(InMemorySecretShareStorage::new());
        let (mut target, _) = make_manager(&ctx, 3, target_storage);
        target
            .secret_share_store
            .lock()
            .update_highest_known_round(metadata.round);
        target.process_completed_derive(
            metadata.round,
            Ok(Some(create_secret_share(&ctx, 3, &metadata))),
        );

        for (author_index, path) in peer_paths.iter().enumerate() {
            let restarted_storage = Arc::new(SecretShareDb::new(path));
            let (mut restarted_peer, _) = make_manager(&ctx, author_index, restarted_storage);
            restarted_peer
                .secret_share_store
                .lock()
                .update_highest_known_round(metadata.round);
            let (request, response) = request_rpc(metadata.clone());
            restarted_peer.handle_incoming_msg(request);

            let bytes = response.await.unwrap().unwrap();
            let message: ConsensusMsg = ProtocolId::ConsensusRpcBcs.from_bytes(&bytes).unwrap();
            let SecretShareMessage::Share(recovered) =
                SecretShareMessage::from_network_message(message).unwrap()
            else {
                panic!("expected a secret share response")
            };
            target
                .secret_share_store
                .lock()
                .add_share(recovered)
                .unwrap();
        }

        let result = tokio::time::timeout(Duration::from_secs(5), target.decision_rx.next())
            .await
            .expect("timed out waiting for reconstructed key")
            .expect("secret share decision channel closed");
        match result {
            SecretShareAggregationResult::Success(key) => assert_eq!(key.metadata, metadata),
            SecretShareAggregationResult::Failure { .. } => {
                panic!("persisted threshold did not reconstruct the key")
            },
        }
    }
}
