// Copyright © Aptos Foundation

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::runtime::Runtime;
use aptos_config::config::{NetworkConfig, NodeConfig};
use aptos_config::network_id::{NetworkContext, NetworkId};
use aptos_consensus::network_interface::ConsensusMsg;
use aptos_dkg_runtime::DKGMessage;
use aptos_jwk_consensus::types::JWKConsensusMsg;
use aptos_network2::protocols::wire::handshake::v1::ProtocolId;
use aptos_network2_builder::NetworkBuilder;
use aptos_logger::{debug, info};
use aptos_network2::application::interface::NetworkClient;
use aptos_network2::protocols::network::{NetworkEvents, NetworkSender, NetworkSource, NewNetworkEvents, NewNetworkSender, OutboundPeerConnections};
use aptos_network2::application::storage::{PEERS_AND_METADATA_SINGLETON, PeersAndMetadata};
use aptos_time_service::TimeService;
use aptos_types::chain_id::ChainId;
use aptos_event_notifications::EventSubscriptionService;
use aptos_peer_monitoring_service_types::PeerMonitoringServiceMessage;
use aptos_storage_service_types::StorageServiceMessage;
use aptos_mempool::MempoolSyncMsg;
use aptos_network2::application::{ApplicationCollector, ApplicationConnections};
use aptos_network2::application::pamgauge::PeersAndMetadataGauge;
use aptos_network_benchmark::NetbenchMessage;

pub trait MessageTrait : Clone + DeserializeOwned + Serialize + Send + Sync + Unpin + 'static {}
impl<T: Clone + DeserializeOwned + Serialize + Send + Sync + Unpin + 'static> MessageTrait for T {}

/// A simple struct that holds both the network client
/// and receiving interfaces for an application.
pub struct ApplicationNetworkInterfaces<T> {
    pub network_client: NetworkClient<T>,
    pub network_events: NetworkEvents<T>,
}

impl<T: MessageTrait> ApplicationNetworkInterfaces<T> {
    pub fn new(
        direct_send_protocols_and_preferences: Vec<ProtocolId>,
        rpc_protocols_and_preferences: Vec<ProtocolId>,
        peers_and_metadata: Arc<PeersAndMetadata>,
        network_source: NetworkSource,
        network_ids: Vec<NetworkId>,
        peer_senders: Arc<OutboundPeerConnections>,
        label: &str,
        contexts: Arc<BTreeMap<NetworkId, NetworkContext>>,
    ) -> Self {
        let mut network_senders = HashMap::new();
        for network_id in network_ids.into_iter() {
            let role_type = contexts.get(&network_id).unwrap().role();
            network_senders.insert(network_id, NetworkSender::new(network_id, peer_senders.clone(), role_type));
        }
        let network_client = NetworkClient::new(
            direct_send_protocols_and_preferences,
            rpc_protocols_and_preferences,
            network_senders,
            peers_and_metadata,
        );
        let network_events = NetworkEvents::new(network_source, peer_senders.clone(), label, contexts);
        Self {
            network_client,
            network_events,
        }
    }
}

fn has_validator_network(node_config: &NodeConfig) -> bool {
    if node_config.validator_network.is_some() {
        return true;
    }
    for net_config in node_config.full_node_networks.iter() {
        if net_config.network_id.is_validator_network() {
            return true;
        }
    }
    return false;
}

/// The read-only-ish parts of setting up a new application code module
/// (e.g. mempool, consensus, etc)
#[derive(Clone)]
pub struct AppSetupContext {
    pub node_config: NodeConfig,
    pub peers_and_metadata: Arc<PeersAndMetadata>,
    pub peer_senders: Arc<OutboundPeerConnections>,
    pub contexts: Arc<BTreeMap<NetworkId, NetworkContext>>,
}

