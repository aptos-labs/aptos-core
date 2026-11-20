// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    counters::RAND_QUEUE_SIZE,
    logging::{LogEvent, LogSchema},
    network::{IncomingRandGenRequest, NetworkSender, TConsensusMsg},
    pipeline::buffer_manager::{OrderedBlocks, ResetAck, ResetRequest, ResetSignal},
    rand::rand_gen::{
        aug_data_store::AugDataStore,
        block_queue::{BlockQueue, QueueItem},
        network_messages::{RandMessage, RpcRequest},
        rand_store::{AggregationResult, RandCheckFuture, RandStore},
        reliable_broadcast_state::{
            AugDataCertBuilder, CertifiedAugDataAckState, ShareAggregateState,
        },
        storage::interface::RandStorage,
        types::{
            CertifiedAugData, RandConfig, RequestCertifiedAugData, RequestShare, TAugmentedData,
            TShare,
        },
    },
};
use anyhow::ensure;
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::config::ReliableBroadcastConfig;
use aptos_consensus_types::common::{Author, Round};
use aptos_infallible::Mutex;
use aptos_logger::{debug, error, info, spawn_named, trace, warn};
use aptos_network::{protocols::network::RpcError, ProtocolId};
use aptos_reliable_broadcast::{DropGuard, ReliableBroadcast};
use aptos_time_service::TimeService;
use aptos_types::{
    epoch_state::EpochState,
    randomness::{FullRandMetadata, RandMetadata, Randomness},
    validator_signer::ValidatorSigner,
};
use bytes::Bytes;
use fail::fail_point;
use futures::{
    future::{AbortHandle, Abortable},
    FutureExt, StreamExt,
};
use futures_channel::{
    mpsc::{unbounded, UnboundedReceiver, UnboundedSender},
    oneshot,
};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio_retry::strategy::ExponentialBackoff;

pub type Sender<T> = UnboundedSender<T>;
pub type Receiver<T> = UnboundedReceiver<T>;

pub struct RandManager<S: TShare, D: TAugmentedData> {
    author: Author,
    epoch_state: Arc<EpochState>,
    stop: bool,
    config: RandConfig,
    reliable_broadcast: Arc<ReliableBroadcast<RandMessage<S, D>, ExponentialBackoff>>,
    network_sender: Arc<NetworkSender>,
    bounded_executor: BoundedExecutor,
    rpc_backoff_policy: ExponentialBackoff,
    rpc_timeout: Duration,

    // local channel received from rand_store for aggregation results
    decision_rx: Receiver<AggregationResult>,
    // downstream channels
    outgoing_blocks: Sender<OrderedBlocks>,
    // local state
    rand_store: Arc<Mutex<RandStore<S>>>,
    aug_data_store: AugDataStore<D>,
    block_queue: BlockQueue,

    // sender for aggregation results, shared with rand_store and skip listeners
    decision_tx: Sender<AggregationResult>,

    // Fetch peer certificates lost on restart after proactive broadcasts have completed.
    missing_apk_tx: aptos_channel::Sender<Author, Author>,
    missing_apk_rx: aptos_channel::Receiver<Author, Author>,
    cert_fetch_tx: Sender<(Author, CertifiedAugData<D>)>,
    cert_fetch_rx: Receiver<(Author, CertifiedAugData<D>)>,
    // Dropped at epoch change to cancel unfinished certificate fetches.
    cert_fetches: HashMap<Author, DropGuard>,
}

