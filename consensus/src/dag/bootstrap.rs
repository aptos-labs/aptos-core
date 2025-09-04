// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    adapter::{OrderedNotifierAdapter, TLedgerInfoProvider},
    anchor_election::{AnchorElection, CommitHistory, RoundRobinAnchorElection},
    dag_driver::DagDriver,
    dag_fetcher::{DagFetcher, DagFetcherService, FetchRequestHandler},
    dag_handler::NetworkHandler,
    dag_network::TDAGNetworkSender,
    dag_state_sync::{DagStateSynchronizer, StateSyncTrigger},
    dag_store::DagStore,
    health::{ChainHealthBackoff, HealthBackoff, PipelineLatencyBasedBackpressure, TChainHealth},
    order_rule::OrderRule,
    rb_handler::NodeBroadcastHandler,
    storage::{CommitEvent, DAGStorage},
    types::{CertifiedNodeMessage, DAGMessage},
    DAGRpcResult, ProofNotifier,
};
use crate::{
    dag::{
        adapter::{compute_initial_block_and_ledger_info, LedgerInfoProvider},
        anchor_election::{LeaderReputationAdapter, MetadataBackendAdapter},
        dag_state_sync::{SyncModeMessageHandler, SyncOutcome},
        observability::logging::{LogEvent, LogSchema},
        round_state::{AdaptiveResponsive, RoundState},
    },
    liveness::{
        leader_reputation::{ProposerAndVoterHeuristic, ReputationHeuristic},
        proposal_generator::{ChainHealthBackoffConfig, PipelineBackpressureConfig},
    },
    monitor,
    network::IncomingDAGRequest,
    payload_client::PayloadClient,
    payload_manager::TPayloadManager,
    pipeline::{buffer_manager::OrderedBlocks, execution_client::TExecutionClient},
};
use velor_bounded_executor::BoundedExecutor;
use velor_channels::{
    velor_channel::{self, Receiver},
    message_queues::QueueStyle,
};
use velor_config::config::DagConsensusConfig;
use velor_consensus_types::common::{Author, Round};
use velor_infallible::{Mutex, RwLock};
use velor_logger::{debug, info};
use velor_reliable_broadcast::{RBNetworkSender, ReliableBroadcast};
use velor_types::{
    epoch_state::EpochState,
    on_chain_config::{
        AnchorElectionMode, DagConsensusConfigV1,
        LeaderReputationType::{ProposerAndVoter, ProposerAndVoterV2},
        OnChainJWKConsensusConfig, OnChainRandomnessConfig, ProposerAndVoterConfig,
        ValidatorTxnConfig,
    },
    validator_signer::ValidatorSigner,
};
use async_trait::async_trait;
use enum_dispatch::enum_dispatch;
use futures_channel::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    oneshot,
};
use std::{fmt, ops::Deref, sync::Arc, time::Duration};
use tokio::{
    runtime::Handle,
    select,
    task::{block_in_place, JoinHandle},
};
use tokio_retry::strategy::ExponentialBackoff;

#[derive(Clone)]
struct BootstrapBaseState {
    dag_store: Arc<DagStore>,
    order_rule: Arc<Mutex<OrderRule>>,
    ledger_info_provider: Arc<dyn TLedgerInfoProvider>,
    ordered_notifier: Arc<OrderedNotifierAdapter>,
    commit_history: Arc<dyn CommitHistory>,
}

#[enum_dispatch(TDagMode)]
enum Mode {
    Active(ActiveMode),
    Sync(SyncMode),
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Mode::Active(_) => write!(f, "Active"),
            Mode::Sync(_) => write!(f, "Sync"),
        }
    }
}

#[async_trait]
#[enum_dispatch]
trait TDagMode {
    async fn run(
        self,
        dag_rpc_rx: &mut Receiver<Author, IncomingDAGRequest>,
        bootstrapper: &DagBootstrapper,
    ) -> Option<Mode>;
}

struct ActiveMode {
    handler: NetworkHandler,
    fetch_service: DagFetcherService,
    base_state: BootstrapBaseState,
    buffer: Vec<DAGMessage>,
}

#[async_trait]
impl TDagMode for ActiveMode {
    async fn run(
        self,
        dag_rpc_rx: &mut Receiver<Author, IncomingDAGRequest>,
        bootstrapper: &DagBootstrapper,
    ) -> Option<Mode> {
        monitor!(
            "dag_active_mode",
            self.run_internal(dag_rpc_rx, bootstrapper).await
        )
    }
}

