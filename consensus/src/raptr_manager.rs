// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::RAIKOU_COMMIT_NOTIFY_TO_MEMPOOL_NOTIFY,
    liveness::proposal_status_tracker::{
        ExponentialWindowFailureTracker, LockedExponentialWindowFailureTracker,
        OptQSPullParamsProvider, TOptQSPullParamsProvider,
    },
    monitor,
    network::NetworkSender,
    network_interface::ConsensusMsg,
    payload_client::PayloadClient,
    payload_manager::{QuorumStorePayloadManager, TPayloadManager},
    pipeline::buffer_manager::OrderedBlocks,
    quorum_store,
};
use anyhow::Context;
use aptos_bitvec::BitVec;
use aptos_channels::aptos_channel;
use aptos_config::config::ConsensusConfig;
use aptos_consensus_notifications::ConsensusNotificationSender;
use aptos_consensus_types::{
    block::Block,
    block_data::{BlockData, BlockType},
    common::{Author, Payload, PayloadFilter},
    payload::{InlineBatches, OptQuorumStorePayload, RaptrPayload},
    payload_pull_params::{OptQSPayloadPullParams, PayloadPullParameters},
    proof_of_store::{BatchInfo, ProofCache},
    quorum_cert::QuorumCert,
    utils::PayloadTxnsSize,
};
use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_logger::{error, info};
use aptos_types::{
    chain_id::ChainId,
    epoch_state::EpochState,
    network_address::parse_ip_tcp,
    on_chain_config::ValidatorSet,
    transaction::{
        authenticator::AccountAuthenticator, RawTransaction, Script, SignedTransaction,
        Transaction, TransactionPayload,
    },
    validator_signer::ValidatorSigner,
    validator_verifier::ValidatorVerifier,
    PeerId,
};
use aptos_validator_transaction_pool::TransactionFilter;
use futures::{executor::block_on, future::BoxFuture, FutureExt, StreamExt};
use futures_channel::{mpsc::UnboundedSender, oneshot};
use itertools::Itertools;
use move_core_types::account_address::AccountAddress;
use raptr::{
    framework::{
        injection::{delay_injection, drop_injection},
        module_network::{match_event_type, ModuleId, ModuleNetwork, ModuleNetworkService},
        network::{MessageCertifier, MessageVerifier, NetworkService},
        tcp_network::TcpNetworkService,
        timer::LocalTimerService,
        NodeId, Protocol,
    },
    metrics::{self, display_metric_to},
    raptr::{
        dissemination::{self, DisseminationLayer},
        duration_since_epoch,
        types::{self as raikou_types, Prefix, N_SUB_BLOCKS},
        RaptrNode,
    },
};
use rayon::{
    iter::ParallelIterator,
    prelude::{IndexedParallelIterator, IntoParallelIterator},
    slice::ParallelSlice,
};
use serde::{Deserialize, Serialize};
use std::{
    any::{Any, TypeId},
    collections::{BTreeMap, HashMap, HashSet},
    future::Future,
    marker::PhantomData,
    mem::Discriminant,
    net::{IpAddr, SocketAddr},
    ops::{BitOr, Deref},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, SystemTime},
};
use tokio::{net::lookup_host, time::Instant};

const CONS_BASE_PORT: u16 = 12000;
const DISS_BASE_PORT: u16 = 12500;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RaptrNetworkMessage {
    epoch: u64,
    #[serde(with = "serde_bytes")]
    data: Vec<u8>,
}
impl RaptrNetworkMessage {
    pub(crate) fn epoch(&self) -> anyhow::Result<u64> {
        Ok(self.epoch)
    }
}

pub struct RaptrManager {}

