// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::services::start_netbench_service;
use aptos_channels::{self, aptos_channel, message_queues::QueueStyle};
use aptos_config::{
    config::{NetworkConfig, NodeConfig},
    network_id::NetworkId,
};
use aptos_consensus::{
    consensus_observer, consensus_observer::network::observer_message::ConsensusObserverMessage,
    network_interface::ConsensusMsg,
};
use aptos_dkg_runtime::DKGMessage;
use aptos_event_notifications::EventSubscriptionService;
use aptos_jwk_consensus::types::JWKConsensusMsg;
use aptos_logger::debug;
use aptos_mempool::network::MempoolSyncMsg;
use aptos_network::{
    application::{
        interface::{NetworkClient, NetworkServiceEvents},
        storage::PeersAndMetadata,
    },
    protocols::network::{
        NetworkApplicationConfig, NetworkClientConfig, NetworkEvents, NetworkSender,
        NetworkServiceConfig,
    },
    ProtocolId,
};
use aptos_network_benchmark::NetbenchMessage;
use aptos_network_builder::builder::NetworkBuilder;
use aptos_peer_monitoring_service_types::PeerMonitoringServiceMessage;
use aptos_storage_service_types::StorageServiceMessage;
use aptos_time_service::TimeService;
use aptos_types::chain_id::ChainId;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::runtime::Runtime;

/// A simple struct that holds both the network client
/// and receiving interfaces for an application.
pub struct ApplicationNetworkInterfaces<T> {
    pub network_client: NetworkClient<T>,
    pub network_service_events: NetworkServiceEvents<T>,
}

/// A simple struct that holds an individual application
/// network handle (i.e., network id, sender and receiver).
struct ApplicationNetworkHandle<T> {
    pub network_id: NetworkId,
    pub network_sender: NetworkSender<T>,
    pub network_events: NetworkEvents<T>,
}

/// TODO: make this configurable (e.g., for compression)
/// Returns the network application config for the consensus client and service
pub fn consensus_network_configuration(
    node_config: &NodeConfig,
    compress: bool,
) -> NetworkApplicationConfig {
    let direct_send_protocols: Vec<ProtocolId> = if compress {
        aptos_consensus::network_interface::DIRECT_SEND.into()
    } else {
        aptos_consensus::network_interface::DIRECT_SEND_NOCOMPRESS.into()
    };
    let rpc_protocols: Vec<ProtocolId> = if compress {
        aptos_consensus::network_interface::RPC.into()
    } else {
        aptos_consensus::network_interface::RPC_NOCOMPRESS.into()
    };

    let network_client_config =
        NetworkClientConfig::new(direct_send_protocols.clone(), rpc_protocols.clone());
    let network_service_config = NetworkServiceConfig::new(
        direct_send_protocols,
        rpc_protocols,
        aptos_channel::Config::new(node_config.consensus.max_network_channel_size)
            .queue_style(QueueStyle::FIFO)
            .counters(&aptos_consensus::counters::PENDING_CONSENSUS_NETWORK_EVENTS),
    );
    NetworkApplicationConfig::new(network_client_config, network_service_config)
}

/// Returns the network application config for the DKG client and service
pub fn dkg_network_configuration(node_config: &NodeConfig) -> NetworkApplicationConfig {
    let direct_send_protocols: Vec<ProtocolId> =
        aptos_dkg_runtime::network_interface::DIRECT_SEND.into();
    let rpc_protocols: Vec<ProtocolId> = aptos_dkg_runtime::network_interface::RPC.into();

    let network_client_config =
        NetworkClientConfig::new(direct_send_protocols.clone(), rpc_protocols.clone());
    let network_service_config = NetworkServiceConfig::new(
        direct_send_protocols,
        rpc_protocols,
        aptos_channel::Config::new(node_config.dkg.max_network_channel_size)
            .queue_style(QueueStyle::FIFO),
    );
    NetworkApplicationConfig::new(network_client_config, network_service_config)
}

