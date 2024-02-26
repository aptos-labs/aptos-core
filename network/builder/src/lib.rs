// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Handle;
use aptos_config::config::{DiscoveryMethod, HANDSHAKE_VERSION, NetworkConfig, RoleType};
use aptos_config::network_id::{NetworkContext, NetworkId};
use aptos_crypto::x25519;
use aptos_event_notifications::{DbBackedOnChainConfig, EventSubscriptionService};
use aptos_logger::info;
#[cfg(any(test, feature = "testing", feature = "fuzzing"))]
use aptos_netcore::transport::memory::MemoryTransport;
use aptos_netcore::transport::tcp::{TCPBufferCfg, TcpSocket, TcpTransport};
use aptos_network2::application::ApplicationCollector;
use aptos_network_discovery::DiscoveryChangeListener;
use aptos_time_service::TimeService;
use aptos_types::chain_id::ChainId;
use aptos_network2::application::storage::PeersAndMetadata;
use aptos_network2::connectivity_manager::{ConnectivityManager, ConnectivityRequest};
use aptos_network2::noise::stream::NoiseStream;
use aptos_network2::protocols::wire::handshake::v1::ProtocolIdSet;
use aptos_network2::protocols::network::OutboundPeerConnections;
use aptos_network2::transport::{APTOS_TCP_TRANSPORT, AptosNetTransport, AptosNetTransportActual};
use aptos_types::network_address::{NetworkAddress, Protocol};
use tokio_retry::strategy::ExponentialBackoff;
use peer_listener::PeerListener;

mod peer_listener;

#[derive(Debug, PartialEq, PartialOrd)]
enum State {
    CREATED,
    BUILT,
    STARTED,
}


/// Inbound and Outbound connections are always secured with NoiseIK.  The dialer
/// will always verify the listener.
#[derive(Debug)]
pub enum AuthenticationMode {
    /// Inbound connections will first be checked against the known peers set, and
    /// if the `PeerId` is known it will be authenticated against it's `PublicKey`
    /// Otherwise, the incoming connections will be allowed through in the common
    /// pool of unknown peers.
    MaybeMutual(x25519::PrivateKey),
    /// Both dialer and listener will verify public keys of each other in the
    /// handshake.
    Mutual(x25519::PrivateKey),
}

/// Build Network module with custom configuration values.
/// Methods can be chained in order to set the configuration values.
/// MempoolNetworkHandler and ConsensusNetworkHandler are constructed by calling
/// [`NetworkBuilder::build`].  New instances of `NetworkBuilder` are obtained
/// via [`NetworkBuilder::create`].
pub struct NetworkBuilder {
    state: State,
    time_service: TimeService,
    network_context: NetworkContext,
    chain_id: ChainId,
    config: NetworkConfig,
    discovery_listeners: Vec<DiscoveryChangeListener<DbBackedOnChainConfig>>,
    peers_and_metadata: Arc<PeersAndMetadata>,
    apps: Arc<ApplicationCollector>,
    peer_senders: Arc<OutboundPeerConnections>,
    handle: Option<Handle>,
    // temporarily hold a value from create() until start()
    connectivity_req_rx: Option<tokio::sync::mpsc::Receiver<ConnectivityRequest>>,
}

impl NetworkBuilder {
    /// Create a new NetworkBuilder based on the provided configuration.
    pub fn create(
        chain_id: ChainId,
        role: RoleType,
        config: &NetworkConfig,
        time_service: TimeService,
        reconfig_subscription_service: Option<&mut EventSubscriptionService>,
        peers_and_metadata: Arc<PeersAndMetadata>,
        peer_senders: Arc<OutboundPeerConnections>,
        handle: Option<Handle>,
    ) -> NetworkBuilder {
        let network_context = NetworkContext::new(role, config.network_id, config.peer_id());
        let (connectivity_req_sender, connectivity_req_rx) = tokio::sync::mpsc::channel::<ConnectivityRequest>(10);
        let mut nb = NetworkBuilder{
            state: State::CREATED,
            time_service,
            network_context,
            chain_id,
            config: config.clone(),
            discovery_listeners: vec![],
            peers_and_metadata,
            peer_senders,
            apps: Arc::new(ApplicationCollector::new()), // temporary empty app set
            handle,
            connectivity_req_rx: Some(connectivity_req_rx),
        };
        nb.setup_discovery(reconfig_subscription_service, connectivity_req_sender);
        nb
    }