impl RaptrManager {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn run(
        self,
        self_author: Author,
        epoch_state: Arc<EpochState>,
        network_sender: Arc<NetworkSender>,
        delta: f64,
        total_duration_in_delta: u32,
        enable_optimistic_dissemination: bool,
        messages_rx: aptos_channel::Receiver<
            (AccountAddress, Discriminant<ConsensusMsg>),
            (AccountAddress, ConsensusMsg),
        >,
        diss_rx: aptos_channels::aptos_channel::Receiver<PeerId, (Author, RaptrNetworkMessage)>,
        mut shutdown_rx: oneshot::Receiver<oneshot::Sender<()>>,
        payload_client: Arc<dyn PayloadClient>,
        payload_manager: Arc<dyn TPayloadManager>,
        consensus_config: ConsensusConfig,
        validator_set: ValidatorSet,
        validator_signer: Arc<ValidatorSigner>,
        state_sync_notifier: Arc<dyn ConsensusNotificationSender>,
        proof_cache: ProofCache,
    ) {
        let n_nodes = epoch_state.verifier.len();
        let f = (n_nodes - 1) / 3;
        let poa_quorum = 2 * f + 1;
        let start_time = Instant::now();

        let timer = LocalTimerService::new();

        let address_to_index = epoch_state.verifier.address_to_validator_index().clone();
        let index_to_address = address_to_index
            .clone()
            .into_iter()
            .map(|(k, v)| (v, k))
            .collect::<HashMap<_, _>>();

        let node_id = *address_to_index.get(&self_author).unwrap();
        info!("my node id is {}", node_id);

        if validator_set.active_validators[node_id]
            .config()
            .find_ip_addr()
            .is_none()
        {
            error!("ip missing for self: {:?}", validator_set);
        }

        let signer = raptr::framework::crypto::Signer::new(
            validator_signer.clone(),
            node_id,
            N_SUB_BLOCKS + 1,
        );

        let sig_verifier = raptr::framework::crypto::SignatureVerifier::new(
            index_to_address
                .iter()
                .sorted_by_key(|(&index, _)| index)
                .map(|(_, address)| epoch_state.verifier.get_public_key(address).unwrap())
                .collect(),
            epoch_state.verifier.clone(),
            N_SUB_BLOCKS + 1,
        );

        let failures_tracker: Arc<LockedExponentialWindowFailureTracker> = Arc::new(
            Mutex::new(ExponentialWindowFailureTracker::new(
                100,
                epoch_state.verifier.get_ordered_account_addresses(),
            ))
            .into(),
        );
        let opt_qs_payload_param_provider = Arc::new(OptQSPullParamsProvider::new(
            consensus_config.quorum_store.enable_opt_quorum_store,
            consensus_config.quorum_store.opt_qs_minimum_batch_age_usecs,
            failures_tracker.clone(),
        ));

        let config = raptr::raptr::Config {
            n_nodes,
            f,
            storage_requirement: f + 1, // f + (f / 2 + 1),
            leader_timeout: Duration::from_secs_f64(delta * 4.0),
            delta: Duration::from_secs_f64(delta),
            end_of_run: Instant::now() + Duration::from_secs_f64(delta) * total_duration_in_delta,
            extra_wait_before_qc_vote: Duration::from_secs_f64(delta * 0.1),
            enable_partial_qc_votes: true,
            enable_commit_votes: true,
            status_interval: Duration::from_secs_f64(delta) * 10,
            round_sync_interval: Duration::from_secs_f64(delta) * 15,
            block_fetch_multiplicity: std::cmp::min(2, n_nodes),
            block_fetch_interval: Duration::from_secs_f64(delta) * 2,
            poa_quorum,
        };

        let mut module_network = ModuleNetwork::new();
        let diss_module_network = module_network.register().await;
        let cons_module_network = module_network.register().await;
        let diss_module_id = diss_module_network.module_id();
        let cons_module_id = cons_module_network.module_id();

        // Consensus metrics
        let mut block_consensus_latency = metrics::UnorderedBuilder::new();
        let mut batch_consensus_latency = metrics::UnorderedBuilder::new();

        // Dissemination layer metrics
        let mut batch_commit_time = metrics::UnorderedBuilder::new();
        let mut batch_execute_time = metrics::UnorderedBuilder::new();
        let mut queueing_time = metrics::UnorderedBuilder::new();
        let mut penalty_wait_time = metrics::UnorderedBuilder::new();
        let mut fetch_wait_time_after_commit = metrics::UnorderedBuilder::new();
        let executed_txns_counter = Arc::new(AtomicUsize::new(0));

        let ip_addresses = validator_set
            .active_validators
            .iter()
            .enumerate()
            .map(|(peer_id, info)| {
                let ip = info.config().find_ip_addr();
                let addr = if let Some(addr) = ip {
                    addr
                } else {
                    let dns = info.config().find_dns_name().unwrap();
                    aptos_logger::info!("Looking up IP address for peer {} ({})", peer_id, dns);

                    let addr = block_on(lookup_host((
                        dns.to_string(),
                        CONS_BASE_PORT + peer_id as u16,
                    )))
                    .expect(&format!(
                        "Failed to resolve dns for peer {} ({})",
                        peer_id, dns
                    ))
                    .next()
                    .unwrap()
                    .ip();

                    aptos_logger::info!(
                        "Resolved IP address for peer {} ({}): {}",
                        peer_id,
                        dns,
                        addr
                    );
                    addr
                };

                addr
            })
            .collect_vec();

        #[cfg(all(feature = "sim-types", not(feature = "force-aptos-types")))]
        let dissemination = Self::spawn_fake_dissemination_layer(
            node_id,
            n_nodes,
            f,
            poa_quorum,
            diss_module_network,
            delta,
            cons_module_id,
            start_time,
            &ip_addresses,
            signer.clone(),
            sig_verifier.clone(),
            enable_optimistic_dissemination,
            dissemination::Metrics {
                batch_commit_time: Some(batch_commit_time.new_sender()),
                batch_execute_time: Some(batch_execute_time.new_sender()),
                queueing_time: Some(queueing_time.new_sender()),
                penalty_wait_time: Some(penalty_wait_time.new_sender()),
                fetch_wait_time_after_commit: Some(fetch_wait_time_after_commit.new_sender()),
            },
            executed_txns_counter.clone(),
        )
        .await;

        #[cfg(any(not(feature = "sim-types"), feature = "force-aptos-types"))]
        let dissemination = Self::spawn_qs_dissemination_layer(
            node_id,
            payload_client,
            consensus_config,
            payload_manager,
            diss_module_network,
            state_sync_notifier,
            index_to_address,
            opt_qs_payload_param_provider,
        )
        .await;

        let raptr_node = Arc::new(tokio::sync::Mutex::new(RaptrNode::new(
            node_id,
            config,
            dissemination,
            true,
            raptr::raptr::Metrics {
                // propose_time: propose_time_sender,
                // enter_time: enter_time_sender,
                block_consensus_latency: Some(block_consensus_latency.new_sender()),
                batch_consensus_latency: Some(batch_consensus_latency.new_sender()),
                // indirectly_committed_slots: indirectly_committed_slots_sender,
            },
            signer.clone(),
            sig_verifier.clone(),
            // ordered_nodes_tx,
            Some(failures_tracker),
        )));

        let network_service = RaptrNetworkService::new(
            epoch_state.clone(),
            messages_rx,
            network_sender.clone(),
            Arc::new(raptr::raptr::protocol::Certifier::new()),
            Arc::new(raptr::raptr::protocol::Verifier::new(
                raptr_node.lock().await.deref(),
                proof_cache,
            )),
        )
        .await;

        // let network_service = TcpNetworkService::new(
        //     node_id,
        //     format!("0.0.0.0:{}", CONS_BASE_PORT + node_id as u16)
        //         .parse()
        //         .unwrap(),
        //     raptr::framework::tcp_network::Config {
        //         peers: ip_addresses
        //             .iter()
        //             .enumerate()
        //             .map(|(peer_id, addr)| {
        //                 format!("{}:{}", addr, CONS_BASE_PORT + peer_id as u16)
        //                     .parse()
        //                     .unwrap()
        //             })
        //             .collect(),
        //         streams_per_peer: 4,
        //     },
        //     Arc::new(raptr::raptr::protocol::Certifier::new()),
        //     Arc::new(raptr::raptr::protocol::Verifier::new(
        //         raikou_node.lock().await.deref(),
        //     )),
        //     32 * 1024 * 1024, // 32MB max block size
        // )
        // .await;

        let print_metrics = async {
            // Notify the protocol to stop.
            let module_net = module_network.register().await;
            module_net
                .notify(cons_module_id, dissemination::Kill())
                .await;

            // All data from the warmup period is discarded.
            let warmup_period_in_delta = 50;

            let mut metrics_output_buf = Vec::new();

            // Printing metrics, internally, will wait for the protocol to halt.
            display_metric_to(
                &mut metrics_output_buf,
                "Fetch wait time after commit",
                "The duration from committing a block until being able to execute it, i.e., \
                    until we have the whole prefix of the chain fetched.",
                fetch_wait_time_after_commit,
                start_time,
                delta,
                warmup_period_in_delta,
            )
            .await
            .unwrap();

            display_metric_to(
                &mut metrics_output_buf,
                "Penalty system delay",
                "The penalties for optimistically committed batches. \
                    Measured on the leader.",
                penalty_wait_time,
                start_time,
                delta,
                warmup_period_in_delta,
            )
            .await
            .unwrap();

            display_metric_to(
                &mut metrics_output_buf,
                "Optimistic batch queueing time",
                "The duration from when the batch is received by leader until the block \
                    containing this batch is proposed. \
                    Only measured if the block is committed. \
                    Only measured for optimistically committed batches. \
                    Measured on the leader.",
                queueing_time,
                start_time,
                delta,
                warmup_period_in_delta,
            )
            .await
            .unwrap();

            display_metric_to(
                &mut metrics_output_buf,
                "Batch consensus latency",
                "The duration from when the batch is included in a block until \
                    the block is committed. \
                    Measured on the leader.",
                batch_consensus_latency,
                start_time,
                delta,
                warmup_period_in_delta,
            )
            .await
            .unwrap();

            display_metric_to(
                &mut metrics_output_buf,
                "Batch commit time",
                "The duration from creating the batch until committing it. \
                    After committing, we may have to wait for the data to be fetched. \
                    Measured on the batch creator.",
                batch_commit_time,
                start_time,
                delta,
                warmup_period_in_delta,
            )
            .await
            .unwrap();

            display_metric_to(
                &mut metrics_output_buf,
                "Batch execute time (the end-to-end latency)",
                "The duration from creating the batch until executing it. \
                    Measured on the batch creator.",
                batch_execute_time,
                start_time,
                delta,
                warmup_period_in_delta,
            )
            .await
            .unwrap();

            info!(
                "Metrics: \n{}",
                std::str::from_utf8(&metrics_output_buf).unwrap(),
            );

            let executed_txns = executed_txns_counter.load(Ordering::SeqCst);
            info!(
                "Executed transactions: {} ({:.0} TPS)",
                executed_txns,
                executed_txns as f64 / (delta * total_duration_in_delta as f64)
            );
        };

        tokio::select! {
            Ok(ack_tx) = &mut shutdown_rx => {
                print_metrics.await;
                let _ = ack_tx.send(());
            },
            _ = Protocol::run(raptr_node, node_id, network_service, cons_module_network, timer) => {
                info!("run terminated");
                print_metrics.await;
            },
        }
    }