impl ActiveMode {
    async fn run_internal(
        self,
        dag_rpc_rx: &mut Receiver<Author, IncomingDAGRequest>,
        bootstrapper: &DagBootstrapper,
    ) -> Option<Mode> {
        info!(
            LogSchema::new(LogEvent::ActiveMode)
                .round(self.base_state.dag_store.deref().read().highest_round()),
            highest_committed_round = self
                .base_state
                .ledger_info_provider
                .get_latest_ledger_info()
                .commit_info()
                .round(),
            highest_ordered_round = self
                .base_state
                .dag_store
                .read()
                .highest_ordered_anchor_round(),
        );

        // Spawn the fetch service
        let handle = tokio::spawn(self.fetch_service.start());
        defer!({
            // Signal and stop the fetch service
            debug!("aborting fetch service");
            handle.abort();
            let _ = block_in_place(move || Handle::current().block_on(handle));
            debug!("aborting fetch service complete");
        });

        // Run the network handler until it returns with state sync status.
        let sync_outcome = self
            .handler
            .run(dag_rpc_rx, bootstrapper.executor.clone(), self.buffer)
            .await;

        info!(
            LogSchema::new(LogEvent::SyncOutcome),
            sync_outcome = %sync_outcome,
        );

        match sync_outcome {
            SyncOutcome::NeedsSync(certified_node_msg) => Some(Mode::Sync(SyncMode {
                certified_node_msg,
                base_state: self.base_state,
            })),
            SyncOutcome::EpochEnds => None,
            _ => unreachable!(),
        }
    }
}

struct SyncMode {
    certified_node_msg: CertifiedNodeMessage,
    base_state: BootstrapBaseState,
}

#[async_trait]
impl TDagMode for SyncMode {
    async fn run(
        self,
        dag_rpc_rx: &mut Receiver<Author, IncomingDAGRequest>,
        bootstrapper: &DagBootstrapper,
    ) -> Option<Mode> {
        monitor!(
            "dag_sync_mode",
            self.run_internal(dag_rpc_rx, bootstrapper).await
        )
    }
}

impl SyncMode {
    async fn run_internal(
        self,
        dag_rpc_rx: &mut Receiver<Author, IncomingDAGRequest>,
        bootstrapper: &DagBootstrapper,
    ) -> Option<Mode> {
        let sync_manager = DagStateSynchronizer::new(
            bootstrapper.epoch_state.clone(),
            bootstrapper.time_service.clone(),
            bootstrapper.execution_client.clone(),
            bootstrapper.storage.clone(),
            bootstrapper.payload_manager.clone(),
            bootstrapper
                .onchain_config
                .dag_ordering_causal_history_window as Round,
        );

        let highest_committed_anchor_round = self
            .base_state
            .ledger_info_provider
            .get_highest_committed_anchor_round();

        info!(
            LogSchema::new(LogEvent::SyncMode)
                .round(self.base_state.dag_store.read().highest_round()),
            target_round = self.certified_node_msg.round(),
            local_ordered_round = self
                .base_state
                .dag_store
                .read()
                .highest_ordered_anchor_round(),
            local_committed_round = highest_committed_anchor_round
        );
        let dag_fetcher = DagFetcher::new(
            bootstrapper.epoch_state.clone(),
            bootstrapper.dag_network_sender.clone(),
            bootstrapper.time_service.clone(),
            bootstrapper.config.fetcher_config.clone(),
        );

        let (request, responders, sync_dag_store) = sync_manager.build_request(
            &self.certified_node_msg,
            self.base_state.dag_store.clone(),
            highest_committed_anchor_round,
        );

        let commit_li = self.certified_node_msg.ledger_info().clone();

        let network_handle = SyncModeMessageHandler::new(
            bootstrapper.epoch_state.clone(),
            request.start_round(),
            request.target_round(),
            bootstrapper
                .onchain_config
                .dag_ordering_causal_history_window as u64,
        );

        let (res_tx, res_rx) = oneshot::channel();
        let handle = tokio::spawn(async move {
            let result = sync_manager
                .sync_dag_to(dag_fetcher, request, responders, sync_dag_store, commit_li)
                .await;
            let _ = res_tx.send(result);
        });
        defer!({
            debug!("aborting dag synchronizer");
            handle.abort();
            let _ = block_in_place(move || Handle::current().block_on(handle));
            debug!("aborting dag synchronizer complete");
        });

        let mut buffer = Vec::new();

        select! {
            biased;
            res = res_rx => {
                match res {
                    Ok(sync_result) => {
                        if sync_result.is_ok() {
                            info!("sync succeeded. running full bootstrap.");
                            // If the sync task finishes successfully, we can transition to Active mode by
                            // rebootstrapping all components starting from the DAG store.
                            let (new_state, new_handler, new_fetch_service) = bootstrapper.full_bootstrap();
                            Some(Mode::Active(ActiveMode {
                                handler: new_handler,
                                fetch_service: new_fetch_service,
                                base_state: new_state,
                                buffer,
                            }))
                        } else {
                            info!("sync failed. resuming with current DAG state.");
                            // If the sync task fails, then continue the DAG in Active Mode with existing state.
                            let (new_handler, new_fetch_service) =
                                bootstrapper.bootstrap_components(&self.base_state);
                            Some(Mode::Active(ActiveMode {
                                handler: new_handler,
                                fetch_service: new_fetch_service,
                                base_state: self.base_state,
                                buffer,
                            }))
                        }
                    },
                    Err(_) => unreachable!("sender won't be dropped without sending"),
                }
            },
            res = network_handle.run(dag_rpc_rx, &mut buffer) => {
                // The network handle returns if the sender side of dag_rpc_rx closes,
                // or network handle found a future CertifiedNodeMessage to cancel the
                // current sync.
                if let Some(msg) = res {
                    Some(Mode::Sync(SyncMode {
                        certified_node_msg: msg,
                        base_state: self.base_state,
                    }))
                } else {
                    unreachable!("remote mustn't drop the network message sender until bootstrapper returns");
                }
            }
        }
    }
}

