// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consensus_observer::{
        network::{
            network_handler::ConsensusObserverNetworkMessage,
            observer_client::ConsensusObserverClient, observer_message::ConsensusObserverMessage,
        },
        observer::consensus_observer::ConsensusObserver,
        publisher::consensus_publisher::ConsensusPublisher,
    },
    counters,
    epoch_manager::EpochManager,
    network::NetworkTask,
    network_interface::{ConsensusMsg, ConsensusNetworkClient},
    persistent_liveness_storage::StorageWriteProxy,
    pipeline::execution_client::{DummyExecutionClient, ExecutionProxyClient, TExecutionClient},
    quorum_store::quorum_store_db::QuorumStoreDB,
    rand::rand_gen::storage::db::RandDb,
    state_computer::ExecutionProxy,
    txn_notifier::MempoolNotifier,
    util::time_service::ClockTimeService,
};
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::aptos_channel::Receiver;
use aptos_config::config::NodeConfig;
use aptos_consensus_notifications::ConsensusNotificationSender;
use aptos_event_notifications::{DbBackedOnChainConfig, ReconfigNotificationListener};
use aptos_executor::block_executor::BlockExecutor;
use aptos_logger::prelude::*;
use aptos_mempool::QuorumStoreRequest;
use aptos_network::application::interface::{NetworkClient, NetworkServiceEvents};
use aptos_storage_interface::DbReaderWriter;
use aptos_time_service::TimeService;
use aptos_validator_transaction_pool::VTxnPoolState;
use aptos_vm::aptos_vm::AptosVMBlockExecutor;
use futures::channel::mpsc;
use move_core_types::account_address::AccountAddress;
use std::{collections::HashMap, sync::Arc};
use tokio::runtime::Runtime;

/// Helper function to start consensus based on configuration and return the runtime
#[allow(clippy::unwrap_used)]
pub fn start_consensus(
    node_config: &NodeConfig,
    network_client: NetworkClient<ConsensusMsg>,
    network_service_events: NetworkServiceEvents<ConsensusMsg>,
    state_sync_notifier: Arc<dyn ConsensusNotificationSender>,
    consensus_to_mempool_sender: mpsc::Sender<QuorumStoreRequest>,
    aptos_db: DbReaderWriter,
    reconfig_events: ReconfigNotificationListener<DbBackedOnChainConfig>,
    vtxn_pool: VTxnPoolState,
    consensus_publisher: Option<Arc<ConsensusPublisher>>,
) -> (Runtime, Arc<StorageWriteProxy>, Arc<QuorumStoreDB>) {
    let runtime = aptos_runtimes::spawn_named_runtime("consensus".into(), None);
    let storage = Arc::new(StorageWriteProxy::new(node_config, aptos_db.reader.clone()));
    let quorum_store_db = Arc::new(QuorumStoreDB::new(node_config.storage.dir()));

    let txn_notifier = Arc::new(MempoolNotifier::new(
        consensus_to_mempool_sender.clone(),
        node_config.consensus.mempool_executed_txn_timeout_ms,
    ));

    let execution_proxy = ExecutionProxy::new(
        Arc::new(BlockExecutor::<AptosVMBlockExecutor>::new(aptos_db)),
        txn_notifier,
        state_sync_notifier,
        node_config.transaction_filters.execution_filter.clone(),
        node_config.consensus.enable_pre_commit,
    );

    let time_service = Arc::new(ClockTimeService::new(runtime.handle().clone()));

    let (timeout_sender, timeout_receiver) =
        aptos_channels::new(1_024, &counters::PENDING_ROUND_TIMEOUTS);
    let (self_sender, self_receiver) =
        aptos_channels::new_unbounded(&counters::PENDING_SELF_MESSAGES);
    let consensus_network_client = ConsensusNetworkClient::new(network_client);
    let bounded_executor = BoundedExecutor::new(
        node_config.consensus.num_bounded_executor_tasks as usize,
        runtime.handle().clone(),
    );
    let rand_storage = Arc::new(RandDb::new(node_config.storage.dir()));

    let execution_client = Arc::new(ExecutionProxyClient::new(
        node_config.consensus.clone(),
        Arc::new(execution_proxy),
        node_config.validator_network.as_ref().unwrap().peer_id(),
        self_sender.clone(),
        consensus_network_client.clone(),
        bounded_executor.clone(),
        rand_storage.clone(),
        node_config.consensus_observer,
        consensus_publisher.clone(),
    ));

    let epoch_mgr = EpochManager::new(
        node_config,
        time_service,
        self_sender,
        consensus_network_client,
        timeout_sender,
        consensus_to_mempool_sender,
        execution_client,
        storage.clone(),
        quorum_store_db.clone(),
        reconfig_events,
        bounded_executor,
        aptos_time_service::TimeService::real(),
        vtxn_pool,
        rand_storage,
        consensus_publisher,
    );

    let (network_task, network_receiver) = NetworkTask::new(network_service_events, self_receiver);

    runtime.spawn(network_task.start());
    runtime.spawn(epoch_mgr.start(timeout_receiver, network_receiver));

    debug!("Consensus started.");
    (runtime, storage, quorum_store_db)
}

