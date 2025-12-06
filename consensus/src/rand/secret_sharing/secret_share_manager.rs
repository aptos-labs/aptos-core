use crate::{
    counters::DEC_QUEUE_SIZE,
    logging::{LogEvent, LogSchema},
    network::{IncomingSecretShareRequest, NetworkSender, TConsensusMsg},
    pipeline::buffer_manager::{OrderedBlocks, ResetAck, ResetRequest, ResetSignal},
    rand::secret_sharing::{
        block_queue::{BlockQueue, QueueItem},
        network_messages::{SecretShareMessage, SecretShareRpcRequest},
        reliable_broadcast_state::SecretShareAggregateState,
        secret_share_store::SecretShareStore,
        types::RequestSecretShare,
    },
};
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::aptos_channel;
use aptos_config::config::ReliableBroadcastConfig;
use aptos_consensus_types::{
    common::{Author, Round},
    pipelined_block::PipelinedBlock,
};
use aptos_infallible::Mutex;
use aptos_logger::{error, info, spawn_named, trace, warn};
use aptos_network::{protocols::network::RpcError, ProtocolId};
use aptos_reliable_broadcast::{DropGuard, ReliableBroadcast};
use aptos_time_service::TimeService;
use aptos_types::{
    epoch_state::EpochState,
    secret_sharing::{SecretShare, SecretShareConfig, SecretShareKey, SecretShareMetadata},
    validator_signer::ValidatorSigner,
};
use bytes::Bytes;
use fail::fail_point;
use futures::{
    future::{join_all, AbortHandle, Abortable},
    FutureExt, StreamExt,
};
use futures_channel::{
    mpsc::{unbounded, UnboundedReceiver, UnboundedSender},
    oneshot,
};
use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio_retry::strategy::ExponentialBackoff;

pub type Sender<T> = UnboundedSender<T>;
pub type Receiver<T> = UnboundedReceiver<T>;

pub struct SecretShareManager {
    author: Author,
    epoch_state: Arc<EpochState>,
    stop: bool,
    config: SecretShareConfig,
    reliable_broadcast: Arc<ReliableBroadcast<SecretShareMessage, ExponentialBackoff>>,
    network_sender: Arc<NetworkSender>,

    // local channel received from dec_store
    decision_rx: Receiver<SecretShareKey>,
    // downstream channels
    outgoing_blocks: Sender<OrderedBlocks>,
    // local state
    secret_share_store: Arc<Mutex<SecretShareStore>>,
    block_queue: BlockQueue,
}

impl SecretShareManager {
    pub fn new(
        author: Author,
        epoch_state: Arc<EpochState>,
        _signer: Arc<ValidatorSigner>,
        config: SecretShareConfig,
        outgoing_blocks: Sender<OrderedBlocks>,
        network_sender: Arc<NetworkSender>,
        bounded_executor: BoundedExecutor,
        rb_config: &ReliableBroadcastConfig,
    ) -> Self {
        let rb_backoff_policy = ExponentialBackoff::from_millis(rb_config.backoff_policy_base_ms)
            .factor(rb_config.backoff_policy_factor)
            .max_delay(Duration::from_millis(rb_config.backoff_policy_max_delay_ms));
        let reliable_broadcast = Arc::new(ReliableBroadcast::new(
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
            config.clone(),
            decision_tx,
        )));