pub struct DagBootstrapper {
    self_peer: Author,
    config: DagConsensusConfig,
    onchain_config: DagConsensusConfigV1,
    signer: Arc<ValidatorSigner>,
    epoch_state: Arc<EpochState>,
    storage: Arc<dyn DAGStorage>,
    rb_network_sender: Arc<dyn RBNetworkSender<DAGMessage, DAGRpcResult>>,
    dag_network_sender: Arc<dyn TDAGNetworkSender>,
    proof_notifier: Arc<dyn ProofNotifier>,
    time_service: velor_time_service::TimeService,
    payload_manager: Arc<dyn TPayloadManager>,
    payload_client: Arc<dyn PayloadClient>,
    ordered_nodes_tx: UnboundedSender<OrderedBlocks>,
    execution_client: Arc<dyn TExecutionClient>,
    quorum_store_enabled: bool,
    vtxn_config: ValidatorTxnConfig,
    randomness_config: OnChainRandomnessConfig,
    jwk_consensus_config: OnChainJWKConsensusConfig,
    executor: BoundedExecutor,
    allow_batches_without_pos_in_proposal: bool,
}

impl DagBootstrapper {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        self_peer: Author,
        config: DagConsensusConfig,
        onchain_config: DagConsensusConfigV1,
        signer: Arc<ValidatorSigner>,
        epoch_state: Arc<EpochState>,
        storage: Arc<dyn DAGStorage>,
        rb_network_sender: Arc<dyn RBNetworkSender<DAGMessage, DAGRpcResult>>,
        dag_network_sender: Arc<dyn TDAGNetworkSender>,
        proof_notifier: Arc<dyn ProofNotifier>,
        time_service: velor_time_service::TimeService,
        payload_manager: Arc<dyn TPayloadManager>,
        payload_client: Arc<dyn PayloadClient>,
        ordered_nodes_tx: UnboundedSender<OrderedBlocks>,
        execution_client: Arc<dyn TExecutionClient>,
        quorum_store_enabled: bool,
        vtxn_config: ValidatorTxnConfig,
        randomness_config: OnChainRandomnessConfig,
        jwk_consensus_config: OnChainJWKConsensusConfig,
        executor: BoundedExecutor,
        allow_batches_without_pos_in_proposal: bool,
    ) -> Self {
        Self {
            self_peer,
            config,
            onchain_config,
            signer,
            epoch_state,
            storage,
            rb_network_sender,
            dag_network_sender,
            proof_notifier,
            time_service,
            payload_manager,
            payload_client,
            ordered_nodes_tx,
            execution_client,
            quorum_store_enabled,
            vtxn_config,
            randomness_config,
            jwk_consensus_config,
            executor,
            allow_batches_without_pos_in_proposal,
        }
    }

    fn build_leader_reputation_components(
        &self,
        config: &ProposerAndVoterConfig,
    ) -> Arc<LeaderReputationAdapter> {
        let num_validators = self.epoch_state.verifier.len();
        let epoch_to_validators_vec = self.storage.get_epoch_to_proposers();
        let epoch_to_validator_map = epoch_to_validators_vec
            .iter()
            .map(|(key, value)| {
                (
                    *key,
                    value
                        .iter()
                        .enumerate()
                        .map(|(idx, author)| (*author, idx))
                        .collect(),
                )
            })
            .collect();
        let metadata_adapter = Arc::new(MetadataBackendAdapter::new(
            num_validators
                * std::cmp::max(
                    config.proposer_window_num_validators_multiplier,
                    config.voter_window_num_validators_multiplier,
                ),
            epoch_to_validator_map,
        ));
        let heuristic: Box<dyn ReputationHeuristic> = Box::new(ProposerAndVoterHeuristic::new(
            self.self_peer,
            config.active_weight,
            config.inactive_weight,
            config.failed_weight,
            config.failure_threshold_percent,
            num_validators * config.voter_window_num_validators_multiplier,
            num_validators * config.proposer_window_num_validators_multiplier,
            false,
        ));

        let voting_power: Vec<u64> = self
            .epoch_state
            .verifier
            .get_ordered_account_addresses_iter()
            .map(|p| {
                self.epoch_state
                    .verifier
                    .get_voting_power(&p)
                    .expect("No voting power associated with AccountAddress!")
            })
            .collect();

        Arc::new(LeaderReputationAdapter::new(
            self.epoch_state.epoch,
            epoch_to_validators_vec,
            voting_power,
            metadata_adapter,
            heuristic,
            100,
        ))
    }

    fn build_anchor_election(
        &self,
    ) -> (
        Arc<dyn AnchorElection>,
        Arc<dyn CommitHistory>,
        Option<Vec<CommitEvent>>,
    ) {
        match &self.onchain_config.anchor_election_mode {
            AnchorElectionMode::RoundRobin => {
                let election = Arc::new(RoundRobinAnchorElection::new(
                    self.epoch_state.verifier.get_ordered_account_addresses(),
                ));
                (election.clone(), election, None)
            },
            AnchorElectionMode::LeaderReputation(reputation_type) => {
                let (commit_events, leader_reputation) = match reputation_type {
                    ProposerAndVoterV2(config) => {
                        let commit_events = self
                            .storage
                            .get_latest_k_committed_events(
                                std::cmp::max(
                                    config.proposer_window_num_validators_multiplier,
                                    config.voter_window_num_validators_multiplier,
                                ) as u64
                                    * self.epoch_state.verifier.len() as u64,
                            )
                            .expect("Failed to read commit events from storage");
                        (
                            commit_events,
                            self.build_leader_reputation_components(config),
                        )
                    },
                    ProposerAndVoter(_) => unreachable!("unsupported mode"),
                };

                (
                    leader_reputation.clone(),
                    leader_reputation,
                    Some(commit_events),
                )
            },
        }
    }

    fn bootstrap_dag_store(
        &self,
        anchor_election: Arc<dyn AnchorElection>,
        commit_history: Arc<dyn CommitHistory>,
        commit_events: Option<Vec<CommitEvent>>,
        dag_window_size_config: u64,
    ) -> BootstrapBaseState {
        let ledger_info_from_storage = self
            .storage
            .get_latest_ledger_info()
            .expect("latest ledger info must exist");
        let (parent_block_info, ledger_info) =
            compute_initial_block_and_ledger_info(ledger_info_from_storage);

        let ledger_info_provider = Arc::new(RwLock::new(LedgerInfoProvider::new(ledger_info)));

        let initial_ledger_info = ledger_info_provider
            .get_latest_ledger_info()
            .ledger_info()
            .clone();
        let commit_round = initial_ledger_info.round();
        let initial_round = std::cmp::max(
            1,
            initial_ledger_info
                .round()
                .saturating_sub(dag_window_size_config),
        );

        let dag = Arc::new(DagStore::new(
            self.epoch_state.clone(),
            self.storage.clone(),
            self.payload_manager.clone(),
            initial_round,
            dag_window_size_config,
        ));

        let ordered_notifier = Arc::new(OrderedNotifierAdapter::new(
            self.ordered_nodes_tx.clone(),
            dag.clone(),
            self.epoch_state.clone(),
            parent_block_info,
            ledger_info_provider.clone(),
            self.allow_batches_without_pos_in_proposal,
        ));

        let order_rule = Arc::new(Mutex::new(OrderRule::new(
            self.epoch_state.clone(),
            commit_round + 1,
            dag.clone(),
            anchor_election.clone(),
            ordered_notifier.clone(),
            self.onchain_config.dag_ordering_causal_history_window as Round,
            commit_events,
        )));

        BootstrapBaseState {
            dag_store: dag,
            order_rule,
            ledger_info_provider,
            ordered_notifier,
            commit_history,
        }
    }

    fn bootstrap_components(
        &self,
        base_state: &BootstrapBaseState,
    ) -> (NetworkHandler, DagFetcherService) {
        let validators = self.epoch_state.verifier.get_ordered_account_addresses();
        let rb_config = self.config.rb_config.clone();
        let round_state_config = self.config.round_state_config.clone();

        // A backoff policy that starts at _base_*_factor_ ms and multiplies by _base_ each iteration.
        let rb_backoff_policy = ExponentialBackoff::from_millis(rb_config.backoff_policy_base_ms)
            .factor(rb_config.backoff_policy_factor)
            .max_delay(Duration::from_millis(rb_config.backoff_policy_max_delay_ms));
        let rb = Arc::new(ReliableBroadcast::new(
            self.self_peer,
            validators.clone(),
            self.rb_network_sender.clone(),
            rb_backoff_policy,
            self.time_service.clone(),
            Duration::from_millis(rb_config.rpc_timeout_ms),
            self.executor.clone(),
        ));

        let BootstrapBaseState {
            dag_store,
            ledger_info_provider,
            order_rule,
            ordered_notifier,
            commit_history,
        } = base_state;

        let state_sync_trigger = StateSyncTrigger::new(
            self.epoch_state.clone(),
            ledger_info_provider.clone(),
            dag_store.clone(),
            self.proof_notifier.clone(),
            self.onchain_config.dag_ordering_causal_history_window as Round,
        );

        let (dag_fetcher, fetch_requester, node_fetch_waiter, certified_node_fetch_waiter) =
            DagFetcherService::new(
                self.epoch_state.clone(),
                self.dag_network_sender.clone(),
                dag_store.clone(),
                self.time_service.clone(),
                self.config.fetcher_config.clone(),
            );
        let fetch_requester = Arc::new(fetch_requester);
        let (new_round_tx, new_round_rx) = tokio::sync::mpsc::unbounded_channel();
        let round_state = RoundState::new(
            new_round_tx.clone(),
            Box::new(AdaptiveResponsive::new(
                new_round_tx,
                self.epoch_state.clone(),
                Duration::from_millis(round_state_config.adaptive_responsive_minimum_wait_time_ms),
            )),
        );

        let chain_health: Arc<dyn TChainHealth> = ChainHealthBackoff::new(
            ChainHealthBackoffConfig::new(self.config.health_config.chain_backoff_config.clone()),
            commit_history.clone(),
        );
        let pipeline_health = PipelineLatencyBasedBackpressure::new(
            Duration::from_millis(self.config.health_config.voter_pipeline_latency_limit_ms),
            PipelineBackpressureConfig::new(
                self.config
                    .health_config
                    .pipeline_backpressure_config
                    .clone(),
                // TODO: add pipeline backpressure based on execution speed to DAG config
                None,
            ),
            ordered_notifier.clone(),
        );
        let health_backoff =
            HealthBackoff::new(self.epoch_state.clone(), chain_health, pipeline_health);
        let dag_driver = DagDriver::new(
            self.self_peer,
            self.epoch_state.clone(),
            dag_store.clone(),
            self.payload_client.clone(),
            rb,
            self.time_service.clone(),
            self.storage.clone(),
            order_rule.clone(),
            fetch_requester.clone(),
            ledger_info_provider.clone(),
            round_state,
            self.onchain_config.dag_ordering_causal_history_window as Round,
            self.config.node_payload_config.clone(),
            health_backoff.clone(),
            self.quorum_store_enabled,
            self.allow_batches_without_pos_in_proposal,
        );
        let rb_handler = NodeBroadcastHandler::new(
            dag_store.clone(),
            order_rule.clone(),
            self.signer.clone(),
            self.epoch_state.clone(),
            self.storage.clone(),
            fetch_requester,
            self.config.node_payload_config.clone(),
            self.vtxn_config.clone(),
            self.randomness_config.clone(),
            self.jwk_consensus_config.clone(),
            health_backoff,
        );
        let fetch_handler = FetchRequestHandler::new(dag_store.clone(), self.epoch_state.clone());

        let dag_handler = NetworkHandler::new(
            self.epoch_state.clone(),
            rb_handler,
            dag_driver,
            fetch_handler,
            node_fetch_waiter,
            certified_node_fetch_waiter,
            state_sync_trigger,
            new_round_rx,
        );

        (dag_handler, dag_fetcher)
    }

    fn full_bootstrap(&self) -> (BootstrapBaseState, NetworkHandler, DagFetcherService) {
        let (anchor_election, commit_history, commit_events) = self.build_anchor_election();

        let base_state = self.bootstrap_dag_store(
            anchor_election.clone(),
            commit_history,
            commit_events,
            self.onchain_config.dag_ordering_causal_history_window as u64,
        );

        let (handler, fetch_service) = self.bootstrap_components(&base_state);
        (base_state, handler, fetch_service)
    }

    pub async fn start(
        self,
        mut dag_rpc_rx: Receiver<Author, IncomingDAGRequest>,
        mut shutdown_rx: oneshot::Receiver<oneshot::Sender<()>>,
    ) {
        info!(
            LogSchema::new(LogEvent::EpochStart),
            epoch = self.epoch_state.epoch,
        );

        let (base_state, handler, fetch_service) = self.full_bootstrap();

        let mut mode = Mode::Active(ActiveMode {
            handler,
            fetch_service,
            base_state,
            buffer: Vec::new(),
        });
        loop {
            select! {
                biased;
                Ok(ack_tx) = &mut shutdown_rx => {
                    let _ = ack_tx.send(());
                    info!(LogSchema::new(LogEvent::Shutdown), epoch = self.epoch_state.epoch);
                    return;
                },
                Some(next_mode) = mode.run(&mut dag_rpc_rx, &self) => {
                    info!(LogSchema::new(LogEvent::ModeTransition), next_mode = %next_mode);
                    mode = next_mode;
                }
            }
        }
    }
}