    #[cfg(any(not(feature = "sim-types"), feature = "force-aptos-types"))]
    async fn spawn_qs_dissemination_layer(
        node_id: NodeId,
        payload_client: Arc<dyn PayloadClient>,
        consensus_config: ConsensusConfig,
        payload_manager: Arc<dyn TPayloadManager>,
        mut module_network: ModuleNetworkService,
        state_sync_notifier: Arc<dyn ConsensusNotificationSender>,
        index_to_address: HashMap<usize, Author>,
        optqs_payload_param_provider: Arc<dyn TOptQSPullParamsProvider>,
    ) -> impl DisseminationLayer {
        let round_initial_timeout =
            Duration::from_millis(consensus_config.round_initial_timeout_ms);

        let dissemination = RaikouQSDisseminationLayer {
            node_id,
            payload_client,
            config: consensus_config,
            payload_manager: payload_manager.clone(),
            module_id: module_network.module_id(),
            state_sync_notifier,
            optqs_payload_param_provider,
            index_to_address: index_to_address.clone(),
        };

        tokio::spawn(async move {
            loop {
                let (consensus_module, event) = module_network.recv().await;

                if match_event_type::<dissemination::ProposalReceived>(&event) {
                    let event: Box<_> = event
                        .as_any()
                        .downcast::<dissemination::ProposalReceived>()
                        .ok()
                        .unwrap();
                    let dissemination::ProposalReceived { round, payload, .. } = *event;

                    let block_author = index_to_address.get(&payload.author()).cloned();

                    let module_network_sender = module_network.new_sender();
                    let payload_manager = payload_manager.clone();
                    tokio::spawn(async move {
                        let (prefix, _) = monitor!(
                            "payload_manager_available",
                            payload_manager.available_prefix(&payload.inner.as_raptr_payload(), 0)
                        );
                        if prefix == N_SUB_BLOCKS {
                            info!("Full prefix available {}/{}", prefix, N_SUB_BLOCKS);
                            module_network_sender
                                .notify(consensus_module, dissemination::FullBlockAvailable {
                                    round,
                                })
                                .await;
                        } else {
                            info!("Partial prefix available {}/{}", prefix, N_SUB_BLOCKS);
                            if let Ok(_) = monitor!(
                                "rm_dl_pm_wfp",
                                payload_manager
                                    .wait_for_payload(
                                        &payload.inner,
                                        block_author,
                                        // timestamp is only used for batch expiration, which is not
                                        // supported in this prototype.
                                        0,
                                        round_initial_timeout,
                                        false,
                                    )
                                    .await
                            ) {
                                module_network_sender
                                    .notify(consensus_module, dissemination::FullBlockAvailable {
                                        round,
                                    })
                                    .await;
                            }
                        }

                        monitor!(
                            "payload_manager_prefetch",
                            payload_manager.prefetch_payload_data(
                                &payload.inner,
                                block_author.unwrap(),
                                0,
                                None
                            )
                        );
                    });
                } else if match_event_type::<dissemination::NewQCWithPayload>(&event) {
                    let event: Box<_> = event
                        .as_any()
                        .downcast::<dissemination::NewQCWithPayload>()
                        .ok()
                        .unwrap();
                    let dissemination::NewQCWithPayload { payload, qc } = *event;
                    let block_author = index_to_address[&payload.author()];
                    let block_voters: BitVec = qc.signer_ids().map(|id| id as u8).collect();
                    // TODO: recheck fetching
                    monitor!(
                        "raikouman_newqc_fetch",
                        payload_manager.prefetch_payload_data(
                            &payload.inner,
                            block_author,
                            0,
                            Some(block_voters)
                        )
                    )
                } else if match_event_type::<dissemination::Kill>(&event) {
                    break;
                } else {
                    panic!("Unhandled module event: {}", event.debug_string());
                }
            }
        });

        dissemination
    }