impl<S: TShare, D: TAugmentedData> RandManager<S, D> {
    pub fn new(
        author: Author,
        epoch_state: Arc<EpochState>,
        signer: Arc<ValidatorSigner>,
        config: RandConfig,
        outgoing_blocks: Sender<OrderedBlocks>,
        network_sender: Arc<NetworkSender>,
        db: Arc<dyn RandStorage<D>>,
        bounded_executor: BoundedExecutor,
        rb_config: &ReliableBroadcastConfig,
    ) -> Self {
        let rpc_backoff_policy = ExponentialBackoff::from_millis(rb_config.backoff_policy_base_ms)
            .factor(rb_config.backoff_policy_factor)
            .max_delay(Duration::from_millis(rb_config.backoff_policy_max_delay_ms));
        let rpc_timeout = Duration::from_millis(rb_config.rpc_timeout_ms);
        let reliable_broadcast = Arc::new(ReliableBroadcast::new(
            "rand_manager",
            author,
            epoch_state.verifier.get_ordered_account_addresses(),
            network_sender.clone(),
            rpc_backoff_policy.clone(),
            TimeService::real(),
            rpc_timeout,
            bounded_executor.clone(),
        ));
        let (decision_tx, decision_rx) = unbounded();
        // Keep at most one pending notification per author.
        let (missing_apk_tx, missing_apk_rx) = aptos_channel::new(QueueStyle::FIFO, 1, None);
        let (cert_fetch_tx, cert_fetch_rx) = unbounded();
        let rand_store = Arc::new(Mutex::new(RandStore::new(
            epoch_state.epoch,
            author,
            config.clone(),
            decision_tx.clone(),
        )));
        let aug_data_store = AugDataStore::new(epoch_state.epoch, signer, config.clone(), db);

        Self {
            author,
            epoch_state,
            stop: false,
            config,
            reliable_broadcast,
            network_sender,
            bounded_executor,
            rpc_backoff_policy,
            rpc_timeout,

            decision_rx,
            outgoing_blocks,

            rand_store,
            aug_data_store,
            block_queue: BlockQueue::new(),

            decision_tx,

            missing_apk_tx,
            missing_apk_rx,
            cert_fetch_tx,
            cert_fetch_rx,
            cert_fetches: HashMap::new(),
        }
    }

    fn process_incoming_blocks(&mut self, blocks: OrderedBlocks) {
        let rounds: Vec<u64> = blocks.ordered_blocks.iter().map(|b| b.round()).collect();
        debug!(rounds = rounds, "Processing incoming blocks.");

        // Create rand_check shared futures from the pipeline's has_rand_txns_fut.
        // Uses has_rand_txns_fut which resolves early (before randomness aggregation)
        // to avoid deadlock: rand_check_fut waits for randomness, but aggregation
        // waits for rand_check_fut to know if randomness is needed.
        let broadcast_handles: Vec<_> = blocks
            .ordered_blocks
            .iter()
            .map(|block| {
                let rand_check_future = block.pipeline_futs().map(|futs| {
                    let round = block.round();
                    let has_rand_txns = futs.has_rand_txns_fut.clone();
                    let decision_tx = self.decision_tx.clone();
                    let shared_future = async move {
                        let need_rand = match has_rand_txns.await {
                            Ok(need_rand) => need_rand,
                            Err(e) => {
                                warn!("has_rand_txns_fut failed for round {}: {e}, defaulting to needs_randomness=true", round);
                                true
                            },
                        };
                        // If the block doesn't need randomness, send Skip to unblock it
                        // before the share threshold is reached.
                        if !need_rand {
                            let _ = decision_tx
                                .unbounded_send(AggregationResult::Skip { round });
                        }
                        need_rand
                    }
                    .boxed()
                    .shared();
                    tokio::spawn(shared_future.clone());
                    shared_future
                });
                let metadata = FullRandMetadata::from(block.block());
                self.process_incoming_metadata(metadata, rand_check_future)
            })
            .collect();
        let queue_item = QueueItem::new(blocks, Some(broadcast_handles));
        self.block_queue.push_back(queue_item);
    }

    fn process_incoming_metadata(
        &self,
        metadata: FullRandMetadata,
        rand_check_future: Option<RandCheckFuture>,
    ) -> DropGuard {
        let self_share = S::generate(&self.config, metadata.metadata.clone());
        debug!(LogSchema::new(LogEvent::BroadcastRandShare)
            .epoch(self.epoch_state.epoch)
            .author(self.author)
            .round(metadata.round()));
        let mut rand_store = self.rand_store.lock();
        rand_store.update_highest_known_round(metadata.round());
        rand_store
            .add_share(self_share.clone())
            .expect("Add self share should succeed");

        rand_store.add_rand_metadata(metadata.clone(), rand_check_future);
        self.network_sender
            .broadcast_without_self(RandMessage::<S, D>::Share(self_share).into_network_message());
        self.spawn_aggregate_shares_task(metadata.metadata)
    }

