// Copyright © Aptos Foundation

use super::{
    adapter::{OrderedNotifier, OrderedNotifierAdapter, TLedgerInfoProvider},
    anchor_election::RoundRobinAnchorElection,
    dag_driver::DagDriver,
    dag_fetcher::{DagFetcher, DagFetcherService, FetchRequestHandler},
    dag_handler::NetworkHandler,
    dag_network::TDAGNetworkSender,
    dag_state_sync::{DagStateSynchronizer, StateSyncTrigger, DAG_WINDOW},
    dag_store::Dag,
    order_rule::OrderRule,
    rb_handler::NodeBroadcastHandler,
    storage::DAGStorage,
    types::DAGMessage,
    ProofNotifier,
};
use crate::{
    dag::{
        adapter::{compute_initial_block_and_ledger_info, LedgerInfoProvider},
        dag_state_sync::StateSyncStatus,
    },
    experimental::buffer_manager::OrderedBlocks,
    network::IncomingDAGRequest,
    state_replication::{PayloadClient, StateComputer},
};
use aptos_channels::{
    aptos_channel::{self, Receiver},
    message_queues::QueueStyle,
};
use aptos_consensus_types::common::{Author, Round};
use aptos_infallible::RwLock;
use aptos_logger::{debug, error};
use aptos_reliable_broadcast::{RBNetworkSender, ReliableBroadcast};
use aptos_types::{
    epoch_state::EpochState, ledger_info::LedgerInfo, validator_signer::ValidatorSigner,
};
use futures_channel::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    oneshot,
};
use std::{sync::Arc, time::Duration};
use tokio::{select, task::JoinHandle};
use tokio_retry::strategy::ExponentialBackoff;

struct DagBootstrapper {
    self_peer: Author,
    signer: Arc<ValidatorSigner>,
    epoch_state: Arc<EpochState>,
    storage: Arc<dyn DAGStorage>,
    rb_network_sender: Arc<dyn RBNetworkSender<DAGMessage>>,
    dag_network_sender: Arc<dyn TDAGNetworkSender>,
    proof_notifier: Arc<dyn ProofNotifier>,
    time_service: aptos_time_service::TimeService,
    payload_client: Arc<dyn PayloadClient>,
    state_computer: Arc<dyn StateComputer>,
}

impl DagBootstrapper {
    fn new(
        self_peer: Author,
        signer: Arc<ValidatorSigner>,
        epoch_state: Arc<EpochState>,
        storage: Arc<dyn DAGStorage>,
        rb_network_sender: Arc<dyn RBNetworkSender<DAGMessage>>,
        dag_network_sender: Arc<dyn TDAGNetworkSender>,
        proof_notifier: Arc<dyn ProofNotifier>,
        time_service: aptos_time_service::TimeService,
        payload_client: Arc<dyn PayloadClient>,
        state_computer: Arc<dyn StateComputer>,
    ) -> Self {
        Self {
            self_peer,
            signer,
            epoch_state,
            storage,
            rb_network_sender,
            dag_network_sender,
            proof_notifier,
            time_service,
            payload_client,
            state_computer,
        }
    }

    fn bootstrap_dag_store(
        &self,
        initial_ledger_info: LedgerInfo,
        notifier: Arc<dyn OrderedNotifier>,
        dag_window_size_config: usize,
    ) -> (Arc<RwLock<Dag>>, OrderRule) {
        let initial_round = if initial_ledger_info.round() <= dag_window_size_config as Round {
            1
        } else {
            initial_ledger_info
                .round()
                .saturating_sub(dag_window_size_config as Round)
        };

        let dag = Arc::new(RwLock::new(Dag::new(
            self.epoch_state.clone(),
            self.storage.clone(),
            initial_round,
            dag_window_size_config,
        )));

        let validators = self.epoch_state.verifier.get_ordered_account_addresses();
        let anchor_election = Box::new(RoundRobinAnchorElection::new(validators));

        let order_rule = OrderRule::new(
            self.epoch_state.clone(),
            initial_ledger_info,
            dag.clone(),
            anchor_election,
            notifier,
            self.storage.clone(),
        );

        (dag, order_rule)
    }