    #[cfg(all(feature = "sim-types", not(feature = "force-aptos-types")))]
    async fn spawn_fake_dissemination_layer(
        node_id: NodeId,
        n_nodes: usize,
        f: usize,
        poa_quorum: usize,
        diss_module_network: ModuleNetworkService,
        delta: f64,
        consensus_module_id: ModuleId,
        start_time: Instant,
        ip_addresses: &Vec<IpAddr>,
        signer: raikou::framework::crypto::Signer,
        sig_verifier: raikou::framework::crypto::SignatureVerifier,
        enable_optimistic_dissemination: bool,
        metrics: dissemination::Metrics,
        executed_txns_counter: Arc<AtomicUsize>,
    ) -> impl DisseminationLayer {
        let batch_interval_secs = delta;
        let expected_load = f64::ceil(n_nodes as f64 * (3. * delta) / batch_interval_secs) as usize;

        let config = dissemination::native::Config {
            module_id: diss_module_network.module_id(),
            n_nodes,
            f,
            poa_quorum,
            delta: Duration::from_secs_f64(delta),
            batch_interval: Duration::from_secs_f64(batch_interval_secs),
            enable_optimistic_dissemination,
            // penalty tracker doesn't work with 0 delays
            enable_penalty_tracker: false,
            penalty_tracker_report_delay: Duration::from_secs_f64(delta * 5.),
            batch_fetch_multiplicity: std::cmp::min(2, n_nodes),
            batch_fetch_interval: Duration::from_secs_f64(delta) * 2,
            status_interval: Duration::from_secs_f64(delta) * 10,
            block_size_limit: dissemination::native::BlockSizeLimit::from_max_number_of_poas(
                f64::ceil(expected_load as f64 * 1.5) as usize,
                n_nodes,
            ),
        };

        let diss_timer = LocalTimerService::new();

        // TODO: make these into parameters.
        let target_tps = 100;
        let n_client_workers = 5;

        let batch_size =
            f64::ceil(target_tps as f64 * batch_interval_secs / n_nodes as f64) as usize;

        let txns_iter = run_fake_client(batch_size, n_client_workers).await;

        let (execute_tx, mut execute_rx) =
            tokio::sync::mpsc::channel::<dissemination::native::Batch>(1024);

        tokio::spawn(async move {
            while let Some(batch) = execute_rx.recv().await {
                executed_txns_counter.fetch_add(batch.txns().len(), Ordering::SeqCst);
            }
        });

        let dissemination = dissemination::native::NativeDisseminationLayer::new(
            node_id,
            config,
            txns_iter,
            consensus_module_id,
            true,
            metrics,
            signer.clone(),
            sig_verifier,
            execute_tx,
        );

        let diss_network_service = TcpNetworkService::new(
            node_id,
            format!("0.0.0.0:{}", DISS_BASE_PORT + node_id as u16)
                .parse()
                .unwrap(),
            raikou::framework::tcp_network::Config {
                peers: ip_addresses
                    .iter()
                    .enumerate()
                    .map(|(peer_id, addr)| {
                        format!("{}:{}", addr, DISS_BASE_PORT + peer_id as u16)
                            .parse()
                            .unwrap()
                    })
                    .collect(),
                streams_per_peer: 4,
            },
            Arc::new(dissemination::native::Certifier::new(signer)),
            Arc::new(dissemination::native::Verifier::new(&dissemination).await),
            1 * 1024 * 1024,
        )
        .await;

        tokio::spawn(Protocol::run(
            dissemination.protocol(),
            node_id,
            diss_network_service,
            diss_module_network,
            diss_timer,
        ));

        dissemination
    }
}

