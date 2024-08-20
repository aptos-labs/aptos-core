// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use tokio::runtime::Runtime;
use aptos_config::config::ReliableBroadcastConfig;
use aptos_event_notifications::{DbBackedOnChainConfig, EventNotificationListener, ReconfigNotificationListener};
use aptos_network::application::interface::{NetworkClient, NetworkServiceEvents};
use aptos_validator_transaction_pool::VTxnPoolState;
use move_core_types::account_address::AccountAddress;
use crate::epoch_manager::EpochManager;
use crate::network::NetworkTask;
use crate::network_interface::MPCNetworkClient;
use crate::types::MPCMessage;

mod mpc_manager;
mod epoch_manager;
pub mod types;
mod network;
pub mod network_interface;
mod counters;

pub fn start_mpc_runtime(
    my_addr: AccountAddress,
    network_client: NetworkClient<MPCMessage>,
    network_service_events: NetworkServiceEvents<MPCMessage>,
    reconfig_events: ReconfigNotificationListener<DbBackedOnChainConfig>,
    mpc_events: EventNotificationListener,
    vtxn_pool: VTxnPoolState,
    rb_config: ReliableBroadcastConfig,
) -> Runtime {
    let runtime = aptos_runtimes::spawn_named_runtime("mpc".into(), Some(4));
    let (self_sender, self_receiver) = aptos_channels::new(1_024, &counters::PENDING_SELF_MESSAGES);
    let mpc_network_client = MPCNetworkClient::new(network_client);

    let mpc_epoch_manager = EpochManager::new(
        my_addr,
        reconfig_events,
        mpc_events,
        self_sender,
        mpc_network_client,
        vtxn_pool,
        rb_config,
    );
    let (network_task, network_receiver) = NetworkTask::new(network_service_events, self_receiver);
    runtime.spawn(network_task.start());
    runtime.spawn(mpc_epoch_manager.start(network_receiver));
    runtime
}