    fn bootstrap_components(
        &self,
        dag: Arc<RwLock<Dag>>,
        order_rule: OrderRule,
        state_sync_trigger: StateSyncTrigger,
        ledger_info_provider: Arc<dyn TLedgerInfoProvider>,
    ) -> (NetworkHandler, DagFetcherService) {
        let validators = self.epoch_state.verifier.get_ordered_account_addresses();

        // A backoff policy that starts at 100ms and doubles each iteration.
        let rb_backoff_policy = ExponentialBackoff::from_millis(2).factor(50);
        let rb = Arc::new(ReliableBroadcast::new(
            validators.clone(),
            self.rb_network_sender.clone(),
            rb_backoff_policy,
            self.time_service.clone(),
            // TODO: add to config
            Duration::from_millis(500),
        ));

        let (dag_fetcher, fetch_requester, node_fetch_waiter, certified_node_fetch_waiter) =
            DagFetcherService::new(
                self.epoch_state.clone(),
                self.dag_network_sender.clone(),
                dag.clone(),
                self.time_service.clone(),
            );
        let fetch_requester = Arc::new(fetch_requester);

        let dag_driver = DagDriver::new(
            self.self_peer,
            self.epoch_state.clone(),
            dag.clone(),
            self.payload_client.clone(),
            rb,
            self.time_service.clone(),
            self.storage.clone(),
            order_rule,
            fetch_requester.clone(),
            ledger_info_provider,
        );
        let rb_handler = NodeBroadcastHandler::new(
            dag.clone(),
            self.signer.clone(),
            self.epoch_state.clone(),
            self.storage.clone(),
            fetch_requester,
        );
        let fetch_handler = FetchRequestHandler::new(dag, self.epoch_state.clone());

        let dag_handler = NetworkHandler::new(
            self.epoch_state.clone(),
            rb_handler,
            dag_driver,
            fetch_handler,
            node_fetch_waiter,
            certified_node_fetch_waiter,
            state_sync_trigger,
        );

        (dag_handler, dag_fetcher)
    }

    async fn bootstrapper(
        self,
        mut dag_rpc_rx: Receiver<Author, IncomingDAGRequest>,
        ordered_nodes_tx: UnboundedSender<OrderedBlocks>,
        mut shutdown_rx: oneshot::Receiver<()>,
    ) {
        let sync_manager = DagStateSynchronizer::new(
            self.epoch_state.clone(),
            self.time_service.clone(),
            self.state_computer.clone(),
            self.storage.clone(),
        );

        loop {
            let ledger_info_from_storage = self
                .storage
                .get_latest_ledger_info()
                .expect("latest ledger info must exist");
            let (parent_block_info, ledger_info) =
                compute_initial_block_and_ledger_info(ledger_info_from_storage);
            let ledger_info_provider = Arc::new(RwLock::new(LedgerInfoProvider::new(ledger_info)));

            let adapter = Arc::new(OrderedNotifierAdapter::new(
                ordered_nodes_tx.clone(),
                self.storage.clone(),
                self.epoch_state.clone(),
                parent_block_info,
                ledger_info_provider.clone(),
            ));

            let (dag_store, order_rule) = self.bootstrap_dag_store(
                ledger_info_provider
                    .get_latest_ledger_info()
                    .ledger_info()
                    .clone(),
                adapter.clone(),
                DAG_WINDOW,
            );

            let state_sync_trigger = StateSyncTrigger::new(
                self.epoch_state.clone(),
                ledger_info_provider.clone(),
                dag_store.clone(),
                self.proof_notifier.clone(),
            );

            let (handler, fetch_service) = self.bootstrap_components(
                dag_store.clone(),
                order_rule,
                state_sync_trigger,
                ledger_info_provider.clone(),
            );

            let df_handle = tokio::spawn(fetch_service.start());

            // poll the network handler while waiting for rebootstrap notification or shutdown notification
            select! {
                biased;
                _ = &mut shutdown_rx => {
                    df_handle.abort();
                    let _ = df_handle.await;
                    return;
                },
                sync_status = handler.run(&mut dag_rpc_rx) => {
                    debug!("state sync notification received. {:?}", sync_status);
                    df_handle.abort();
                    let _ = df_handle.await;

                    match sync_status {
                        StateSyncStatus::NeedsSync(certified_node_msg) => {
                            let dag_fetcher = DagFetcher::new(self.epoch_state.clone(), self.dag_network_sender.clone(), self.time_service.clone());

                            let highest_committed_anchor_round = ledger_info_provider.get_highest_committed_anchor_round();
                            let sync_future = sync_manager.sync_dag_to(&certified_node_msg, dag_fetcher, dag_store.clone(), highest_committed_anchor_round);

                            select! {
                                Err(e) = sync_future => {
                                    error!(error = ?e, "unable to sync");
                                },
                                Ok(_) = &mut shutdown_rx => {
                                    return;
                                }
                            }

                            debug!("going to rebootstrap.");
                        },
                        StateSyncStatus::EpochEnds => {
                            // Wait for epoch manager to signal shutdown
                            _ = shutdown_rx.await;
                            return;
                        },
                        _ => unreachable!()
                    }
                }
            }
        }
    }
}