/// Returns the network application config for the JWK consensus client and service
pub fn jwk_consensus_network_configuration(node_config: &NodeConfig) -> NetworkApplicationConfig {
    let direct_send_protocols: Vec<ProtocolId> =
        aptos_jwk_consensus::network_interface::DIRECT_SEND.into();
    let rpc_protocols: Vec<ProtocolId> = aptos_jwk_consensus::network_interface::RPC.into();

    let network_client_config =
        NetworkClientConfig::new(direct_send_protocols.clone(), rpc_protocols.clone());
    let network_service_config = NetworkServiceConfig::new(
        direct_send_protocols,
        rpc_protocols,
        aptos_channel::Config::new(node_config.jwk_consensus.max_network_channel_size)
            .queue_style(QueueStyle::FIFO),
    );
    NetworkApplicationConfig::new(network_client_config, network_service_config)
}

/// Returns the network application config for the mempool client and service
pub fn mempool_network_configuration(node_config: &NodeConfig) -> NetworkApplicationConfig {
    let direct_send_protocols = vec![ProtocolId::MempoolDirectSend];
    let rpc_protocols = vec![]; // Mempool does not use RPC

    let network_client_config =
        NetworkClientConfig::new(direct_send_protocols.clone(), rpc_protocols.clone());
    let network_service_config = NetworkServiceConfig::new(
        direct_send_protocols,
        rpc_protocols,
        aptos_channel::Config::new(node_config.mempool.max_network_channel_size)
            .queue_style(QueueStyle::KLAST) // TODO: why is this not FIFO?
            .counters(&aptos_mempool::counters::PENDING_MEMPOOL_NETWORK_EVENTS),
    );
    NetworkApplicationConfig::new(network_client_config, network_service_config)
}

/// Returns the network application config for the peer monitoring client and server
pub fn peer_monitoring_network_configuration(node_config: &NodeConfig) -> NetworkApplicationConfig {
    let direct_send_protocols = vec![]; // The monitoring service does not use direct send
    let rpc_protocols = vec![ProtocolId::PeerMonitoringServiceRpc];
    let max_network_channel_size =
        node_config.peer_monitoring_service.max_network_channel_size as usize;

    let network_client_config =
        NetworkClientConfig::new(direct_send_protocols.clone(), rpc_protocols.clone());
    let network_service_config = NetworkServiceConfig::new(
        direct_send_protocols,
        rpc_protocols,
        aptos_channel::Config::new(max_network_channel_size)
            .queue_style(QueueStyle::FIFO)
            .counters(
                &aptos_peer_monitoring_service_server::metrics::PENDING_PEER_MONITORING_SERVER_NETWORK_EVENTS,
            ),
    );
    NetworkApplicationConfig::new(network_client_config, network_service_config)
}

/// Returns the network application config for the storage service client and server
pub fn storage_service_network_configuration(node_config: &NodeConfig) -> NetworkApplicationConfig {
    let direct_send_protocols = vec![]; // The storage service does not use direct send
    let rpc_protocols = vec![ProtocolId::StorageServiceRpc];
    let max_network_channel_size = node_config
        .state_sync
        .storage_service
        .max_network_channel_size as usize;

    let network_client_config =
        NetworkClientConfig::new(direct_send_protocols.clone(), rpc_protocols.clone());
    let network_service_config = NetworkServiceConfig::new(
        direct_send_protocols,
        rpc_protocols,
        aptos_channel::Config::new(max_network_channel_size)
            .queue_style(QueueStyle::FIFO)
            .counters(
                &aptos_storage_service_server::metrics::PENDING_STORAGE_SERVER_NETWORK_EVENTS,
            ),
    );
    NetworkApplicationConfig::new(network_client_config, network_service_config)
}

/// Returns the network application config for the consensus observer client and server
pub fn consensus_observer_network_configuration(
    node_config: &NodeConfig,
) -> NetworkApplicationConfig {
    let direct_send_protocols = vec![ProtocolId::ConsensusObserver];
    let rpc_protocols = vec![ProtocolId::ConsensusObserverRpc];
    let max_network_channel_size = node_config.consensus_observer.max_network_channel_size as usize;

    let network_client_config =
        NetworkClientConfig::new(direct_send_protocols.clone(), rpc_protocols.clone());
    let network_service_config = NetworkServiceConfig::new(
        direct_send_protocols,
        rpc_protocols,
        aptos_channel::Config::new(max_network_channel_size)
            .queue_style(QueueStyle::FIFO)
            .counters(
                &consensus_observer::common::metrics::PENDING_CONSENSUS_OBSERVER_NETWORK_EVENTS,
            ),
    );
    NetworkApplicationConfig::new(network_client_config, network_service_config)
}

