// Copyright © Aptos Foundation

use super::{
    anchor_election::RoundRobinAnchorElection,
    dag_driver::DagDriver,
    dag_fetcher::{DagFetcher, FetchRequestHandler},
    dag_handler::NetworkHandler,
    dag_network::TDAGNetworkSender,
    dag_store::Dag,
    order_rule::OrderRule,
    rb_handler::NodeBroadcastHandler,
    storage::DAGStorage,
    types::DAGMessage,
    CertifiedNode,
};
use crate::{network::IncomingDAGRequest, state_replication::PayloadClient};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_consensus_types::common::Author;
use aptos_infallible::RwLock;
use aptos_reliable_broadcast::{RBNetworkSender, ReliableBroadcast};
use aptos_types::{
    epoch_state::EpochState, ledger_info::LedgerInfo, validator_signer::ValidatorSigner,
};
use futures::stream::{AbortHandle, Abortable};
use std::sync::Arc;
use tokio_retry::strategy::ExponentialBackoff;

pub fn bootstrap_dag(
    self_peer: Author,
    signer: ValidatorSigner,
    epoch_state: Arc<EpochState>,
    latest_ledger_info: LedgerInfo,
    storage: Arc<dyn DAGStorage>,
    rb_network_sender: Arc<dyn RBNetworkSender<DAGMessage>>,
    dag_network_sender: Arc<dyn TDAGNetworkSender>,
    time_service: aptos_time_service::TimeService,
    payload_client: Arc<dyn PayloadClient>,
) -> (
    AbortHandle,
    AbortHandle,
    aptos_channel::Sender<Author, IncomingDAGRequest>,
    futures_channel::mpsc::UnboundedReceiver<Vec<Arc<CertifiedNode>>>,
) {
    let validators = epoch_state.verifier.get_ordered_account_addresses();
    let current_round = latest_ledger_info.round();

    let (ordered_nodes_tx, ordered_nodes_rx) = futures_channel::mpsc::unbounded();
    let (dag_rpc_tx, dag_rpc_rx) = aptos_channel::new(QueueStyle::FIFO, 64, None);

    // A backoff policy that starts at 100ms and doubles each iteration.
    let rb_backoff_policy = ExponentialBackoff::from_millis(2).factor(50);
    let rb = Arc::new(ReliableBroadcast::new(
        validators.clone(),
        rb_network_sender,
        rb_backoff_policy,
        time_service.clone(),
    ));

    let dag = Arc::new(RwLock::new(Dag::new(epoch_state.clone(), storage.clone())));

    let anchor_election = Box::new(RoundRobinAnchorElection::new(validators));
    let order_rule = OrderRule::new(
        epoch_state.clone(),
        latest_ledger_info,
        dag.clone(),
        anchor_election,
        ordered_nodes_tx,
    );

    let (dag_fetcher, fetch_requester, node_fetch_waiter, certified_node_fetch_waiter) =
        DagFetcher::new(
            epoch_state.clone(),
            dag_network_sender,
            dag.clone(),
            time_service.clone(),
        );
    let fetch_requester = Arc::new(fetch_requester);

    let dag_driver = DagDriver::new(
        self_peer,
        epoch_state.clone(),
        dag.clone(),
        payload_client,
        rb,
        current_round,
        time_service,
        storage.clone(),
        order_rule,
        fetch_requester,
    );
    let rb_handler =
        NodeBroadcastHandler::new(dag.clone(), signer, epoch_state.clone(), storage.clone());
    let fetch_handler = FetchRequestHandler::new(dag, epoch_state.clone());

    let dag_handler = NetworkHandler::new(
        epoch_state,
        dag_rpc_rx,
        rb_handler,
        dag_driver,
        fetch_handler,
        node_fetch_waiter,
        certified_node_fetch_waiter,
    );

    let (nh_abort_handle, nh_abort_registration) = AbortHandle::new_pair();
    let (df_abort_handle, df_abort_registration) = AbortHandle::new_pair();

    tokio::spawn(Abortable::new(dag_handler.start(), nh_abort_registration));
    tokio::spawn(Abortable::new(dag_fetcher.start(), df_abort_registration));

    (
        nh_abort_handle,
        df_abort_handle,
        dag_rpc_tx,
        ordered_nodes_rx,
    )
}