async fn run_fake_client(
    batch_size: usize,
    n_workers: usize,
) -> impl Iterator<Item = Vec<SignedTransaction>> {
    let (txns_tx, mut txns_rx) = tokio::sync::mpsc::channel(5);

    for _ in 0..n_workers {
        let txns_tx = txns_tx.clone();

        tokio::spawn(async move {
            let mut seq_num = 0;
            let sender = PeerId::random();

            loop {
                let mut txns = vec![];
                txns.reserve_exact(batch_size);

                for _ in 0..batch_size {
                    let txn = SignedTransaction::new_single_sender(
                        RawTransaction::new(
                            sender,
                            seq_num,
                            TransactionPayload::Script(Script::new(
                                Vec::new(),
                                Vec::new(),
                                Vec::new(),
                            )),
                            0,
                            0,
                            Duration::from_secs(60).as_secs(),
                            ChainId::test(),
                        ),
                        AccountAuthenticator::NoAccountAuthenticator,
                    );
                    seq_num += 1;

                    txns.push(txn);
                }

                if txns_tx.send(txns).await.is_err() {
                    // The execution ended.
                    break;
                }
            }
        });
    }

    std::iter::from_fn(move || {
        if let Ok(txns) = txns_rx.try_recv() {
            Some(txns)
        } else {
            aptos_logger::warn!("Fake client is not fast enough! Consider adding more workers.");
            None
        }
    })
}