pub(super) fn bootstrap_dag_for_test(
    self_peer: Author,
    signer: ValidatorSigner,
    epoch_state: Arc<EpochState>,
    storage: Arc<dyn DAGStorage>,
    rb_network_sender: Arc<dyn RBNetworkSender<DAGMessage, DAGRpcResult>>,
    dag_network_sender: Arc<dyn TDAGNetworkSender>,
    proof_notifier: Arc<dyn ProofNotifier>,
    time_service: velor_time_service::TimeService,
    payload_manager: Arc<dyn TPayloadManager>,
    payload_client: Arc<dyn PayloadClient>,
    execution_client: Arc<dyn TExecutionClient>,
) -> (
    JoinHandle<SyncOutcome>,
    JoinHandle<()>,
    velor_channel::Sender<Author, IncomingDAGRequest>,
    UnboundedReceiver<OrderedBlocks>,
) {
    let (ordered_nodes_tx, ordered_nodes_rx) = futures_channel::mpsc::unbounded();
    let bootstraper = DagBootstrapper::new(
        self_peer,
        DagConsensusConfig::default(),
        DagConsensusConfigV1::default(),
        signer.into(),
        epoch_state.clone(),
        storage.clone(),
        rb_network_sender,
        dag_network_sender,
        proof_notifier.clone(),
        time_service,
        payload_manager,
        payload_client,
        ordered_nodes_tx,
        execution_client,
        false,
        ValidatorTxnConfig::default_enabled(),
        OnChainRandomnessConfig::default_enabled(),
        OnChainJWKConsensusConfig::default_enabled(),
        BoundedExecutor::new(2, Handle::current()),
        true,
    );

    let (_base_state, handler, fetch_service) = bootstraper.full_bootstrap();

    let (dag_rpc_tx, dag_rpc_rx) = velor_channel::new(QueueStyle::FIFO, 64, None);

    let dh_handle = tokio::spawn(async move {
        let mut dag_rpc_rx = dag_rpc_rx;
        handler
            .run(&mut dag_rpc_rx, bootstraper.executor.clone(), Vec::new())
            .await
    });
    let df_handle = tokio::spawn(fetch_service.start());

    (dh_handle, df_handle, dag_rpc_tx, ordered_nodes_rx)
}
