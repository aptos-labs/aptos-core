// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod counters;
mod dkg_manager;
pub mod epoch_manager;
pub mod network;
pub mod network_interface;
pub mod transcript_aggregation;
pub mod types;

use crate::{
    epoch_manager::EpochManager, network::NetworkTask, network_interface::DKGNetworkClient,
};
use aptos_config::config::IdentityBlob;
use aptos_event_notifications::{
    DbBackedOnChainConfig, EventNotificationListener, ReconfigNotificationListener,
};
use aptos_network::application::interface::{NetworkClient, NetworkServiceEvents};
use move_core_types::account_address::AccountAddress;
use std::sync::Arc;
use tokio::runtime::Runtime;
pub use types::DKGMessage;

pub fn start_dkg_runtime(
    my_addr: AccountAddress,
    identity_blob: Arc<IdentityBlob>,
    network_client: NetworkClient<DKGMessage>,
    network_service_events: NetworkServiceEvents<DKGMessage>,
    reconfig_events: ReconfigNotificationListener<DbBackedOnChainConfig>,
    dkg_start_events: EventNotificationListener,
    dkg_txn_writer: aptos_validator_transaction_pool::SingleTopicWriteClient,
    dkg_pulled_rx: aptos_validator_transaction_pool::PullNotificationReceiver,
) -> Runtime {
    let runtime = aptos_runtimes::spawn_named_runtime("dkg".into(), Some(4));
    let (self_sender, self_receiver) = aptos_channels::new(1_024, &counters::PENDING_SELF_MESSAGES);
    let dkg_network_client = DKGNetworkClient::new(network_client);

    let dkg_epoch_manager = EpochManager::new(
        my_addr,
        identity_blob.clone(),
        reconfig_events,
        dkg_start_events,
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

//TODO(zjma): make this test-only after real dkg.
pub(crate) mod dummy_dkg;