    pub fn set_apps(&mut self, apps: Arc<ApplicationCollector>) {
        self.apps = apps;
    }

    pub fn active_protocol_ids(&self) -> ProtocolIdSet {
        let mut out = ProtocolIdSet::empty();
        for (protocol_id, _) in self.apps.iter() {
            out.insert(*protocol_id);
        }
        out
    }

    pub fn build(&mut self, handle: Handle) {
        if self.state != State::CREATED {
            panic!("NetworkBuilder.build but not in state CREATED");
        }
        self.handle = Some(handle);
        self.state = State::BUILT;
    }

    fn setup_discovery(
        &mut self,
        mut reconfig_subscription_service: Option<&mut EventSubscriptionService>,
        conn_mgr_reqs_tx: tokio::sync::mpsc::Sender<ConnectivityRequest>,
    ) {
        for disco in self.config.discovery_methods().into_iter() {
            let listener = match disco {
                DiscoveryMethod::Onchain => {
                    let reconfig_events = reconfig_subscription_service
                        .as_mut()
                        .expect("An event subscription service is required for on-chain discovery!")
                        .subscribe_to_reconfigurations()
                        .expect("On-chain discovery is unable to subscribe to reconfigurations!");
                    let identity_key = self.config.identity_key();
                    let pubkey = identity_key.public_key();
                    DiscoveryChangeListener::validator_set(
                        self.network_context,
                        conn_mgr_reqs_tx.clone(),
                        pubkey,
                        reconfig_events,
                    )
                }
                DiscoveryMethod::File(file_discovery) => DiscoveryChangeListener::file(
                    self.network_context,
                    conn_mgr_reqs_tx.clone(),
                    file_discovery.path.as_path(),
                    Duration::from_secs(file_discovery.interval_secs),
                    self.time_service.clone(),
                ),
                DiscoveryMethod::Rest(rest_discovery) => DiscoveryChangeListener::rest(
                    self.network_context,
                    conn_mgr_reqs_tx.clone(),
                    rest_discovery.url.clone(),
                    Duration::from_secs(rest_discovery.interval_secs),
                    self.time_service.clone(),
                ),
                DiscoveryMethod::None => {
                    continue;
                }
            };
            self.discovery_listeners.push(listener);
        }
    }

    fn get_tcp_buffers_cfg(&self) -> TCPBufferCfg {
        TCPBufferCfg::new_configs(
            self.config.inbound_rx_buffer_size_bytes,
            self.config.inbound_tx_buffer_size_bytes,
            self.config.outbound_rx_buffer_size_bytes,
            self.config.outbound_tx_buffer_size_bytes,
        )
    }