        Self {
            author,
            epoch_state,
            stop: false,
            config,
            reliable_broadcast,
            network_sender,

            decision_rx,
            outgoing_blocks,

            secret_share_store: dec_store,
            block_queue: BlockQueue::new(),
        }
    }

    async fn process_incoming_blocks(&mut self, blocks: OrderedBlocks) {
        let rounds: Vec<u64> = blocks.ordered_blocks.iter().map(|b| b.round()).collect();
        info!(rounds = rounds, "Processing incoming blocks.");

        let mut broadcast_handles = Vec::new();
        let mut encrypted_blocks_rounds = HashSet::new();
        for block in blocks.ordered_blocks.iter() {
            if let Some(handle) = self.process_incoming_block(block).await {
                broadcast_handles.push(handle);
                encrypted_blocks_rounds.insert(block.round());
            } else if let Some(tx) = block.pipeline_tx().lock().as_mut() {
                tx.secret_shared_key_tx.take().map(|tx| tx.send(None));
            } else {
                panic!(
                    "[SecretShareManager] pipeline tx is not set for block {}",
                    block.round()
                );
            }
        }

        let queue_item = QueueItem::new(blocks, Some(broadcast_handles), encrypted_blocks_rounds);
        self.block_queue.push_back(queue_item);
    }

    async fn process_incoming_block(&self, block: &PipelinedBlock) -> Option<DropGuard> {
        let self_shares = self.derive_self_shares(block).await;
        // Now acquire lock and update store
        {
            let mut secret_share_store = self.secret_share_store.lock();
            secret_share_store.update_highest_known_round(block.round());

            if let Some(secret_share) = self_shares.clone() {
                secret_share_store
                    .add_share(secret_share.clone())
                    .expect("Add self dec share should succeed");
                secret_share_store.add_secret_share_metadata(secret_share.metadata().clone());
            }
        }

        if let Some(dec_share) = self_shares {
            info!(LogSchema::new(LogEvent::BroadcastSecretShare)
                .epoch(self.epoch_state.epoch)
                .author(self.author)
                .round(block.round()));
            self.network_sender.broadcast_without_self(
                SecretShareMessage::Share(dec_share.clone()).into_network_message(),
            );
            Some(self.spawn_aggregate_shares_task(dec_share.metadata().clone()))
        } else {
            None
        }
    }

    async fn derive_self_shares(&self, block: &PipelinedBlock) -> Option<SecretShare> {
        let futures = block.pipeline_futs().unwrap();
        let share_result = futures
            .compute_decryption_share_fut
            .clone()
            .await
            .expect("Decryption share computation failed");
        if let Some(secret_share) = share_result {
            Some(secret_share)
        } else {
            None
        }
    }

    fn process_ready_blocks(&mut self, ready_blocks: Vec<OrderedBlocks>) {
        let rounds: Vec<u64> = ready_blocks
            .iter()
            .flat_map(|b| b.ordered_blocks.iter().map(|b3| b3.round()))
            .collect();
        fail_point!("dec_manager::process_ready_blocks", |_| {});
        info!(rounds = rounds, "Processing dec-ready blocks.");

        for blocks in ready_blocks {
            let _ = self.outgoing_blocks.unbounded_send(blocks);
        }
    }

    fn process_reset(&mut self, request: ResetRequest) {
        let ResetRequest { tx, signal } = request;
        let target_round = match signal {
            ResetSignal::Stop => 0,
            ResetSignal::TargetRound(round) => round,
        };
        self.block_queue = BlockQueue::new();
        self.secret_share_store
            .lock()
            .update_highest_known_round(target_round);
        self.stop = matches!(signal, ResetSignal::Stop);
        let _ = tx.send(ResetAck::default());
    }

    fn process_secret_share_key(&mut self, secret_share_key: SecretShareKey) {
        if let Some(item) = self.block_queue.item_mut(secret_share_key.metadata.round) {
            item.set_secret_shared_key(secret_share_key.metadata.round, secret_share_key);
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
        verified_msg_tx: UnboundedSender<SecretShareRpcRequest>,
        config: SecretShareConfig,
        bounded_executor: BoundedExecutor,
    ) {
        while let Some(dec_msg) = incoming_rpc_request.next().await {
            let tx = verified_msg_tx.clone();
            let epoch_state_clone = epoch_state.clone();
            let config_clone = config.clone();
            bounded_executor
                .spawn(async move {
                    match bcs::from_bytes::<SecretShareMessage>(dec_msg.req.data()) {
                        Ok(msg) => {
                            if msg.verify(&epoch_state_clone, &config_clone).is_ok() {
                                let _ = tx.unbounded_send(SecretShareRpcRequest {
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

    fn spawn_aggregate_shares_task(&self, metadata: SecretShareMetadata) -> DropGuard {
        let rb = self.reliable_broadcast.clone();
        let aggregate_state = Arc::new(SecretShareAggregateState::new(
            self.secret_share_store.clone(),
            metadata.clone(),
            self.config.clone(),
        ));
        let epoch_state = self.epoch_state.clone();
        let secret_share_store = self.secret_share_store.clone();
        let task = async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            let maybe_existing_shares = secret_share_store.lock().get_all_shares_authors(&metadata);
            if let Some(existing_shares) = maybe_existing_shares {
                let epoch = epoch_state.epoch;
                let request = RequestSecretShare::new(metadata.clone());
                let targets = epoch_state
                    .verifier
                    .get_ordered_account_addresses_iter()
                    .filter(|author| !existing_shares.contains(author))
                    .collect::<Vec<_>>();
                info!(
                    epoch = epoch,
                    round = metadata.round,
                    "[SecretShareManager] Start broadcasting share request for {}",
                    targets.len(),
                );
                rb.multicast(request, aggregate_state, targets)
                    .await
                    .expect("Broadcast cannot fail");
                info!(
                    epoch = epoch,
                    round = metadata.round,
                    "[SecretShareManager] Finish broadcasting share request",
                );
            }
        };
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(task, abort_registration));
        DropGuard::new(abort_handle)
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
        let dec_config = self.config.clone();
        {
            self.secret_share_store
                .lock()
                .update_highest_known_round(highest_known_round);
        }
        spawn_named!(
            "secret share manager verification task",
            Self::verification_task(
                epoch_state,
                incoming_rpc_request,
                verified_msg_tx,
                dec_config,
                bounded_executor,
            )
        );

        let mut interval = tokio::time::interval(Duration::from_millis(5000));
        while !self.stop {
            tokio::select! {
                Some(blocks) = incoming_blocks.next() => {
                    self.process_incoming_blocks(blocks).await;
                }
                Some(reset) = reset_rx.next() => {
                    while matches!(incoming_blocks.try_next(), Ok(Some(_))) {}
                    self.process_reset(reset);
                }
                Some(dec_key) = self.decision_rx.next() => {
                    self.process_secret_share_key(dec_key);
                }
                Some(request) = verified_msg_rx.next() => {
                    let SecretShareRpcRequest {
                        msg,
                        protocol,
                        response_sender,
                    } = request;
                    match msg {
                        SecretShareMessage::RequestShare(request) => {
                            let result = self.secret_share_store.lock().get_self_share(request.metadata());
                            match result {
                                Ok(maybe_share) => {
                                    // if the block is available
                                    if let Some(block) = self.block_queue.get_block_for_round(request.metadata().round) {
                                        let self_shares = self.derive_self_shares(block).await;
                                        if let Some(share) = self_shares {
                                            self.secret_share_store.lock().add_share(share.clone()).expect("Add self dec share should succeed");
                                        self.process_response(protocol, response_sender, SecretShareMessage::Share(share));
                                        } else {
                                            warn!("[SecretShareManager] Requesting dec share for round {} but block is not encrypted", request.metadata().round);
                                        }
                                    } else {
                                        warn!("[SecretShareManager] Block for round {} not found", request.metadata().round);
                                    }
                                },
                                Err(e) => {
                                    warn!("[SecretShareManager] Failed to get share: {}", e);
                                }
                            }
                        }
                        SecretShareMessage::Share(share) => {
                            info!(LogSchema::new(LogEvent::ReceiveProactiveSecretShare)
                                .author(self.author)
                                .epoch(share.epoch())
                                .round(share.metadata().round)
                                .remote_peer(*share.author()));

                            if let Err(e) = self.secret_share_store.lock().add_share(share) {
                                warn!("[DecManager] Failed to add share: {}", e);
                            }
                        }
                    }
                }
                _ = interval.tick().fuse() => {
                    self.observe_queue();
                },
            }
            let maybe_ready_blocks = self.block_queue.dequeue_dec_ready_prefix();
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