/// A helper function to start the consensus observer
pub fn start_consensus_observer(
    node_config: &NodeConfig,
    consensus_observer_runtime: &Runtime,
    consensus_observer_client: Arc<
        ConsensusObserverClient<NetworkClient<ConsensusObserverMessage>>,
    >,
    consensus_observer_message_receiver: Receiver<(), ConsensusObserverNetworkMessage>,
    consensus_publisher: Option<Arc<ConsensusPublisher>>,
    state_sync_notifier: Arc<dyn ConsensusNotificationSender>,
    consensus_to_mempool_sender: mpsc::Sender<QuorumStoreRequest>,
    aptos_db: DbReaderWriter,
    reconfig_events: Option<ReconfigNotificationListener<DbBackedOnChainConfig>>,
) {
    // Create the (dummy) consensus network client
    let (self_sender, _self_receiver) =
        aptos_channels::new_unbounded(&counters::PENDING_SELF_MESSAGES);
    let consensus_network_client = ConsensusNetworkClient::new(NetworkClient::new(
        vec![],
        vec![],
        HashMap::new(),
        consensus_observer_client.get_peers_and_metadata(),
    ));

    // If the consensus observer is enabled, create the execution client.
    // If not, stub it out with a dummy client.
    let execution_client = if node_config.consensus_observer.observer_enabled {
        // Create the execution proxy
        let txn_notifier = Arc::new(MempoolNotifier::new(
            consensus_to_mempool_sender.clone(),
            node_config.consensus.mempool_executed_txn_timeout_ms,
        ));
        let execution_proxy = ExecutionProxy::new(
            Arc::new(BlockExecutor::<AptosVMBlockExecutor>::new(aptos_db.clone())),
            txn_notifier,
            state_sync_notifier,
            node_config.transaction_filters.execution_filter.clone(),
            node_config.consensus.enable_pre_commit,
        );

        // Create the execution proxy client
        let bounded_executor =
            BoundedExecutor::new(32, consensus_observer_runtime.handle().clone());
        let rand_storage = Arc::new(RandDb::new(node_config.storage.dir()));
        let execution_proxy_client = Arc::new(ExecutionProxyClient::new(
            node_config.consensus.clone(),
            Arc::new(execution_proxy),
            AccountAddress::ONE,
            self_sender.clone(),
            consensus_network_client,
            bounded_executor,
            rand_storage.clone(),
            node_config.consensus_observer,
            consensus_publisher.clone(),
        ));
        execution_proxy_client as Arc<dyn TExecutionClient>
    } else {
        Arc::new(DummyExecutionClient) as Arc<dyn TExecutionClient>
    };

    // Create the consensus observer
    let (state_sync_notification_sender, state_sync_notification_listener) =
        tokio::sync::mpsc::unbounded_channel();
    let consensus_observer = ConsensusObserver::new(
        node_config.clone(),
        consensus_observer_client,
        aptos_db.reader.clone(),
        execution_client,
        state_sync_notification_sender,
        reconfig_events,
        consensus_publisher,
        TimeService::real(),
    );

    // Start the consensus observer
    consensus_observer_runtime.spawn(consensus_observer.start(
        node_config.consensus_observer,
        consensus_observer_message_receiver,
        state_sync_notification_listener,
    ));
}