/// Returns the network application config for the netbench client and server
pub fn netbench_network_configuration(
    node_config: &NodeConfig,
) -> Option<NetworkApplicationConfig> {
    let cfg = match node_config.netbench {
        None => return None,
        Some(x) => x,
    };
    if !cfg.enabled {
        return None;
    }
    let direct_send_protocols = vec![ProtocolId::NetbenchDirectSend];
    let rpc_protocols = vec![ProtocolId::NetbenchRpc];
    let network_client_config =
        NetworkClientConfig::new(direct_send_protocols.clone(), rpc_protocols.clone());
    let max_network_channel_size = cfg.max_network_channel_size as usize;
    let network_service_config = NetworkServiceConfig::new(
        direct_send_protocols,
        rpc_protocols,
        aptos_channel::Config::new(max_network_channel_size)
            .queue_style(QueueStyle::FIFO)
            .counters(&aptos_network_benchmark::PENDING_NETBENCH_NETWORK_EVENTS),
    );
    Some(NetworkApplicationConfig::new(
        network_client_config,
        network_service_config,
    ))
}

/// Extracts all network configs from the given node config
fn extract_network_configs(node_config: &NodeConfig) -> Vec<NetworkConfig> {
    let mut network_configs: Vec<NetworkConfig> = node_config.full_node_networks.to_vec();
    if let Some(network_config) = node_config.validator_network.as_ref() {
        // Ensure that mutual authentication is enabled by default!
        if !network_config.mutual_authentication {
            panic!("Validator networks must always have mutual_authentication enabled!");
        }
        network_configs.push(network_config.clone());
    }
    if let Some(network_config) = node_config.validator_network2.as_ref() {
        // Ensure that mutual authentication is enabled by default!
        if !network_config.mutual_authentication {
            panic!("Validator networks must always have mutual_authentication enabled!");
        }
        network_configs.push(network_config.clone());
    }
    if let Some(network_config) = node_config.validator_network3.as_ref() {
        // Ensure that mutual authentication is enabled by default!
        if !network_config.mutual_authentication {
            panic!("Validator networks must always have mutual_authentication enabled!");
        }
        network_configs.push(network_config.clone());
    }
    network_configs
}

/// Extracts all network ids from the given node config
fn extract_network_ids(node_config: &NodeConfig) -> Vec<NetworkId> {
    extract_network_configs(node_config)
        .into_iter()
        .map(|network_config| network_config.network_id)
        .collect()
}

/// Creates the global peers and metadata struct
pub fn create_peers_and_metadata(node_config: &NodeConfig) -> Arc<PeersAndMetadata> {
    let network_ids = extract_network_ids(node_config);
    PeersAndMetadata::new(&network_ids)
}