    fn process_ready_blocks(&mut self, ready_blocks: Vec<OrderedBlocks>) {
        let rounds: Vec<u64> = ready_blocks
            .iter()
            .flat_map(|b| b.ordered_blocks.iter().map(|b3| b3.round()))
            .collect();
        fail_point!("rand_manager::process_ready_blocks", |_| {});
        debug!(rounds = rounds, "Processing rand-ready blocks.");

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
        self.rand_store.lock().reset(target_round);
        // Drain stale decision messages (Success/Skip) from previously spawned
        // aggregation tasks and shared futures to prevent them from affecting
        // new blocks that may arrive at the same rounds after reset.
        while matches!(self.decision_rx.try_next(), Ok(Some(_))) {}
        self.stop = matches!(signal, ResetSignal::Stop);
        let _ = tx.send(ResetAck::default());
    }

    fn process_randomness(&mut self, randomness: Randomness) {
        let rand = hex::encode(randomness.randomness());
        info!(
            metadata = randomness.metadata(),
            rand = rand,
            "Processing decisioned randomness."
        );
        if let Some(block) = self.block_queue.item_mut(randomness.round()) {
            block.set_randomness(randomness.round(), randomness);
        }
    }

    fn process_response(
        &self,
        protocol: ProtocolId,
        sender: oneshot::Sender<Result<Bytes, RpcError>>,
        message: RandMessage<S, D>,
    ) {
        let msg = message.into_network_message();
        let _ = sender.send(Ok(protocol
            .to_bytes(&msg)
            .expect("Message should be serializable into protocol")
            .into()));
    }

    async fn verification_task(
        epoch_state: Arc<EpochState>,
        mut incoming_rpc_request: aptos_channel::Receiver<Author, IncomingRandGenRequest>,
        verified_msg_tx: UnboundedSender<RpcRequest<S, D>>,
        rand_config: RandConfig,
        bounded_executor: BoundedExecutor,
    ) {
        while let Some(rand_gen_msg) = incoming_rpc_request.next().await {
            // Drop messages from peers that are not part of the validator set
            if !epoch_state
                .verifier
                .address_to_validator_index()
                .contains_key(&rand_gen_msg.sender)
            {
                warn!(
                    "Dropping rand gen message from {} that is not in the epoch {} validator set",
                    rand_gen_msg.sender, epoch_state.epoch,
                );
                continue;
            }

            // Process the message
            let tx = verified_msg_tx.clone();
            let epoch_state_clone = epoch_state.clone();
            let config_clone = rand_config.clone();
            bounded_executor
                .spawn_blocking(move || {
                    match bcs::from_bytes::<RandMessage<S, D>>(rand_gen_msg.req.data()) {
                        Ok(msg) => {
                            if msg
                                .verify(&epoch_state_clone, &config_clone, rand_gen_msg.sender)
                                .is_ok()
                            {
                                let _ = tx.unbounded_send(RpcRequest {
                                    req: msg,
                                    sender: rand_gen_msg.sender,
                                    protocol: rand_gen_msg.protocol,
                                    response_sender: rand_gen_msg.response_sender,
                                });
                            }
                        },
                        Err(e) => {
                            warn!("Invalid rand gen message: {}", e);
                        },
                    }
                })
                .await;
        }
    }