fn build_network_connections<T: MessageTrait>(
    setup: &AppSetupContext,
    direct_send_protocols : Vec<ProtocolId>,
    rpc_protocols : Vec<ProtocolId>,
    queue_size: usize,
    counter_label: &str,
    apps: &mut ApplicationCollector,
) -> ApplicationNetworkInterfaces<T> {
    // If we were to pack a BTreeMap<ProtocolId, Receiver> we could allow application code to handle each ProtocolId with different code more efficiently rather than at this step multiplexing multiple ProtocolId messages over one channel and then possible de-multiplexing them on the app side; but it currently doesn't matter because Consensus is the only service that uses multiple ProtocolId simultaneously and it only uses them for encoding variation (json, BCS, or compressed BCS).
    let mut receivers = vec![];

    let network_ids = extract_network_ids(&setup.node_config);

    for protocol_id in direct_send_protocols.iter() {
        let (app_con, receiver) = ApplicationConnections::build(*protocol_id, queue_size, counter_label);
        receivers.push(receiver);
        apps.add(app_con);
    }
    for protocol_id in rpc_protocols.iter() {
        let (app_con, receiver) = ApplicationConnections::build(*protocol_id, queue_size, counter_label);
        receivers.push(receiver);
        apps.add(app_con);
    }

    let network_source = if receivers.len() == 1 {
        NetworkSource::new_single_source(receivers.remove(0))
    } else if receivers.len() > 1 {
        NetworkSource::new_multi_source(receivers)
    } else {
        panic!("{:?} built no receivers", counter_label);
    };
    ApplicationNetworkInterfaces::new(
        direct_send_protocols,
        rpc_protocols,
        setup.peers_and_metadata.clone(),
        network_source,
        network_ids,
        setup.peer_senders.clone(),
        counter_label,
        setup.contexts.clone(),
    )
}

pub fn consensus_network_connections(
    apps: &mut ApplicationCollector,
    setup: &AppSetupContext,
) -> Option<ApplicationNetworkInterfaces<ConsensusMsg>> {
    if !has_validator_network(&setup.node_config) {
        info!("app_int not a validator, no consensus");
        return None;
    }

    let direct_send_protocols: Vec<ProtocolId> = aptos_consensus::network_interface::DIRECT_SEND.into();
    let rpc_protocols: Vec<ProtocolId> = aptos_consensus::network_interface::RPC.into();
    let queue_size = setup.node_config.consensus.max_network_channel_size;
    let counter_label = "consensus";

    Some(build_network_connections(setup, direct_send_protocols, rpc_protocols, queue_size, counter_label, apps))
}

pub fn dkg_network_connections(
    apps: &mut ApplicationCollector,
    setup: &AppSetupContext,
) -> Option<ApplicationNetworkInterfaces<DKGMessage>> {
    if !has_validator_network(&setup.node_config) {
        info!("app_int not a validator, no dkg");
        return None;
    }

    let direct_send_protocols: Vec<ProtocolId> = aptos_dkg_runtime::network_interface::DIRECT_SEND.into();
    let rpc_protocols: Vec<ProtocolId> = aptos_dkg_runtime::network_interface::RPC.into();
    let queue_size = setup.node_config.dkg.max_network_channel_size;
    let counter_label = "dkg";

    Some(build_network_connections(setup, direct_send_protocols, rpc_protocols, queue_size, counter_label, apps))
}

pub fn jwk_consensus_network_connections(
    apps: &mut ApplicationCollector,
    setup: &AppSetupContext,
) -> Option<ApplicationNetworkInterfaces<JWKConsensusMsg>> {
    if !has_validator_network(&setup.node_config) {
        info!("app_int not a validator, no jwk");
        return None;
    }

    let direct_send_protocols: Vec<ProtocolId> = aptos_jwk_consensus::network_interface::DIRECT_SEND.into();
    let rpc_protocols: Vec<ProtocolId> = aptos_jwk_consensus::network_interface::RPC.into();
    let queue_size = setup.node_config.jwk_consensus.max_network_channel_size;
    let counter_label = "jwk";

    Some(build_network_connections(setup, direct_send_protocols, rpc_protocols, queue_size, counter_label, apps))
}

pub fn peer_monitoring_network_connections(
    apps: &mut ApplicationCollector,
    setup: &AppSetupContext,
) -> ApplicationNetworkInterfaces<PeerMonitoringServiceMessage> {
    let direct_send_protocols = Vec::<ProtocolId>::new();
    let rpc_protocols = vec![ProtocolId::PeerMonitoringServiceRpc];
    let queue_size = setup.node_config.peer_monitoring_service.max_network_channel_size as usize;
    let counter_label = "peer_monitoring";

    build_network_connections(setup, direct_send_protocols, rpc_protocols, queue_size, counter_label, apps)
}

