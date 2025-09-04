// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    application::storage::PeersAndMetadata,
    counters,
    noise::{stream::NoiseStream, HandshakeAuthMode},
    peer_manager::{
        conn_notifs_channel, ConnectionRequest, ConnectionRequestSender, PeerManager,
        PeerManagerRequest, PeerManagerRequestSender,
    },
    protocols::{
        network::{NetworkClientConfig, NetworkServiceConfig, ReceivedMessage},
        wire::handshake::v1::ProtocolIdSet,
    },
    transport::{self, VelorNetTransport, Connection, VELOR_TCP_TRANSPORT},
    ProtocolId,
};
use velor_channels::{self, velor_channel, message_queues::QueueStyle};
use velor_config::{config::HANDSHAKE_VERSION, network_id::NetworkContext};
use velor_crypto::x25519;
use velor_logger::prelude::*;
#[cfg(any(test, feature = "testing", feature = "fuzzing"))]
use velor_netcore::transport::memory::MemoryTransport;
use velor_netcore::transport::{
    tcp::{TCPBufferCfg, TcpSocket, TcpTransport},
    Transport,
};
use velor_time_service::TimeService;
use velor_types::{chain_id::ChainId, network_address::NetworkAddress, PeerId};
use std::{clone::Clone, collections::HashMap, fmt::Debug, sync::Arc};
use tokio::runtime::Handle;

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

struct TransportContext {
    chain_id: ChainId,
    supported_protocols: ProtocolIdSet,
    authentication_mode: AuthenticationMode,
    peers_and_metadata: Arc<PeersAndMetadata>,
    enable_proxy_protocol: bool,
}

impl TransportContext {
    fn add_protocols(&mut self, protocols: &Vec<ProtocolId>) {
        let protocol_id_set = ProtocolIdSet::from_iter(protocols);
        self.supported_protocols = self.supported_protocols.union(&protocol_id_set);
    }
}

struct PeerManagerContext {
    // TODO(philiphayes): better support multiple listening addrs
    pm_reqs_tx: velor_channel::Sender<(PeerId, ProtocolId), PeerManagerRequest>,
    pm_reqs_rx: velor_channel::Receiver<(PeerId, ProtocolId), PeerManagerRequest>,
    connection_reqs_tx: velor_channel::Sender<PeerId, ConnectionRequest>,
    connection_reqs_rx: velor_channel::Receiver<PeerId, ConnectionRequest>,

    peers_and_metadata: Arc<PeersAndMetadata>,
    upstream_handlers:
        HashMap<ProtocolId, velor_channel::Sender<(PeerId, ProtocolId), ReceivedMessage>>,
    connection_event_handlers: Vec<conn_notifs_channel::Sender>,

    channel_size: usize,
    max_frame_size: usize,
    max_message_size: usize,
    inbound_connection_limit: usize,
    tcp_buffer_cfg: TCPBufferCfg,
}

impl PeerManagerContext {
    #[allow(clippy::too_many_arguments)]
    fn new(
        pm_reqs_tx: velor_channel::Sender<(PeerId, ProtocolId), PeerManagerRequest>,
        pm_reqs_rx: velor_channel::Receiver<(PeerId, ProtocolId), PeerManagerRequest>,
        connection_reqs_tx: velor_channel::Sender<PeerId, ConnectionRequest>,
        connection_reqs_rx: velor_channel::Receiver<PeerId, ConnectionRequest>,

        peers_and_metadata: Arc<PeersAndMetadata>,
        upstream_handlers: HashMap<
            ProtocolId,
            velor_channel::Sender<(PeerId, ProtocolId), ReceivedMessage>,
        >,
        connection_event_handlers: Vec<conn_notifs_channel::Sender>,

        channel_size: usize,
        max_frame_size: usize,
        max_message_size: usize,
        inbound_connection_limit: usize,
        tcp_buffer_cfg: TCPBufferCfg,
    ) -> Self {
        Self {
            pm_reqs_tx,
            pm_reqs_rx,
            connection_reqs_tx,
            connection_reqs_rx,

            peers_and_metadata,
            upstream_handlers,
            connection_event_handlers,

            channel_size,
            max_frame_size,
            max_message_size,
            inbound_connection_limit,
            tcp_buffer_cfg,
        }
    }

    fn add_upstream_handler(
        &mut self,
        protocol_id: ProtocolId,
        channel: velor_channel::Sender<(PeerId, ProtocolId), ReceivedMessage>,
    ) -> &mut Self {
        self.upstream_handlers.insert(protocol_id, channel);
        self
    }

    pub fn add_connection_event_listener(&mut self) -> conn_notifs_channel::Receiver {
        let (tx, rx) = conn_notifs_channel::new();
        self.connection_event_handlers.push(tx);
        rx
    }
}

#[cfg(any(test, feature = "testing", feature = "fuzzing"))]
type MemoryPeerManager =
    PeerManager<VelorNetTransport<MemoryTransport>, NoiseStream<velor_memsocket::MemorySocket>>;