    fn build_transport(&mut self) -> (TransportPeerManager, AptosNetTransportActual) {
        let listen_parts = self.config.listen_address.as_slice();
        let key = self.config.identity_key();
        let mutual_auth = self.config.mutual_authentication;
        let protos = self.active_protocol_ids();
        let enable_proxy_protocol = self.config.enable_proxy_protocol;
        match listen_parts[0] {
            Protocol::Ip4(_) | Protocol::Ip6(_) => {
                // match listen_parts[1]
                let mut aptos_tcp_transport = APTOS_TCP_TRANSPORT.clone();
                let tcp_cfg = self.get_tcp_buffers_cfg();
                aptos_tcp_transport.set_tcp_buffers(&tcp_cfg);
                let ant = AptosNetTransport::<TcpTransport>::new(
                    aptos_tcp_transport,
                    self.network_context,
                    self.time_service.clone(),
                    key,
                    self.peers_and_metadata.clone(),
                    mutual_auth,
                    HANDSHAKE_VERSION,
                    self.chain_id,
                    protos,
                    enable_proxy_protocol,
                );
                let pm = PeerListener::new(ant.clone(), self.peers_and_metadata.clone(), self.config.clone(), self.network_context, self.apps.clone(), self.peer_senders.clone());
                (TransportPeerManager::Tcp(pm), AptosNetTransportActual::Tcp(ant))
            }
            #[cfg(any(test, feature = "testing", feature = "fuzzing"))]
            Protocol::Memory(_) => {
                let ant = AptosNetTransport::<MemoryTransport>::new(
                    MemoryTransport,
                    self.network_context,
                    self.time_service.clone(),
                    key,
                    self.peers_and_metadata.clone(),
                    mutual_auth,
                    HANDSHAKE_VERSION,
                    self.chain_id,
                    protos,
                    enable_proxy_protocol,
                );
                let pm = PeerListener::new(ant.clone(), self.peers_and_metadata.clone(), self.config.clone(), self.network_context, self.apps.clone(), self.peer_senders.clone());
                (TransportPeerManager::Memory(pm), AptosNetTransportActual::Memory(ant))
            }
            _ => {
                panic!("cannot listen on address {:?}", self.config.listen_address);
            }
        }
    }

    pub fn start(&mut self) {
        if self.state != State::BUILT {
            panic!("NetworkBuilder.build but not in state BUILT");
        }
        let handle = self.handle.clone().unwrap();
        _ = handle.enter();
        let seeds = self.config.merge_seeds();
        let connectivity_req_rx = self.connectivity_req_rx.take().unwrap();
        let (tpm, ant) = self.build_transport();
        let cm = ConnectivityManager::new(
            self.config.clone(),
            self.network_context,
            self.time_service.clone(),
            self.peers_and_metadata.clone(),
            seeds,
            connectivity_req_rx,
            ExponentialBackoff::from_millis(self.config.connection_backoff_base).factor(1000),
            ant,
            self.apps.clone(),
            self.peer_senders.clone(),
        );
        handle.spawn(cm.start(handle.clone()));
        for disco in self.discovery_listeners.drain(..) {
            disco.start(&handle);
        }
        let listen_addr = transport_peer_manager_start(
            tpm,
            self.config.listen_address.clone(),
            handle,
            self.network_context.network_id(),
            // self.apps.clone(),
        );
        info!("network {:?} listening on {:?}", self.network_context.network_id(), listen_addr);
        self.state = State::STARTED;
    }

    pub fn network_context(&self) -> NetworkContext {
        self.network_context//.clone()
    }
}

// single function to wrap variants of templated enum
fn transport_peer_manager_start(
    tpm: TransportPeerManager,
    listen_address: NetworkAddress,
    executor: Handle,
    network_id: NetworkId,
    // apps: Arc<ApplicationCollector>,
) -> NetworkAddress {
    let result = match tpm {
        TransportPeerManager::Tcp(pm) => { pm.listen( listen_address,  executor ) }
        #[cfg(any(test, feature = "testing", feature = "fuzzing"))]
        TransportPeerManager::Memory(pm) => { pm.listen( listen_address,  executor ) }
    };
    match result {
        Ok(listen_address) => { listen_address }
        Err(err) => {
            panic!("could not start network {:?}: {:?}", network_id, err);
        }
    }
}


type TcpPeerManager = PeerListener<AptosNetTransport<TcpTransport>, NoiseStream<TcpSocket>>;
#[cfg(any(test, feature = "testing", feature = "fuzzing"))]
type MemoryPeerManager =
PeerListener<AptosNetTransport<MemoryTransport>, NoiseStream<aptos_memsocket::MemorySocket>>;

enum TransportPeerManager {
    Tcp(TcpPeerManager),
    #[cfg(any(test, feature = "testing", feature = "fuzzing"))]
    Memory(MemoryPeerManager),
}