pub fn storage_service_network_connections(
    apps: &mut ApplicationCollector,
    setup: &AppSetupContext,
) -> ApplicationNetworkInterfaces<StorageServiceMessage> {
    let direct_send_protocols = Vec::<ProtocolId>::new();
    let rpc_protocols = vec![ProtocolId::StorageServiceRpc];
    let queue_size = setup.node_config.state_sync.storage_service.max_network_channel_size as usize;
    let counter_label = "storage_service";

    build_network_connections(setup, direct_send_protocols, rpc_protocols, queue_size, counter_label, apps)
}

pub fn mempool_network_connections(
    apps: &mut ApplicationCollector,
    setup: &AppSetupContext,
) -> ApplicationNetworkInterfaces<MempoolSyncMsg> {
    let direct_send_protocols = vec![ProtocolId::MempoolDirectSend];
    let rpc_protocols = vec![];
    let queue_size = setup.node_config.mempool.max_network_channel_size;
    let counter_label = "mempool";

    build_network_connections(setup, direct_send_protocols, rpc_protocols, queue_size, counter_label, apps)
}

pub fn netbench_network_connections(
    apps: &mut ApplicationCollector,
    setup: &AppSetupContext,
) -> Option<ApplicationNetworkInterfaces<NetbenchMessage>> {
    let cfg = match setup.node_config.netbench {
        None => {return None;},
        Some(x) => {x},
    } ;
    if setup.node_config.netbench.is_none() {
        return None;
    }

    let direct_send_protocols = vec![ProtocolId::NetbenchDirectSend];
    let rpc_protocols = vec![ProtocolId::NetbenchRpc];
    let queue_size = cfg.max_network_channel_size as usize;
    let counter_label = "netbench";

    Some(build_network_connections(setup, direct_send_protocols, rpc_protocols, queue_size, counter_label, apps))
}
/// Creates a network runtime for the given network config
pub fn create_network_runtime(network_config: &NetworkConfig) -> Runtime {
    let network_id = network_config.network_id;
    debug!("Creating runtime for network ID: {}", network_id);

    // Create the runtime
    let thread_name = format!(
        "network-{}",
        network_id.as_str().chars().take(3).collect::<String>()
    );
    aptos_runtimes::spawn_named_runtime(thread_name, network_config.runtime_threads)
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
    network_configs
}

/// Extracts all network ids from the given node config
fn extract_network_ids(node_config: &NodeConfig) -> Vec<NetworkId> {
    let mut out = vec![];
    for network_config in node_config.full_node_networks.iter() {
        out.push(network_config.network_id);
    }
    if let Some(network_config) = node_config.validator_network.as_ref() {
        out.push(network_config.network_id);
    }
    out
}

/// Creates the global peers and metadata struct
pub fn create_peers_and_metadata(node_config: &NodeConfig) -> Arc<PeersAndMetadata> {
    let network_ids = extract_network_ids(node_config);
    let out = PeersAndMetadata::new(&network_ids);
    PEERS_AND_METADATA_SINGLETON.set(out.clone()).unwrap();
    PeersAndMetadataGauge::register(
        "aptos_connections".to_string(),
        "Number of current connections and their direction".to_string(),
        out.clone());
    out
}

pub fn setup_networks(
    node_config: &NodeConfig,
    chain_id: ChainId,
    peers_and_metadata: Arc<PeersAndMetadata>,
    peer_senders: Arc<OutboundPeerConnections>,
    event_subscription_service: &mut EventSubscriptionService,
) -> (Vec<Runtime>, Vec<NetworkBuilder>) {
    let network_configs = extract_network_configs(node_config);

    let mut network_runtimes = vec![];
    let mut networks = vec![];

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
            peers_and_metadata.clone(),
            peer_senders.clone(),
            Some(runtime.handle().clone()),
        );

        // Register consensus (both client and server) with the network
        // let network_id = network_config.network_id;
        // if network_id.is_validator_network() {}
        // Build and start the network on the runtime
        network_builder.build(runtime.handle().clone());
        debug!(
            "Network built for the network context: {}",
            network_builder.network_context()
        );
        network_runtimes.push(runtime);
        networks.push(network_builder);
    }

    (network_runtimes, networks)
}