pub struct RaikouNetworkSenderInner<M, C> {
    epoch: u64,
    n_nodes: usize,
    index_to_address: HashMap<usize, Author>,
    network_sender: Arc<NetworkSender>,
    certifier: Arc<C>,
    _phantom: PhantomData<M>,
}

impl<M, C> RaikouNetworkSenderInner<M, C>
where
    M: raptr::framework::network::NetworkMessage + Serialize + for<'de> Deserialize<'de>,
    C: MessageCertifier<Message = M>,
{
    async fn send_impl(&self, mut msg: M, targets: Option<Vec<Author>>) {
        let epoch = self.epoch;

        let aptos_network_sender = self.network_sender.clone();
        let certifier = self.certifier.clone();

        // Serialization and signing are done in a separate task to avoid blocking the main loop.
        tokio::spawn(async move {
            certifier.certify(&mut msg).await.unwrap();

            let raikou_msg = RaptrNetworkMessage {
                epoch,
                data: bcs::to_bytes(&msg).unwrap(),
            };

            let msg: ConsensusMsg = ConsensusMsg::RaptrMessage(raikou_msg);

            if let Some(targets) = targets {
                aptos_network_sender.send(msg, targets).await;
            } else {
                aptos_network_sender.broadcast(msg).await;
            }
        });
    }

    async fn send(&self, mut msg: M, targets: Vec<NodeId>) {
        let remote_peer_ids: Option<Vec<Author>> = Some(
            targets
                .into_iter()
                .map(|i| *self.index_to_address.get(&i).unwrap())
                .collect(),
        );
        self.send_impl(msg, remote_peer_ids).await;
    }

    async fn multicast(&self, msg: M) {
        self.send_impl(msg, None).await;
    }

    fn n_nodes(&self) -> usize {
        self.n_nodes
    }
}

pub struct RaptrNetworkSender<M, C> {
    inner: Arc<RaikouNetworkSenderInner<M, C>>,
}

// #[derive(Clone)] doesn't work for `M` and `C` that are not `Clone`.
impl<M, C> Clone for RaptrNetworkSender<M, C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<M, C> raptr::framework::network::NetworkSender for RaptrNetworkSender<M, C>
where
    M: raptr::framework::network::NetworkMessage + Serialize + for<'de> Deserialize<'de>,
    C: MessageCertifier<Message = M>,
{
    type Message = M;

    async fn send(&self, data: Self::Message, targets: Vec<NodeId>) {
        self.inner.send(data, targets).await;
    }

    async fn multicast(&self, data: Self::Message) {
        self.inner.multicast(data).await;
    }

    fn n_nodes(&self) -> usize {
        self.inner.n_nodes()
    }
}

pub struct RaptrNetworkService<M, C, V> {
    sender: RaptrNetworkSender<M, C>,
    deserialized_messages_rx: tokio::sync::mpsc::Receiver<(NodeId, M)>,
    _phantom: PhantomData<V>,
}

