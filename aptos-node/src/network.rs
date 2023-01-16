// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_config::{
    config::{NetworkConfig, NodeConfig},
    network_id::NetworkId,
};
use aptos_consensus::network_interface::{consensus_network_config, ConsensusMsg};
use aptos_event_notifications::EventSubscriptionService;
use aptos_logger::debug;
use aptos_mempool::network::{mempool_network_config, MempoolSyncMsg};
use aptos_network::{
    application::storage::PeerMetadataStorage,
    protocols::network::{NetworkEvents, NetworkSender},
};
use aptos_network_builder::builder::NetworkBuilder;
use aptos_storage_service_client::storage_client_network_config;
use aptos_storage_service_server::network::{
    storage_service_network_config, StorageServiceNetworkEvents,
};
use aptos_storage_service_types::StorageServiceMessage;
use aptos_time_service::TimeService;
use aptos_types::chain_id::ChainId;
use std::{collections::HashMap, sync::Arc};
use tokio::runtime::Runtime;

const MEMPOOL_NETWORK_CHANNEL_BUFFER_SIZE: usize = 1_024;

/// A simple struct that holds an application network handle
/// (i.e., network id, sender and receiver).
pub struct ApplicationNetworkHandle<T> {
    pub network_id: NetworkId,
    pub network_sender: NetworkSender<T>,
    pub network_events: NetworkEvents<T>,
}

/// Extracts all network configs and ids from the given node config.
/// This method also does some basic verification of the network configs.
fn extract_network_configs_and_ids(
    node_config: &NodeConfig,
) -> (Vec<NetworkConfig>, Vec<NetworkId>) {
    // Extract all network configs
    let mut network_configs: Vec<NetworkConfig> = node_config.full_node_networks.to_vec();
    if let Some(network_config) = node_config.validator_network.as_ref() {
        // Ensure that mutual authentication is enabled by default!
        if !network_config.mutual_authentication {
            panic!("Validator networks must always have mutual_authentication enabled!");
        }
        network_configs.push(network_config.clone());
    }

    // Extract all network IDs
    let mut network_ids = vec![];
    for network_config in &network_configs {
        // Guarantee there is only one of this network
        let network_id = network_config.network_id;
        if network_ids.contains(&network_id) {
            panic!(
                "Duplicate NetworkId: '{}'. Can't start node with duplicate networks! Check the node config!",
                network_id
            );
        }
        network_ids.push(network_id);
    }

    (network_configs, network_ids)
}

/// Sets up all networks and returns the appropriate application network handles
pub fn setup_networks_and_get_handles(
    node_config: &NodeConfig,
    chain_id: ChainId,
    event_subscription_service: &mut EventSubscriptionService,
) -> (
    Vec<Runtime>,
    Arc<PeerMetadataStorage>,
    Vec<ApplicationNetworkHandle<MempoolSyncMsg>>,
    Option<ApplicationNetworkHandle<ConsensusMsg>>,
    Vec<StorageServiceNetworkEvents>,
    HashMap<NetworkId, NetworkSender<StorageServiceMessage>>,
) {
    // Gather all network configs and network ids
    let (network_configs, network_ids) = extract_network_configs_and_ids(node_config);

    // Create the global peer metadata storage
    let peer_metadata_storage = PeerMetadataStorage::new(&network_ids);

    // Create each network and register the applications
    let mut network_runtimes = vec![];
    let mut mempool_network_handles = vec![];
    let mut consensus_network_handle = None;
    let mut storage_service_network_events = vec![];
    let mut storage_client_network_senders = HashMap::new();
    for network_config in network_configs.into_iter() {
        // Create a network runtime for the config
        let runtime = create_network_runtime(&network_config);

        // Entering gives us a runtime to instantiate all the pieces of the builder
        let _enter = runtime.enter();

        // Create a new network builder
        let mut network_builder = NetworkBuilder::create(
            chain_id,
            node_config.base.role,
            &network_config,
            TimeService::real(),
            Some(event_subscription_service),
            peer_metadata_storage.clone(),
        );

        // Register the storage service client and server with the network
        let network_id = network_config.network_id;
        let storage_client_network_sender =
            network_builder.add_client(&storage_client_network_config());
        storage_client_network_senders.insert(network_id, storage_client_network_sender);
        let storage_service_events = network_builder.add_service(&storage_service_network_config(
            node_config.state_sync.storage_service,
        ));
        storage_service_network_events.push(storage_service_events);

        // Register mempool (both client and server) with the network
        let (mempool_network_sender, mempool_network_events) = network_builder
            .add_client_and_service(&mempool_network_config(MEMPOOL_NETWORK_CHANNEL_BUFFER_SIZE));
        let mempool_network_handle = ApplicationNetworkHandle {
            network_id,
            network_sender: mempool_network_sender,
            network_events: mempool_network_events,
        };
        mempool_network_handles.push(mempool_network_handle);

        // Register consensus (both client and server) with the network
        if network_id.is_validator_network() {
            // A validator node must have only a single consensus network handle
            if consensus_network_handle.is_some() {
                panic!("There can be at most one validator network!");
            } else {
                let (network_sender, network_events) =
                    network_builder.add_client_and_service(&consensus_network_config());
                let application_network_handle = ApplicationNetworkHandle {
                    network_id: NetworkId::Validator,
                    network_sender,
                    network_events,
                };
                consensus_network_handle = Some(application_network_handle);
            }
        }

        // Build and start the network on the runtime
        network_builder.build(runtime.handle().clone());
        network_builder.start();
        network_runtimes.push(runtime);
        debug!(
            "Network built for the network context: {}",
            network_builder.network_context()
        );
    }

    (
        network_runtimes,
        peer_metadata_storage,
        mempool_network_handles,
        consensus_network_handle,
        storage_service_network_events,
        storage_client_network_senders,
    )
}

/// Creates a network runtime for the given network config
fn create_network_runtime(network_config: &NetworkConfig) -> Runtime {
    let network_id = network_config.network_id;
    debug!("Creating runtime for network ID: {}", network_id);

    // Create the runtime
    let thread_name = format!(
        "network-{}",
        network_id.as_str().chars().take(3).collect::<String>()
    );
    aptos_runtimes::spawn_named_runtime(thread_name, network_config.runtime_threads)
}