/// Sets up all networks and returns the appropriate application network interfaces
pub fn setup_networks_and_get_interfaces(
    node_config: &NodeConfig,
    chain_id: ChainId,
    peers_and_metadata: Arc<PeersAndMetadata>,
    event_subscription_service: &mut EventSubscriptionService,
    event_subscription_service2: &mut EventSubscriptionService,
    event_subscription_service3: &mut EventSubscriptionService,
) -> (
    Vec<Runtime>,
    Option<ApplicationNetworkInterfaces<ConsensusMsg>>,
    Option<ApplicationNetworkInterfaces<ConsensusMsg>>,
    Option<ApplicationNetworkInterfaces<ConsensusMsg>>,
    Option<ApplicationNetworkInterfaces<ConsensusObserverMessage>>,
    Option<ApplicationNetworkInterfaces<DKGMessage>>,
    Option<ApplicationNetworkInterfaces<JWKConsensusMsg>>,
    ApplicationNetworkInterfaces<MempoolSyncMsg>,
    ApplicationNetworkInterfaces<PeerMonitoringServiceMessage>,
    ApplicationNetworkInterfaces<StorageServiceMessage>,
) {
    // Gather all network configs
    let network_configs = extract_network_configs(node_config);

    // Create each network and register the application handles
    let mut network_runtimes = vec![];
    let mut consensus_network_handle = None;
    let mut consensus_network_handle2 = None;
    let mut consensus_network_handle3 = None;
    let mut peers_and_metadata1 = None;
    let mut peers_and_metadata2 = None;
    let mut peers_and_metadata3 = None;
    let mut consensus_observer_network_handles: Option<
        Vec<ApplicationNetworkHandle<ConsensusObserverMessage>>,
    > = None;
    let mut dkg_network_handle = None;
    let mut jwk_consensus_network_handle = None;
    let mut mempool_network_handles = vec![];
    let mut peer_monitoring_service_network_handles = vec![];
    let mut storage_service_network_handles = vec![];
    let mut netbench_handles = Vec::<ApplicationNetworkHandle<NetbenchMessage>>::new();
    for network_config in network_configs.into_iter() {
        // Create a network runtime for the config
        let runtime = create_network_runtime(&network_config);

        // Entering gives us a runtime to instantiate all the pieces of the builder
        let _enter = runtime.enter();

        let ess = if consensus_network_handle.is_none() {
            &mut *event_subscription_service
        } else if consensus_network_handle2.is_none() {
            &mut *event_subscription_service2
        } else {
            &mut *event_subscription_service3
        };

        let peers_and_metadata = if consensus_network_handle.is_none() {
            peers_and_metadata.clone()
        } else {
            PeersAndMetadata::new(&[NetworkId::Validator])
        };

        // Create a new network builder
        let mut network_builder = NetworkBuilder::create(
            chain_id,
            node_config.base.role,
            &network_config,
            TimeService::real(),
            Some(ess),
            peers_and_metadata.clone(),
        );

        // Register consensus (both client and server) with the network
        let network_id = network_config.network_id;
        if network_id.is_validator_network() {
            // A validator node must have only a single consensus network handle
            if consensus_network_handle.is_some()
                && consensus_network_handle2.is_some()
                && consensus_network_handle3.is_some()
            {
                panic!("There can be at most two validator network!");
            } else {
                let compress = if consensus_network_handle.is_none() {
                    true
                } else if consensus_network_handle2.is_none() {
                    false
                } else {
                    true
                };

                let network_handle = register_client_and_service_with_network(
                    &mut network_builder,
                    network_id,
                    &network_config,
                    consensus_network_configuration(node_config, compress),
                    false,
                );
                if consensus_network_handle.is_none() {
                    peers_and_metadata1 = Some(peers_and_metadata);
                    consensus_network_handle = Some(network_handle);
                } else if consensus_network_handle2.is_none() {
                    peers_and_metadata2 = Some(peers_and_metadata);
                    consensus_network_handle2 = Some(network_handle);
                } else {
                    peers_and_metadata3 = Some(peers_and_metadata);
                    consensus_network_handle3 = Some(network_handle);
                }
            }

            // if dkg_network_handle.is_some() {
            //     panic!("There can be at most one validator network!");
            // } else {
            //     let network_handle = register_client_and_service_with_network(
            //         &mut network_builder,
            //         network_id,
            //         &network_config,
            //         dkg_network_configuration(node_config),
            //         true,
            //     );
            //     dkg_network_handle = Some(network_handle);
            // }
            //
            // if jwk_consensus_network_handle.is_some() {
            //     panic!("There can be at most one validator network!");
            // } else {
            //     let network_handle = register_client_and_service_with_network(
            //         &mut network_builder,
            //         network_id,
            //         &network_config,
            //         jwk_consensus_network_configuration(node_config),
            //         true,
            //     );
            //     jwk_consensus_network_handle = Some(network_handle);
            // }
        }

        // Register consensus observer (both client and server) with the network
        if node_config
            .consensus_observer
            .is_observer_or_publisher_enabled()
        {
            // Create the network handle for this network type
            let network_handle = register_client_and_service_with_network(
                &mut network_builder,
                network_id,
                &network_config,
                consensus_observer_network_configuration(node_config),
                false,
            );

            // Add the network handle to the set of handles
            if let Some(consensus_observer_network_handles) =
                &mut consensus_observer_network_handles
            {
                consensus_observer_network_handles.push(network_handle);
            } else {
                consensus_observer_network_handles = Some(vec![network_handle]);
            }
        }

        // Register mempool (both client and server) with the network
        let mempool_network_handle = register_client_and_service_with_network(
            &mut network_builder,
            network_id,
            &network_config,
            mempool_network_configuration(node_config),
            true,
        );
        mempool_network_handles.push(mempool_network_handle);

        // Register the peer monitoring service (both client and server) with the network
        let peer_monitoring_service_network_handle = register_client_and_service_with_network(
            &mut network_builder,
            network_id,
            &network_config,
            peer_monitoring_network_configuration(node_config),
            true,
        );
        peer_monitoring_service_network_handles.push(peer_monitoring_service_network_handle);

        // Register the storage service (both client and server) with the network
        let storage_service_network_handle = register_client_and_service_with_network(
            &mut network_builder,
            network_id,
            &network_config,
            storage_service_network_configuration(node_config),
            true,
        );
        storage_service_network_handles.push(storage_service_network_handle);

        // Register the network benchmark test service
        if let Some(app_config) = netbench_network_configuration(node_config) {
            let netbench_handle = register_client_and_service_with_network(
                &mut network_builder,
                network_id,
                &network_config,
                app_config,
                true,
            );
            netbench_handles.push(netbench_handle);
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

    // Transform all network handles into application interfaces
    let (
        consensus_interfaces,
        qs_interfaces,
        qs2_interfaces,
        consensus_observer_interfaces,
        dkg_interfaces,
        jwk_consensus_interfaces,
        mempool_interfaces,
        peer_monitoring_service_interfaces,
        storage_service_interfaces,
    ) = transform_network_handles_into_interfaces(
        node_config,
        consensus_network_handle,
        consensus_network_handle2,
        consensus_network_handle3,
        consensus_observer_network_handles,
        dkg_network_handle,
        jwk_consensus_network_handle,
        mempool_network_handles,
        peer_monitoring_service_network_handles,
        storage_service_network_handles,
        peers_and_metadata1.unwrap(),
        peers_and_metadata2.unwrap(),
        peers_and_metadata3.unwrap(),
    );

    if !netbench_handles.is_empty() {
        let netbench_interfaces = create_network_interfaces(
            netbench_handles,
            netbench_network_configuration(node_config).unwrap(),
            peers_and_metadata,
        );
        let netbench_service_threads = node_config.netbench.unwrap().netbench_service_threads;
        let netbench_runtime =
            aptos_runtimes::spawn_named_runtime("benchmark".into(), netbench_service_threads);
        start_netbench_service(node_config, netbench_interfaces, netbench_runtime.handle());
        network_runtimes.push(netbench_runtime);
    }

    (
        network_runtimes,
        consensus_interfaces,
        qs_interfaces,
        qs2_interfaces,
        consensus_observer_interfaces,
        dkg_interfaces,
        jwk_consensus_interfaces,
        mempool_interfaces,
        peer_monitoring_service_interfaces,
        storage_service_interfaces,
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

/// Registers a new application client and service with the network
fn register_client_and_service_with_network<
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
>(
    network_builder: &mut NetworkBuilder,
    network_id: NetworkId,
    network_config: &NetworkConfig,
    application_config: NetworkApplicationConfig,
    allow_out_of_order_delivery: bool,
) -> ApplicationNetworkHandle<T> {
    let (network_sender, network_events) = network_builder.add_client_and_service(
        &application_config,
        network_config.max_parallel_deserialization_tasks,
        allow_out_of_order_delivery,
    );
    ApplicationNetworkHandle {
        network_id,
        network_sender,
        network_events,
    }
}

/// Transforms the given network handles into interfaces that can
/// be used by the applications themselves.
fn transform_network_handles_into_interfaces(
    node_config: &NodeConfig,
    consensus_network_handle: Option<ApplicationNetworkHandle<ConsensusMsg>>,
    consensus_network_handle2: Option<ApplicationNetworkHandle<ConsensusMsg>>,
    consensus_network_handle3: Option<ApplicationNetworkHandle<ConsensusMsg>>,
    consensus_observer_network_handles: Option<
        Vec<ApplicationNetworkHandle<ConsensusObserverMessage>>,
    >,
    dkg_network_handle: Option<ApplicationNetworkHandle<DKGMessage>>,
    jwk_consensus_network_handle: Option<ApplicationNetworkHandle<JWKConsensusMsg>>,
    mempool_network_handles: Vec<ApplicationNetworkHandle<MempoolSyncMsg>>,
    peer_monitoring_service_network_handles: Vec<
        ApplicationNetworkHandle<PeerMonitoringServiceMessage>,
    >,
    storage_service_network_handles: Vec<ApplicationNetworkHandle<StorageServiceMessage>>,
    peers_and_metadata: Arc<PeersAndMetadata>,
    peers_and_metadata2: Arc<PeersAndMetadata>,
    peers_and_metadata3: Arc<PeersAndMetadata>,
) -> (
    Option<ApplicationNetworkInterfaces<ConsensusMsg>>,
    Option<ApplicationNetworkInterfaces<ConsensusMsg>>,
    Option<ApplicationNetworkInterfaces<ConsensusMsg>>,
    Option<ApplicationNetworkInterfaces<ConsensusObserverMessage>>,
    Option<ApplicationNetworkInterfaces<DKGMessage>>,
    Option<ApplicationNetworkInterfaces<JWKConsensusMsg>>,
    ApplicationNetworkInterfaces<MempoolSyncMsg>,
    ApplicationNetworkInterfaces<PeerMonitoringServiceMessage>,
    ApplicationNetworkInterfaces<StorageServiceMessage>,
) {
    let consensus_interfaces = consensus_network_handle.map(|consensus_network_handle| {
        create_network_interfaces(
            vec![consensus_network_handle],
            consensus_network_configuration(node_config, true),
            peers_and_metadata.clone(),
        )
    });

    let consensus_interfaces2 = consensus_network_handle2.map(|consensus_network_handle| {
        create_network_interfaces(
            vec![consensus_network_handle],
            consensus_network_configuration(node_config, false),
            peers_and_metadata2.clone(),
        )
    });

    let consensus_interfaces3 = consensus_network_handle3.map(|consensus_network_handle| {
        create_network_interfaces(
            vec![consensus_network_handle],
            consensus_network_configuration(node_config, true),
            peers_and_metadata3.clone(),
        )
    });

    let consensus_observer_interfaces =
        consensus_observer_network_handles.map(|consensus_observer_network_handles| {
            create_network_interfaces(
                consensus_observer_network_handles,
                consensus_observer_network_configuration(node_config),
                peers_and_metadata.clone(),
            )
        });

    let dkg_interfaces = dkg_network_handle.map(|handle| {
        create_network_interfaces(
            vec![handle],
            dkg_network_configuration(node_config),
            peers_and_metadata.clone(),
        )
    });

    let jwk_consensus_interfaces = jwk_consensus_network_handle.map(|handle| {
        create_network_interfaces(
            vec![handle],
            jwk_consensus_network_configuration(node_config),
            peers_and_metadata.clone(),
        )
    });

    let mempool_interfaces = create_network_interfaces(
        mempool_network_handles,
        mempool_network_configuration(node_config),
        peers_and_metadata.clone(),
    );

    let peer_monitoring_service_interfaces = create_network_interfaces(
        peer_monitoring_service_network_handles,
        peer_monitoring_network_configuration(node_config),
        peers_and_metadata.clone(),
    );

    let storage_service_interfaces = create_network_interfaces(
        storage_service_network_handles,
        storage_service_network_configuration(node_config),
        peers_and_metadata.clone(),
    );

    (
        consensus_interfaces,
        consensus_interfaces2,
        consensus_interfaces3,
        consensus_observer_interfaces,
        dkg_interfaces,
        jwk_consensus_interfaces,
        mempool_interfaces,
        peer_monitoring_service_interfaces,
        storage_service_interfaces,
    )
}

/// Creates an application network inteface using the given
/// handles and config.
fn create_network_interfaces<
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
>(
    network_handles: Vec<ApplicationNetworkHandle<T>>,
    network_application_config: NetworkApplicationConfig,
    peers_and_metadata: Arc<PeersAndMetadata>,
) -> ApplicationNetworkInterfaces<T> {
    // Gather the network senders and events
    let mut network_senders = HashMap::new();
    let mut network_and_events = HashMap::new();
    for network_handle in network_handles {
        let network_id = network_handle.network_id;
        network_senders.insert(network_id, network_handle.network_sender);
        network_and_events.insert(network_id, network_handle.network_events);
    }

    // Create the network client
    let network_client_config = network_application_config.network_client_config;
    let network_client = NetworkClient::new(
        network_client_config.direct_send_protocols_and_preferences,
        network_client_config.rpc_protocols_and_preferences,
        network_senders,
        peers_and_metadata,
    );

    // Create the network service events
    let network_service_events = NetworkServiceEvents::new(network_and_events);

    // Create and return the new network interfaces
    ApplicationNetworkInterfaces {
        network_client,
        network_service_events,
    }
}