impl<M, C, V> RaptrNetworkService<M, C, V>
where
    M: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static + std::fmt::Debug,
    C: MessageCertifier<Message = M> + Send + Sync + 'static,
    V: MessageVerifier<Message = M> + Send + 'static,
{
    pub async fn new(
        epoch_state: Arc<EpochState>,
        mut messages_rx: aptos_channel::Receiver<
            (AccountAddress, Discriminant<ConsensusMsg>),
            (AccountAddress, ConsensusMsg),
        >,
        network_sender: Arc<NetworkSender>,
        certifier: Arc<C>,
        verifier: Arc<V>,
    ) -> Self {
        let address_to_index = epoch_state.verifier.address_to_validator_index().clone();
        let index_to_address = address_to_index
            .clone()
            .into_iter()
            .map(|(k, v)| (v, k))
            .collect();

        let (deserialized_messages_tx, deserialized_messages_rx) = tokio::sync::mpsc::channel(5000);

        // Spawn a separate task to deserialize messages.
        // This helps to avoid blocking the main loop.
        tokio::spawn(async move {
            loop {
                let (sender, msg) = messages_rx.select_next_some().await;
                let sender = *address_to_index.get(&sender).unwrap();
                let ConsensusMsg::RaptrMessage(msg) = msg else {
                    unreachable!()
                };

                if drop_injection() {
                    info!("APTNET: CONS: Dropping a message from {}", sender);
                    continue;
                }

                let verifier = verifier.clone();

                // Deserialize the message concurrently.
                let deserialized_messages_tx = deserialized_messages_tx.clone();
                tokio::spawn(async move {
                    let msg = monitor!("raikou_rx_deser", bcs::from_bytes(&msg.data).unwrap());

                    delay_injection().await;

                    if let Err(e) = monitor!("raikou_verify", verifier.verify(sender, &msg).await) {
                        error!("Error verifying message: {:?}", e);
                        return;
                    }

                    if deserialized_messages_tx.send((sender, msg)).await.is_err() {
                        // no-op.
                    }
                });
            }
        });

        Self {
            sender: RaptrNetworkSender {
                inner: Arc::new(RaikouNetworkSenderInner {
                    epoch: epoch_state.epoch,
                    n_nodes: epoch_state.verifier.len(),
                    index_to_address,
                    network_sender,
                    certifier,
                    _phantom: PhantomData,
                }),
            },
            deserialized_messages_rx,
            _phantom: PhantomData,
        }
    }
}

impl<M, C, V> raptr::framework::network::NetworkSender for RaptrNetworkService<M, C, V>
where
    M: raptr::framework::network::NetworkMessage + Serialize + for<'de> Deserialize<'de>,
    C: MessageCertifier<Message = M>,
    V: MessageVerifier<Message = M>,
{
    type Message = M;

    async fn send(&self, msg: Self::Message, targets: Vec<NodeId>) {
        self.sender.send(msg, targets).await;
    }

    async fn multicast(&self, data: Self::Message) {
        self.sender.multicast(data).await;
    }

    fn n_nodes(&self) -> usize {
        self.sender.n_nodes()
    }
}

impl<M, C, V> NetworkService for RaptrNetworkService<M, C, V>
where
    M: raptr::framework::network::NetworkMessage + Serialize + for<'de> Deserialize<'de>,
    C: MessageCertifier<Message = M>,
    V: MessageVerifier<Message = M>,
{
    type Sender = RaptrNetworkSender<M, C>;

    fn new_sender(&self) -> Self::Sender {
        self.sender.clone()
    }

    async fn recv(&mut self) -> (NodeId, Self::Message) {
        self.deserialized_messages_rx.recv().await.unwrap()
    }
}

#[cfg(any(feature = "force-aptos-types", not(feature = "sim-types")))]
struct RaikouQSDisseminationLayer {
    node_id: usize,
    payload_client: Arc<dyn PayloadClient>,
    config: ConsensusConfig,
    payload_manager: Arc<dyn TPayloadManager>,
    module_id: ModuleId,
    state_sync_notifier: Arc<dyn ConsensusNotificationSender>,
    optqs_payload_param_provider: Arc<dyn TOptQSPullParamsProvider>,
    index_to_address: HashMap<usize, Author>,
}

#[cfg(any(feature = "force-aptos-types", not(feature = "sim-types")))]
impl RaikouQSDisseminationLayer {}

#[cfg(any(feature = "force-aptos-types", not(feature = "sim-types")))]
impl DisseminationLayer for RaikouQSDisseminationLayer {
    fn module_id(&self) -> ModuleId {
        self.module_id
    }

