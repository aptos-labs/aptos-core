// Copyright © Aptos Foundation

use super::{
    adapter::{OrderedNotifierAdapter, TLedgerInfoProvider},
    anchor_election::AnchorElection,
    dag_driver::DagDriver,
    dag_fetcher::{DagFetcher, DagFetcherService, FetchRequestHandler},
    dag_handler::NetworkHandler,
    dag_network::TDAGNetworkSender,
    dag_state_sync::{DagStateSynchronizer, StateSyncTrigger},
    dag_store::Dag,
    order_rule::OrderRule,
    rb_handler::NodeBroadcastHandler,
    shutdown::{ShutdownHandle, ShutdownGroup},
    storage::DAGStorage,
    types::{CertifiedNodeMessage, DAGMessage},
    DAGRpcResult, ProofNotifier,
};
use crate::{
    dag::{
        adapter::{compute_initial_block_and_ledger_info, LedgerInfoProvider},
        anchor_election::{LeaderReputationAdapter, MetadataBackendAdapter},
        dag_state_sync::StateSyncStatus,
        observability::logging::{LogEvent, LogSchema},
        round_state::{AdaptiveResponsive, RoundState},
    },
    experimental::buffer_manager::OrderedBlocks,
    liveness::{
        leader_reputation::{ProposerAndVoterHeuristic, ReputationHeuristic},
        proposal_generator::ChainHealthBackoffConfig,
    },
    network::IncomingDAGRequest,
    payload_manager::PayloadManager,
    state_replication::{PayloadClient, StateComputer},
};
use aptos_channels::{
    aptos_channel::{self, Receiver},
    message_queues::QueueStyle,
};
use aptos_config::config::DagConsensusConfig;
use aptos_consensus_types::common::{Author, Round};
use aptos_infallible::RwLock;
use aptos_logger::{debug, info};
use aptos_reliable_broadcast::{RBNetworkSender, ReliableBroadcast};
use aptos_types::{
    epoch_state::EpochState, on_chain_config::DagConsensusConfigV1,
    validator_signer::ValidatorSigner,
};
use async_trait::async_trait;
use enum_dispatch::enum_dispatch;
use futures_channel::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    oneshot,
};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{select, task::JoinHandle};
use tokio_retry::strategy::ExponentialBackoff;

#[derive(Clone)]
struct BootstrapBaseState {
    dag_store: Arc<RwLock<Dag>>,
    order_rule: OrderRule,
    ledger_info_provider: Arc<dyn TLedgerInfoProvider>,
    leader_reputation_adapter: Arc<LeaderReputationAdapter>,
}

#[enum_dispatch(TDagMode)]
enum Mode {
    Active(ActiveMode),
    Sync(SyncMode),
    Exit(ExitMode),
}

#[async_trait]
#[enum_dispatch]
trait TDagMode {
    async fn run(
        self,
        dag_rpc_rx: &mut Receiver<Author, IncomingDAGRequest>,
        bootstrapper: &DagBootstrapper,
        shutdown_handle: &ShutdownGroup,
    ) -> Mode;
}

struct ActiveMode {
    handler: NetworkHandler,
    fetch_service: DagFetcherService,
    base_state: BootstrapBaseState,
}