    fn spawn_aggregate_shares_task(&self, metadata: RandMetadata) -> DropGuard {
        let rb = self.reliable_broadcast.clone();
        let aggregate_state = Arc::new(ShareAggregateState::new(
            self.rand_store.clone(),
            metadata.clone(),
            self.config.clone(),
            self.missing_apk_tx.clone(),
        ));
        let epoch_state = self.epoch_state.clone();
        let round = metadata.round;
        let rand_store = self.rand_store.clone();
        let task = async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            let maybe_existing_shares = rand_store.lock().get_all_shares_authors(round);
            if let Some(existing_shares) = maybe_existing_shares {
                let epoch = epoch_state.epoch;
                let request = RequestShare::new(metadata.clone());
                let targets = epoch_state
                    .verifier
                    .get_ordered_account_addresses_iter()
                    .filter(|author| !existing_shares.contains(author))
                    .collect::<Vec<_>>();
                info!(
                    epoch = epoch,
                    round = round,
                    "[RandManager] Start broadcasting share request for {}",
                    targets.len(),
                );
                rb.multicast(request, aggregate_state, targets)
                    .await
                    .expect("Broadcast cannot fail");
                info!(
                    epoch = epoch,
                    round = round,
                    "[RandManager] Finish broadcasting share request",
                );
            }
        };
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(task, abort_registration));
        DropGuard::new(abort_handle)
    }

    /// Fetch a missing peer certificate with backoff until it passes validation.
    fn fetch_certified_aug_data(&mut self, author: Author) {
        if author == self.author
            || self.config.get_certified_apk(&author).is_some()
            || self.cert_fetches.contains_key(&author)
        {
            return;
        }
        let network_sender = self.network_sender.clone();
        let executor = self.bounded_executor.clone();
        let epoch_state = self.epoch_state.clone();
        let cert_fetch_tx = self.cert_fetch_tx.clone();
        let mut backoff = self.rpc_backoff_policy.clone();
        let timeout = self.rpc_timeout;
        let request = RandMessage::<S, D>::RequestCertifiedAugData(RequestCertifiedAugData::new(
            epoch_state.epoch,
        ))
        .into_network_message();
        let task = async move {
            let mut retry_delay = timeout;
            loop {
                let result = async {
                    let response = network_sender
                        .send_rpc(author, request.clone(), timeout)
                        .await?;
                    let epoch_state = epoch_state.clone();
                    executor
                        .spawn_blocking(move || {
                            let response = RandMessage::<S, D>::from_network_message(response)?;
                            let certified_data = CertifiedAugData::<D>::try_from(response)
                                .map_err(|_| {
                                    anyhow::anyhow!("Expected certified aug data response")
                                })?;
                            verify_fetched_aug_data(&certified_data, &epoch_state, author)?;
                            Ok::<_, anyhow::Error>(certified_data)
                        })
                        .await
                        .await?
                }
                .await;
                match result {
                    Ok(certified_data) => {
                        let _ = cert_fetch_tx.unbounded_send((author, certified_data));
                        break;
                    },
                    Err(e) => {
                        debug!(
                            epoch = epoch_state.epoch,
                            "[RandManager] Failed to fetch certified aug data from {}: {:#}",
                            author,
                            e
                        );
                        retry_delay = backoff.next().unwrap_or(retry_delay);
                        tokio::time::sleep(retry_delay).await;
                    },
                }
            }
        };
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        self.cert_fetches
            .insert(author, DropGuard::new(abort_handle));
        tokio::spawn(Abortable::new(task, abort_registration));
    }

    async fn broadcast_aug_data(&mut self) -> DropGuard {
        let data = self
            .aug_data_store
            .get_my_aug_data()
            .unwrap_or_else(|| D::generate(&self.config));
        // Add it synchronously to avoid race that it sends to others but panics before it persists locally.
        self.aug_data_store
            .add_aug_data(data.clone())
            .expect("Add self aug data should succeed");
        let aug_ack = AugDataCertBuilder::new(data.clone(), self.epoch_state.clone());
        let rb = self.reliable_broadcast.clone();
        let rb2 = self.reliable_broadcast.clone();
        let validators = self.epoch_state.verifier.get_ordered_account_addresses();
        let maybe_existing_certified_data = self.aug_data_store.get_my_certified_aug_data();
        let phase1 = async move {
            if let Some(certified_data) = maybe_existing_certified_data {
                info!("[RandManager] Already have certified aug data");
                return certified_data;
            }
            info!("[RandManager] Start broadcasting aug data");
            info!(LogSchema::new(LogEvent::BroadcastAugData)
                .author(*data.author())
                .epoch(data.epoch()));
            let certified_data = rb.broadcast(data, aug_ack).await.expect("cannot fail");
            info!("[RandManager] Finish broadcasting aug data");
            certified_data
        };
        let ack_state = Arc::new(CertifiedAugDataAckState::new(validators.into_iter()));
        let task = phase1.then(|certified_data| async move {
            info!(LogSchema::new(LogEvent::BroadcastCertifiedAugData)
                .author(*certified_data.author())
                .epoch(certified_data.epoch()));
            info!("[RandManager] Start broadcasting certified aug data");
            rb2.broadcast(certified_data, ack_state)
                .await
                .expect("Broadcast cannot fail");
            info!("[RandManager] Finish broadcasting certified aug data");
        });
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(task, abort_registration));
        DropGuard::new(abort_handle)
    }

    pub async fn start(
        mut self,
        mut incoming_blocks: Receiver<OrderedBlocks>,
        incoming_rpc_request: aptos_channel::Receiver<Author, IncomingRandGenRequest>,
        mut reset_rx: Receiver<ResetRequest>,
        bounded_executor: BoundedExecutor,
        highest_known_round: Round,
    ) {
        info!("RandManager started");
        let (verified_msg_tx, mut verified_msg_rx) = unbounded();
        let epoch_state = self.epoch_state.clone();
        let rand_config = self.config.clone();
        self.rand_store
            .lock()
            .update_highest_known_round(highest_known_round);
        spawn_named!(
            "rand manager verification",
            Self::verification_task(
                epoch_state,
                incoming_rpc_request,
                verified_msg_tx,
                rand_config,
                bounded_executor,
            )
        );

        let _guard = self.broadcast_aug_data().await;
        let mut interval = tokio::time::interval(Duration::from_millis(5000));
        while !self.stop {
            tokio::select! {
                Some(blocks) = incoming_blocks.next(), if self.aug_data_store.my_certified_aug_data_exists() => {
                    self.process_incoming_blocks(blocks);
                }
                Some(reset) = reset_rx.next() => {
                    while matches!(incoming_blocks.try_next(), Ok(Some(_))) {}
                    self.process_reset(reset);
                }
                Some(result) = self.decision_rx.next() => {
                    match result {
                        AggregationResult::Success { randomness, .. } => {
                            self.process_randomness(randomness);
                        },
                        AggregationResult::Skip { round, .. } => {
                            if let Some(item) = self.block_queue.item_mut(round) {
                                item.set_randomness(round, Randomness::default());
                            }
                        },
                    }
                }
                Some(request) = verified_msg_rx.next() => {
                    let RpcRequest {
                        req: rand_gen_msg,
                        sender,
                        protocol,
                        response_sender,
                    } = request;
                    match rand_gen_msg {
                        RandMessage::RequestShare(request) => {
                            let result = self.rand_store.lock().get_self_share(request.rand_metadata());
                            match result {
                                Ok(maybe_share) => {
                                    let share = maybe_share.unwrap_or_else(|| {
                                        // reproduce previous share if not found
                                        let share = S::generate(&self.config, request.rand_metadata().clone());
                                        self.rand_store.lock().add_share(share.clone()).expect("Add self share should succeed");
                                        share
                                    });
                                    self.process_response(protocol, response_sender, RandMessage::Share(share));
                                },
                                Err(e) => {
                                    warn!("[RandManager] Failed to get share: {}", e);
                                }
                            }
                        }
                        RandMessage::Share(share) => {
                            trace!(LogSchema::new(LogEvent::ReceiveProactiveRandShare)
                                .author(self.author)
                                .epoch(share.epoch())
                                .round(share.metadata().round)
                                .remote_peer(*share.author()));

                            // A share from an author whose certified aug data we lack never reaches
                            // this arm: `RandMessage::verify` runs `optimistic_verify`, which needs
                            // the apk on both of its branches, so `verification_task` drops the
                            // message before the main loop sees it. Recovery is therefore triggered
                            // from `ShareAggregateState::add`, on the `RequestShare` pull path.
                            if share.author() != &sender {
                                warn!(
                                    "[RandManager] Share author {:?} does not match sender {:?}, dropping",
                                    share.author(),
                                    sender,
                                );
                            } else if let Err(e) = self.rand_store.lock().add_share(share) {
                                warn!("[RandManager] Failed to add share: {}", e);
                            }
                        }
                        RandMessage::AugData(aug_data) => {
                            info!(LogSchema::new(LogEvent::ReceiveAugData)
                                .author(self.author)
                                .epoch(aug_data.epoch())
                                .remote_peer(*aug_data.author()));
                            match self.aug_data_store.add_aug_data(aug_data) {
                                Ok(sig) => self.process_response(protocol, response_sender, RandMessage::AugDataSignature(sig)),
                                Err(e) => {
                                    if e.to_string().contains("[AugDataStore] equivocate data") {
                                        warn!("[RandManager] Failed to add aug data: {}", e);
                                    } else {
                                        error!("[RandManager] Failed to add aug data: {}", e);
                                    }
                                },
                            }
                        }
                        RandMessage::CertifiedAugData(certified_aug_data) => {
                            info!(LogSchema::new(LogEvent::ReceiveCertifiedAugData)
                                .author(self.author)
                                .epoch(certified_aug_data.epoch())
                                .remote_peer(*certified_aug_data.author()));
                            match self.aug_data_store.add_certified_aug_data(certified_aug_data) {
                                Ok(ack) => self.process_response(protocol, response_sender, RandMessage::CertifiedAugDataAck(ack)),
                                Err(e) => error!("[RandManager] Failed to add certified aug data: {}", e),
                            }
                        }
                        RandMessage::RequestCertifiedAugData(_) => {
                            match self.aug_data_store.get_my_certified_aug_data() {
                                Some(certified_data) => self.process_response(
                                    protocol,
                                    response_sender,
                                    RandMessage::CertifiedAugData(certified_data),
                                ),
                                // Nothing to serve yet. `response_sender` is dropped at the end of
                                // this arm, which fails the requester's RPC so it retries with backoff.
                                None => debug!(
                                    "[RandManager] No certified aug data to serve to {}",
                                    sender
                                ),
                            }
                        }
                        _ => unreachable!("[RandManager] Unexpected message type after verification"),
                    }
                }
                Some(author) = self.missing_apk_rx.next() => {
                    self.fetch_certified_aug_data(author);
                }
                Some((author, certified_data)) = self.cert_fetch_rx.next() => {
                    self.cert_fetches.remove(&author);
                    match self.aug_data_store.add_certified_aug_data(certified_data) {
                        // The ack belongs to the proactive broadcast, which this exchange is not
                        // part of, so there is nobody to hand it to.
                        Ok(_ack) => info!(
                            "[RandManager] Fetched certified aug data for {}", author
                        ),
                        Err(e) => error!(
                            "[RandManager] Failed to persist fetched certified aug data for {}: {}",
                            author, e
                        ),
                    }
                }
                _ = interval.tick().fuse() => {
                    self.observe_queue();
                },
            }
            let maybe_ready_blocks = self.block_queue.dequeue_rand_ready_prefix();
            if !maybe_ready_blocks.is_empty() {
                self.process_ready_blocks(maybe_ready_blocks);
            }
        }
        info!("RandManager stopped");
    }

    pub fn observe_queue(&self) {
        let queue = &self.block_queue.queue();
        RAND_QUEUE_SIZE.set(queue.len() as i64);
    }
}

pub(super) fn verify_fetched_aug_data<D: TAugmentedData>(
    certified_data: &CertifiedAugData<D>,
    epoch_state: &EpochState,
    author: Author,
) -> anyhow::Result<()> {
    // RPC responses bypass RandMessage::verify, so check the requested epoch here.
    ensure!(
        certified_data.epoch() == epoch_state.epoch,
        "[RandManager] requested certified aug data for epoch {}, got one for epoch {}",
        epoch_state.epoch,
        certified_data.epoch()
    );
    ensure!(
        certified_data.author() == &author,
        "[RandManager] requested certified aug data from {}, got a cert for {}",
        author,
        certified_data.author()
    );
    certified_data.verify(&epoch_state.verifier)
}