type TcpPeerManager = PeerManager<VelorNetTransport<TcpTransport>, NoiseStream<TcpSocket>>;

enum TransportPeerManager {
    #[cfg(any(test, feature = "testing", feature = "fuzzing"))]
    Memory(MemoryPeerManager),
    Tcp(TcpPeerManager),
}

pub struct PeerManagerBuilder {
    network_context: NetworkContext,
    time_service: TimeService,
    transport_context: Option<TransportContext>,
    peer_manager_context: Option<PeerManagerContext>,
    // TODO(philiphayes): better support multiple listening addrs
    peer_manager: Option<TransportPeerManager>,
    // ListenAddress will be updated when the PeerManager is built
    listen_address: NetworkAddress,
}

impl PeerManagerBuilder {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        chain_id: ChainId,
        network_context: NetworkContext,
        time_service: TimeService,
        // TODO(philiphayes): better support multiple listening addrs
        listen_address: NetworkAddress,
        peers_and_metadata: Arc<PeersAndMetadata>,
        authentication_mode: AuthenticationMode,
        channel_size: usize,
        max_frame_size: usize,
        max_message_size: usize,
        enable_proxy_protocol: bool,
        inbound_connection_limit: usize,
        tcp_buffer_cfg: TCPBufferCfg,
    ) -> Self {
        // Setup channel to send requests to peer manager.
        let (pm_reqs_tx, pm_reqs_rx) = velor_channel::new(
            QueueStyle::FIFO,
            channel_size,
            Some(&counters::PENDING_PEER_MANAGER_REQUESTS),
        );
        // Setup channel to send connection requests to peer manager.
        let (connection_reqs_tx, connection_reqs_rx) =
            velor_channel::new(QueueStyle::FIFO, channel_size, None);

        Self {
            network_context,
            time_service,
            transport_context: Some(TransportContext {
                chain_id,
                supported_protocols: ProtocolIdSet::empty(),
                authentication_mode,
                peers_and_metadata: peers_and_metadata.clone(),
                enable_proxy_protocol,
            }),
            peer_manager_context: Some(PeerManagerContext::new(
                pm_reqs_tx,
                pm_reqs_rx,
                connection_reqs_tx,
                connection_reqs_rx,
                peers_and_metadata,
                HashMap::new(),
                Vec::new(),
                channel_size,
                max_frame_size,
                max_message_size,
                inbound_connection_limit,
                tcp_buffer_cfg,
            )),
            peer_manager: None,
            listen_address,
        }
    }

    pub fn listen_address(&self) -> NetworkAddress {
        self.listen_address.clone()
    }

    pub fn connection_reqs_tx(&self) -> velor_channel::Sender<PeerId, ConnectionRequest> {
        self.peer_manager_context
            .as_ref()
            .expect("Cannot access connection_reqs once PeerManager has been built")
            .connection_reqs_tx
            .clone()
    }

    fn transport_context(&mut self) -> &mut TransportContext {
        self.transport_context
            .as_mut()
            .expect("Cannot get TransportContext once PeerManager has been built")
    }

    fn peer_manager_context(&mut self) -> &mut PeerManagerContext {
        self.peer_manager_context
            .as_mut()
            .expect("Cannot get PeerManagerContext once PeerManager has been built")
    }

    /// Create the configured transport and start PeerManager.
    /// Return the actual NetworkAddress over which this peer is listening.
    pub fn build(&mut self, executor: &Handle) -> &mut Self {
        use velor_types::network_address::Protocol::*;

        let transport_context = self
            .transport_context
            .take()
            .expect("PeerManager can only be built once");

        let protos = transport_context.supported_protocols;
        let chain_id = transport_context.chain_id;
        let enable_proxy_protocol = transport_context.enable_proxy_protocol;

        let (key, auth_mode) = match transport_context.authentication_mode {
            AuthenticationMode::MaybeMutual(key) => (
                key,
                HandshakeAuthMode::maybe_mutual(transport_context.peers_and_metadata),
            ),
            AuthenticationMode::Mutual(key) => (
                key,
                HandshakeAuthMode::mutual(transport_context.peers_and_metadata),
            ),
        };

        let mut velor_tcp_transport = VELOR_TCP_TRANSPORT.clone();
        let tcp_cfg = self.get_tcp_buffers_cfg();
        velor_tcp_transport.set_tcp_buffers(&tcp_cfg);

        self.peer_manager = match self.listen_address.as_slice() {
            [Ip4(_), Tcp(_)] | [Ip6(_), Tcp(_)] => {
                Some(TransportPeerManager::Tcp(self.build_with_transport(
                    VelorNetTransport::new(
                        velor_tcp_transport,
                        self.network_context,
                        self.time_service.clone(),
                        key,
                        auth_mode,
                        HANDSHAKE_VERSION,
                        chain_id,
                        protos,
                        enable_proxy_protocol,
                    ),
                    executor,
                )))
            },
            #[cfg(any(test, feature = "testing", feature = "fuzzing"))]
            [Memory(_)] => Some(TransportPeerManager::Memory(self.build_with_transport(
                VelorNetTransport::new(
                    MemoryTransport,
                    self.network_context,
                    self.time_service.clone(),
                    key,
                    auth_mode,
                    HANDSHAKE_VERSION,
                    chain_id,
                    protos,
                    enable_proxy_protocol,
                ),
                executor,
            ))),
            _ => panic!(
                "{} Unsupported listen_address: '{}', expected '/memory/<port>', \
                 '/ip4/<addr>/tcp/<port>', or '/ip6/<addr>/tcp/<port>'.",
                self.network_context, self.listen_address
            ),
        };

        self
    }

    /// Given a transport build and launch PeerManager.
    /// Return the actual NetworkAddress over which this peer is listening.
    fn build_with_transport<TTransport, TSocket>(
        &mut self,
        transport: TTransport,
        executor: &Handle,
    ) -> PeerManager<TTransport, TSocket>
    where
        TTransport: Transport<Output = Connection<TSocket>> + Send + 'static,
        TSocket: transport::TSocket,
    {
        let pm_context = self
            .peer_manager_context
            .take()
            .expect("PeerManager can only be built once");
        let peer_mgr = PeerManager::new(
            executor.clone(),
            self.time_service.clone(),
            transport,
            self.network_context,
            // TODO(philiphayes): peer manager should take `Vec<NetworkAddress>`
            // (which could be empty, like in client use case)
            self.listen_address.clone(),
            pm_context.peers_and_metadata,
            pm_context.pm_reqs_rx,
            pm_context.connection_reqs_rx,
            pm_context.upstream_handlers,
            pm_context.connection_event_handlers,
            pm_context.channel_size,
            pm_context.max_frame_size,
            pm_context.max_message_size,
            pm_context.inbound_connection_limit,
        );

        // PeerManager constructor appends a public key to the listen_address.
        self.listen_address = peer_mgr.listen_addr().clone();

        peer_mgr
    }

    fn start_peer_manager<TTransport, TSocket>(
        &mut self,
        peer_manager: PeerManager<TTransport, TSocket>,
        executor: &Handle,
    ) where
        TTransport: Transport<Output = Connection<TSocket>> + Send + 'static,
        TSocket: transport::TSocket,
    {
        executor.spawn(peer_manager.start());
        debug!("{} Started peer manager", self.network_context);
    }

    pub fn start(&mut self, executor: &Handle) {
        debug!("{} Starting Peer manager", self.network_context);
        match self
            .peer_manager
            .take()
            .expect("Can only start PeerManager once")
        {
            #[cfg(any(test, feature = "testing", feature = "fuzzing"))]
            TransportPeerManager::Memory(pm) => self.start_peer_manager(pm, executor),
            TransportPeerManager::Tcp(pm) => self.start_peer_manager(pm, executor),
        }
    }

    pub fn add_connection_event_listener(&mut self) -> conn_notifs_channel::Receiver {
        self.peer_manager_context
            .as_mut()
            .expect("Cannot add an event listener if PeerManager has already been built.")
            .add_connection_event_listener()
    }

    pub fn get_tcp_buffers_cfg(&self) -> TCPBufferCfg {
        self.peer_manager_context
            .as_ref()
            .expect("Cannot add an event listener if PeerManager has already been built.")
            .tcp_buffer_cfg
    }

    /// Register a client that's interested in some set of protocols and return
    /// the outbound channels into network.
    pub fn add_client(
        &mut self,
        config: &NetworkClientConfig,
    ) -> (PeerManagerRequestSender, ConnectionRequestSender) {
        // Register the direct send and rpc protocols
        self.transport_context()
            .add_protocols(&config.direct_send_protocols_and_preferences);
        self.transport_context()
            .add_protocols(&config.rpc_protocols_and_preferences);

        // Create the context and return the request senders
        let pm_context = self.peer_manager_context();
        (
            PeerManagerRequestSender::new(pm_context.pm_reqs_tx.clone()),
            ConnectionRequestSender::new(pm_context.connection_reqs_tx.clone()),
        )
    }

    /// Register a service for handling some protocols.
    pub fn add_service(
        &mut self,
        config: &NetworkServiceConfig,
    ) -> velor_channel::Receiver<(PeerId, ProtocolId), ReceivedMessage> {
        // Register the direct send and rpc protocols
        self.transport_context()
            .add_protocols(&config.direct_send_protocols_and_preferences);
        self.transport_context()
            .add_protocols(&config.rpc_protocols_and_preferences);

        // Create the context and register the protocols
        let (network_notifs_tx, network_notifs_rx) = config.inbound_queue_config.build();
        let pm_context = self.peer_manager_context();
        for protocol in config
            .direct_send_protocols_and_preferences
            .iter()
            .chain(&config.rpc_protocols_and_preferences)
        {
            pm_context.add_upstream_handler(*protocol, network_notifs_tx.clone());
        }

        network_notifs_rx
    }
}
