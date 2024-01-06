// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod counters;
mod dkg_manager;
pub mod epoch_manager;
pub mod network;
pub mod network_interface;
pub mod types;

use crate::{
    epoch_manager::EpochManager, network::NetworkTask, network_interface::DKGNetworkClient,
};
use aptos_config::config::NodeConfig;
use aptos_event_notifications::{
    DbBackedOnChainConfig, EventNotificationListener, ReconfigNotificationListener,
};
use aptos_network::application::interface::{NetworkClient, NetworkServiceEvents};
use tokio::runtime::Runtime;
pub use types::{DKGMessage, DKGNode};

pub fn start_dkg_runtime(
    node_config: &NodeConfig,
    network_client: NetworkClient<DKGMessage>,
    network_service_events: NetworkServiceEvents<DKGMessage>,
    reconfig_events: ReconfigNotificationListener<DbBackedOnChainConfig>,
    start_dkg_events: EventNotificationListener,
    dkg_txn_writer: aptos_validator_transaction_pool::SingleTopicWriteClient,
    dkg_pulled_rx: aptos_validator_transaction_pool::PullNotificationReceiver,
) -> Runtime {
    let runtime = aptos_runtimes::spawn_named_runtime("dkg".into(), Some(4));
    let (self_sender, self_receiver) = aptos_channels::new(1_024, &counters::PENDING_SELF_MESSAGES);
    let dkg_network_client = DKGNetworkClient::new(network_client);
    let dkg_epoch_manager = EpochManager::new(
        node_config,
        reconfig_events,
        start_dkg_events,
        self_sender,
        dkg_network_client,
        dkg_txn_writer,
        dkg_pulled_rx,
    );
    let (network_task, network_receiver) = NetworkTask::new(network_service_events, self_receiver);
    runtime.spawn(network_task.start());
    runtime.spawn(dkg_epoch_manager.start(network_receiver));
    runtime
}