#[async_trait]
impl TDagMode for ActiveMode {
    async fn run(
        self,
        dag_rpc_rx: &mut Receiver<Author, IncomingDAGRequest>,
        _bootstrapper: &DagBootstrapper,
        shutdown_group: &ShutdownGroup,
    ) -> Mode {
        let (shutdown_handle, shutdown) = shutdown_group.new_child();

        // Spawn the fetch service
        let handle = tokio::spawn(self.fetch_service.start(shutdown));

        // Run the network handler until it returns with state sync status.
        let sync_status = self.handler.run(dag_rpc_rx).await;

        // Signal and stop the fetch service
        shutdown_handle.shutdown().await;
        let _ = handle.await;

        match sync_status {
            StateSyncStatus::NeedsSync(certified_node_msg) => Mode::Sync(SyncMode {
                certified_node_msg,
                base_state: self.base_state,
            }),
            StateSyncStatus::EpochEnds => Mode::Exit(ExitMode {}),
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
        _dag_rpc_rx: &mut Receiver<Author, IncomingDAGRequest>,
        bootstrapper: &DagBootstrapper,
        _shutdown_handle: &ShutdownGroup,
    ) -> Mode {
        let sync_manager = DagStateSynchronizer::new(
            bootstrapper.epoch_state.clone(),
            bootstrapper.time_service.clone(),
            bootstrapper.state_computer.clone(),
            bootstrapper.storage.clone(),
            bootstrapper
                .onchain_config
                .dag_ordering_causal_history_window as Round,
        );

        let highest_committed_anchor_round = self
            .base_state
            .ledger_info_provider
            .get_highest_committed_anchor_round();
        debug!(
            LogSchema::new(LogEvent::StateSync)
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

        let success = match sync_manager
            .sync_dag_to(
                &self.certified_node_msg,
                dag_fetcher,
                self.base_state.dag_store.clone(),
                highest_committed_anchor_round,
            )
            .await
        {
            Ok(_) => {
                info!("sync success. going to rebootstrap.");
                true
            },
            Err(_) => {
                info!("sync failed. continuing without advancing.");
                false
            },
        };

        if success {
            let (new_state, new_handler, new_fetch_service) = bootstrapper.full_bootstrap();
            Mode::Active(ActiveMode {
                handler: new_handler,
                fetch_service: new_fetch_service,
                base_state: new_state,
            })
        } else {
            let (new_handler, new_fetch_service) =
                bootstrapper.bootstrap_components(&self.base_state);
            Mode::Active(ActiveMode {
                handler: new_handler,
                fetch_service: new_fetch_service,
                base_state: self.base_state,
            })
        }
    }
}

struct ExitMode {}

#[async_trait]
impl TDagMode for ExitMode {
    async fn run(
        self,
        _dag_rpc_rx: &mut Receiver<Author, IncomingDAGRequest>,
        _bootstrapper: &DagBootstrapper,
        _shutdown_handle: &ShutdownGroup,
    ) -> Mode {
        loop {
            tokio::task::yield_now().await;
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
    time_service: aptos_time_service::TimeService,
    payload_manager: Arc<PayloadManager>,
    payload_client: Arc<dyn PayloadClient>,
    state_computer: Arc<dyn StateComputer>,
    ordered_nodes_tx: UnboundedSender<OrderedBlocks>,
}

impl DagBootstrapper {
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
        time_service: aptos_time_service::TimeService,
        payload_manager: Arc<PayloadManager>,
        payload_client: Arc<dyn PayloadClient>,
        state_computer: Arc<dyn StateComputer>,
        ordered_nodes_tx: UnboundedSender<OrderedBlocks>,
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
            state_computer,
            ordered_nodes_tx,
        }
    }

    fn build_leader_reputation_components(&self) -> Arc<LeaderReputationAdapter> {
        let num_validators = self.epoch_state.verifier.len();
        // TODO: support multiple epochs
        let metadata_adapter = Arc::new(MetadataBackendAdapter::new(
            num_validators * 10,
            HashMap::from([(
                self.epoch_state.epoch,
                self.epoch_state
                    .verifier
                    .address_to_validator_index()
                    .clone(),
            )]),
        ));
        // TODO: use onchain config
        let heuristic: Box<dyn ReputationHeuristic> = Box::new(ProposerAndVoterHeuristic::new(
            self.self_peer,
            1000,
            10,
            1,
            10,
            num_validators,
            num_validators * 10,
            false,
        ));

        let voting_power: Vec<u64> = self
            .epoch_state
            .verifier
            .get_ordered_account_addresses_iter()
            .map(|p| self.epoch_state.verifier.get_voting_power(&p).unwrap())
            .collect();
        let anchor_election = Arc::new(LeaderReputationAdapter::new(
            self.epoch_state.epoch,
            HashMap::from([(
                self.epoch_state.epoch,
                self.epoch_state.verifier.get_ordered_account_addresses(),
            )]),
            voting_power,
            metadata_adapter,
            heuristic,
            100,
            ChainHealthBackoffConfig::new(self.config.chain_backoff_config.clone()),
        ));

        anchor_election
    }

    fn bootstrap_dag_store(
        &self,
        anchor_election: Arc<dyn AnchorElection>,
        dag_window_size_config: u64,
    ) -> (Arc<RwLock<Dag>>, OrderRule, Arc<dyn TLedgerInfoProvider>) {
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

        let dag = Arc::new(RwLock::new(Dag::new(
            self.epoch_state.clone(),
            self.storage.clone(),
            initial_round,
            dag_window_size_config,
        )));

        let notifier = Arc::new(OrderedNotifierAdapter::new(
            self.ordered_nodes_tx.clone(),
            dag.clone(),
            self.epoch_state.clone(),
            parent_block_info,
            ledger_info_provider.clone(),
        ));

        let order_rule = OrderRule::new(
            self.epoch_state.clone(),
            commit_round + 1,
            dag.clone(),
            anchor_election.clone(),
            notifier,
            self.storage.clone(),
            self.onchain_config.dag_ordering_causal_history_window as Round,
        );

        (dag, order_rule, ledger_info_provider)
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
            validators.clone(),
            self.rb_network_sender.clone(),
            rb_backoff_policy,
            self.time_service.clone(),
            Duration::from_millis(rb_config.rpc_timeout_ms),
        ));

        let BootstrapBaseState {
            dag_store,
            ledger_info_provider,
            order_rule,
            leader_reputation_adapter,
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
        let (new_round_tx, new_round_rx) =
            tokio::sync::mpsc::channel(round_state_config.round_event_channel_size);
        let round_state = RoundState::new(
            new_round_tx.clone(),
            Box::new(AdaptiveResponsive::new(
                new_round_tx,
                self.epoch_state.clone(),
                Duration::from_millis(round_state_config.adaptive_responsive_minimum_wait_time_ms),
                leader_reputation_adapter.clone(),
            )),
        );

        let dag_driver = DagDriver::new(
            self.self_peer,
            self.epoch_state.clone(),
            dag_store.clone(),
            self.payload_manager.clone(),
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
            leader_reputation_adapter.clone(),
        );
        let rb_handler = NodeBroadcastHandler::new(
            dag_store.clone(),
            self.signer.clone(),
            self.epoch_state.clone(),
            self.storage.clone(),
            fetch_requester,
            self.config.node_payload_config.clone(),
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
        let leader_reputation_adapter = self.build_leader_reputation_components();

        let (dag_store, order_rule, ledger_info_provider) = self.bootstrap_dag_store(
            leader_reputation_adapter.clone(),
            self.onchain_config.dag_ordering_causal_history_window as u64,
        );

        let base_state = BootstrapBaseState {
            dag_store,
            order_rule,
            ledger_info_provider,
            leader_reputation_adapter,
        };

        let (handler, fetch_service) = self.bootstrap_components(&base_state);
        (base_state, handler, fetch_service)
    }

    pub async fn start(
        self,
        mut dag_rpc_rx: Receiver<Author, IncomingDAGRequest>,
        mut shutdown_rx: oneshot::Receiver<oneshot::Sender<()>>,
    ) {
        let (base_state, handler, fetch_service) = self.full_bootstrap();
        let mut mode = Mode::Active(ActiveMode {
            handler,
            fetch_service,
            base_state,
        });
        let shutdown_handle = ShutdownGroup::new();
        loop {
            select! {
                biased;
                Ok(ack_tx) = &mut shutdown_rx => {
                    shutdown_handle.shutdown().await;
                    let _ = ack_tx.send(());
                    return;
                },
                next_mode = mode.run(&mut dag_rpc_rx, &self, &shutdown_handle) => {
                    mode = next_mode
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
    time_service: aptos_time_service::TimeService,
    payload_manager: Arc<PayloadManager>,
    payload_client: Arc<dyn PayloadClient>,
    state_computer: Arc<dyn StateComputer>,
) -> (
    JoinHandle<StateSyncStatus>,
    JoinHandle<()>,
    aptos_channel::Sender<Author, IncomingDAGRequest>,
    UnboundedReceiver<OrderedBlocks>,
    ShutdownGroup,
    ShutdownHandle,
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
        state_computer,
        ordered_nodes_tx,
    );

    let (_base_state, handler, fetch_service) = bootstraper.full_bootstrap();

    let (dag_rpc_tx, dag_rpc_rx) = aptos_channel::new(QueueStyle::FIFO, 64, None);

    let dh_handle = tokio::spawn(async move {
        let mut dag_rpc_rx = dag_rpc_rx;
        handler.run(&mut dag_rpc_rx).await
    });
    let root_handle = ShutdownGroup::new();
    let (child_handle, shutdown) = root_handle.new_child();
    let df_handle = tokio::spawn(fetch_service.start(shutdown));

    (
        dh_handle,
        df_handle,
        dag_rpc_tx,
        ordered_nodes_rx,
        root_handle,
        child_handle,
    )
}