pub(super) fn bootstrap_dag_for_test(
    self_peer: Author,
    signer: ValidatorSigner,
    epoch_state: Arc<EpochState>,
    latest_ledger_info: LedgerInfo,
    storage: Arc<dyn DAGStorage>,
    rb_network_sender: Arc<dyn RBNetworkSender<DAGMessage>>,
    dag_network_sender: Arc<dyn TDAGNetworkSender>,
    proof_notifier: Arc<dyn ProofNotifier>,
    time_service: aptos_time_service::TimeService,
    payload_client: Arc<dyn PayloadClient>,
    state_computer: Arc<dyn StateComputer>,
) -> (
    JoinHandle<StateSyncStatus>,
    JoinHandle<()>,
    aptos_channel::Sender<Author, IncomingDAGRequest>,
    UnboundedReceiver<OrderedBlocks>,
) {
    let bootstraper = DagBootstrapper::new(
        self_peer,
        signer.into(),
        epoch_state.clone(),
        storage.clone(),
        rb_network_sender,
        dag_network_sender,
        proof_notifier.clone(),
        time_service,
        payload_client,
        state_computer,
    );

    let ledger_info_from_storage = storage
        .get_latest_ledger_info()
        .expect("latest ledger info must exist");
    let (parent_block_info, ledger_info) =
        compute_initial_block_and_ledger_info(ledger_info_from_storage);
    let ledger_info_provider = Arc::new(RwLock::new(LedgerInfoProvider::new(ledger_info)));

    let (ordered_nodes_tx, ordered_nodes_rx) = futures_channel::mpsc::unbounded();
    let adapter = Arc::new(OrderedNotifierAdapter::new(
        ordered_nodes_tx,
        storage.clone(),
        epoch_state.clone(),
        parent_block_info,
        ledger_info_provider.clone(),
    ));
    let (dag_rpc_tx, dag_rpc_rx) = aptos_channel::new(QueueStyle::FIFO, 64, None);

    let (dag_store, order_rule) =
        bootstraper.bootstrap_dag_store(latest_ledger_info, adapter.clone(), DAG_WINDOW);

    let state_sync_trigger = StateSyncTrigger::new(
        epoch_state,
        ledger_info_provider.clone(),
        dag_store.clone(),
        proof_notifier.clone(),
    );

    let (handler, fetch_service) = bootstraper.bootstrap_components(
        dag_store.clone(),
        order_rule,
        state_sync_trigger,
        ledger_info_provider,
    );

    let dh_handle = tokio::spawn(async move {
        let mut dag_rpc_rx = dag_rpc_rx;
        handler.run(&mut dag_rpc_rx).await
    });
    let df_handle = tokio::spawn(fetch_service.start());

    (dh_handle, df_handle, dag_rpc_tx, ordered_nodes_rx)
}