    async fn prepare_block(
        &self,
        round: raikou_types::Round,
        exclude: HashSet<raikou_types::BatchInfo>,
        exclude_authors: Option<BitVec>,
    ) -> raikou_types::Payload {
        let mut optqs_params = self.optqs_payload_param_provider.get_params();
        if let Some(param) = optqs_params.as_mut() {
            if let Some(additional_exclude) = exclude_authors {
                for idx in additional_exclude.iter_ones() {
                    let author = *self.index_to_address.get(&idx).unwrap();
                    param.exclude_authors.insert(author);
                }
            }
        }

        // let optqs_params = Some(OptQSPayloadPullParams {
        //     exclude_authors: HashSet::new(),
        //     minimum_batch_age_usecs: Duration::from_millis(30).as_micros() as u64,
        // });

        // let optqs_params = self.optqs_payload_param_provider.get_params();
        let (_, payload) = self
            .payload_client
            .pull_payload(
                PayloadPullParameters {
                    max_poll_time: Duration::from_millis(self.config.quorum_store_poll_time_ms),
                    max_txns: PayloadTxnsSize::new(
                        self.config.max_sending_block_txns,
                        self.config.max_sending_block_bytes,
                    ),
                    max_txns_after_filtering: self.config.max_sending_block_txns,
                    soft_max_txns_after_filtering: self.config.max_sending_block_txns,
                    max_inline_txns: PayloadTxnsSize::new(
                        self.config.max_sending_inline_txns,
                        self.config.max_sending_inline_bytes,
                    ),
                    user_txn_filter: PayloadFilter::InQuorumStore(exclude),
                    pending_ordering: true,
                    pending_uncommitted_blocks: 0,
                    recent_max_fill_fraction: 0.0,
                    block_timestamp: aptos_infallible::duration_since_epoch(),
                    maybe_optqs_payload_pull_params: optqs_params,
                },
                TransactionFilter::no_op(),
                async {}.boxed(),
            )
            .await
            .unwrap_or_else(|e| {
                error!("pull failed {:?}", e);
                (Vec::new(), Payload::Raptr(RaptrPayload::new_empty()))
            });

        raikou_types::Payload::new(round, self.node_id, payload)
    }

    async fn available_prefix(
        &self,
        payload: &raikou_types::Payload,
        cached_value: Prefix,
    ) -> (Prefix, BitVec) {
        monitor!(
            "raikouman_dl_availprefix",
            self.payload_manager
                .available_prefix(payload.inner.as_raptr_payload(), cached_value)
        )
    }

    async fn notify_commit(
        &self,
        payloads: Vec<raikou_types::Payload>,
        block_timestamp_usecs: u64,
        voters: Option<BitVec>,
    ) {
        let payload_manager = self.payload_manager.clone();
        let state_sync_notifier = self.state_sync_notifier.clone();
        let self_peer = *self.index_to_address.get(&self.node_id).unwrap();

        tokio::spawn(async move {
            let _timer = RAIKOU_COMMIT_NOTIFY_TO_MEMPOOL_NOTIFY.start_timer();

            let payloads: Vec<Payload> =
                payloads.into_iter().map(|payload| payload.inner).collect();

            for payload in &payloads {
                quorum_store::counters::NUM_BATCH_PER_BLOCK
                    .observe(payload.as_raptr_payload().num_batches() as f64);
                quorum_store::counters::NUM_TXNS_PER_BLOCK
                    .observe(payload.as_raptr_payload().num_txns() as f64);
                for batch in payload.as_raptr_payload().get_all_batch_infos() {
                    if batch.author == self_peer {
                        let batch_create_ts = Duration::from_micros(batch.expiration)
                            .saturating_sub(Duration::from_secs(60));
                        raptr::raptr::observe_block(
                            batch_create_ts.as_micros() as u64,
                            "BATCHCOMMIT",
                        );
                        raptr::raptr::observe_block(block_timestamp_usecs, "ORIGINCOMMIT");
                    }
                }
            }

            payload_manager.notify_commit(
                aptos_infallible::duration_since_epoch()
                    .saturating_sub(Duration::from_secs(1))
                    .as_micros() as u64,
                payloads.clone(),
            );

            for payload in payloads {
                let num_txns = payload.as_raptr_payload().num_txns();

                let block = Block::new_for_dag(
                    0,
                    0,
                    0,
                    Vec::new(),
                    payload,
                    PeerId::ZERO,
                    Vec::new(),
                    HashValue::zero(),
                    BitVec::with_num_bits(8),
                    Vec::new(),
                );

                let payload_manager = payload_manager.clone();
                let state_sync_notifier = state_sync_notifier.clone();
                let voters = voters.clone();
                tokio::spawn(async move {
                    // TODO(ibalaiarun) fix authors
                    let txns_result = monitor!(
                        "raikouman_dl_nc_gt",
                        payload_manager.get_transactions(&block, voters).await
                    );
                    match  txns_result {
                        Ok((txns, _)) => {
                            assert_eq!(txns.len(), num_txns);
                            let txns = txns.into_par_iter().with_min_len(20).map(Transaction::UserTransaction).collect();
                            state_sync_notifier
                                .notify_new_commit(txns, Vec::new())
                                .await
                                .unwrap();
                        },
                        Err(_e) => unreachable!("Failed to get transactions for block {:?} even after waiting for the payload", block),
                    }
                });
            }
        });
    }

    fn check_payload(&self, payload: &raptr::raptr::types::Payload) -> Result<(), BitVec> {
        self.payload_manager
            .check_payload_availability(&payload.inner)
    }

    async fn set_first_committed_block_timestamp(&self, timestamp: SystemTime) {
        // No-op.
    }
}
