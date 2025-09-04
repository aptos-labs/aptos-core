// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Remotely authenticated vs. unauthenticated network end-points:
//! ---------------------------------------------------
//! A network end-point operates with remote authentication if it only accepts connections
//! from a known set of peers (`trusted_peers`) identified by their network identity keys.
//! This does not mean that the other end-point of a connection also needs to operate with
//! authentication -- a network end-point running with remote authentication enabled will
//! connect to or accept connections from an end-point running in authenticated mode as
//! long as the latter is in its trusted peers set.
use velor_config::{
    config::{
        DiscoveryMethod, NetworkConfig, Peer, PeerRole, PeerSet, RoleType, CONNECTION_BACKOFF_BASE,
        CONNECTIVITY_CHECK_INTERVAL_MS, MAX_CONNECTION_DELAY_MS, MAX_FRAME_SIZE,
        MAX_FULLNODE_OUTBOUND_CONNECTIONS, MAX_INBOUND_CONNECTIONS, NETWORK_CHANNEL_SIZE,
    },
    network_id::NetworkContext,
};
use velor_event_notifications::{DbBackedOnChainConfig, EventSubscriptionService};
use velor_logger::prelude::*;
use velor_netcore::transport::tcp::TCPBufferCfg;
use velor_network::{
    application::storage::PeersAndMetadata,
    connectivity_manager::{builder::ConnectivityManagerBuilder, ConnectivityRequest},
    constants::MAX_MESSAGE_SIZE,
    logging::NetworkSchema,
    peer_manager::{
        builder::{AuthenticationMode, PeerManagerBuilder},
        ConnectionRequestSender,
    },
    protocols::{
        health_checker::{self, builder::HealthCheckerBuilder},
        network::{
            NetworkApplicationConfig, NetworkClientConfig, NetworkServiceConfig, NewNetworkEvents,
            NewNetworkSender,
        },
    },
};
use velor_network_discovery::DiscoveryChangeListener;
use velor_time_service::TimeService;
use velor_types::{chain_id::ChainId, network_address::NetworkAddress};
use std::{clone::Clone, collections::HashSet, sync::Arc, time::Duration};
use tokio::runtime::Handle;

#[derive(Debug, PartialEq, PartialOrd)]
enum State {
    CREATED,
    BUILT,
    STARTED,
}

/// Build Network module with custom configuration values.
/// Methods can be chained in order to set the configuration values.
/// MempoolNetworkHandler and ConsensusNetworkHandler are constructed by calling
/// [`NetworkBuilder::build`].  New instances of `NetworkBuilder` are obtained
/// via [`NetworkBuilder::create`].
pub struct NetworkBuilder {
    state: State,
    executor: Option<Handle>,
    time_service: TimeService,
    network_context: NetworkContext,
    discovery_listeners: Option<Vec<DiscoveryChangeListener<DbBackedOnChainConfig>>>,
    connectivity_manager_builder: Option<ConnectivityManagerBuilder>,
    health_checker_builder: Option<HealthCheckerBuilder>,
    peer_manager_builder: PeerManagerBuilder,
    peers_and_metadata: Arc<PeersAndMetadata>,
}

impl NetworkBuilder {
    /// Return a new NetworkBuilder initialized with default configuration values.
    // TODO:  Remove `pub`.  NetworkBuilder should only be created through `::create()`
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        chain_id: ChainId,
        peers_and_metadata: Arc<PeersAndMetadata>,
        network_context: NetworkContext,
        time_service: TimeService,
        listen_address: NetworkAddress,
        authentication_mode: AuthenticationMode,
        max_frame_size: usize,
        max_message_size: usize,
        enable_proxy_protocol: bool,
        network_channel_size: usize,
        inbound_connection_limit: usize,
        tcp_buffer_cfg: TCPBufferCfg,
    ) -> Self {
        // A network cannot exist without a PeerManager
        // TODO:  construct this in create and pass it to new() as a parameter. The complication is manual construction of NetworkBuilder in various tests.
        let peer_manager_builder = PeerManagerBuilder::create(
            chain_id,
            network_context,
            time_service.clone(),
            listen_address,
            peers_and_metadata.clone(),
            authentication_mode,
            network_channel_size,
            max_frame_size,
            max_message_size,
            enable_proxy_protocol,
            inbound_connection_limit,
            tcp_buffer_cfg,
        );

        NetworkBuilder {
            state: State::CREATED,
            executor: None,
            time_service,
            network_context,
            discovery_listeners: None,
            connectivity_manager_builder: None,
            health_checker_builder: None,
            peer_manager_builder,
            peers_and_metadata,
        }
    }

    pub fn new_for_test(
        chain_id: ChainId,
        seeds: PeerSet,
        network_context: NetworkContext,
        time_service: TimeService,
        listen_address: NetworkAddress,
        authentication_mode: AuthenticationMode,
        peers_and_metadata: Arc<PeersAndMetadata>,
    ) -> NetworkBuilder {
        let mutual_authentication = matches!(authentication_mode, AuthenticationMode::Mutual(_));

        let mut builder = NetworkBuilder::new(
            chain_id,
            peers_and_metadata.clone(),
            network_context,
            time_service,
            listen_address,
            authentication_mode,
            MAX_FRAME_SIZE,
            MAX_MESSAGE_SIZE,
            false, /* Disable proxy protocol */
            NETWORK_CHANNEL_SIZE,
            MAX_INBOUND_CONNECTIONS,
            TCPBufferCfg::default(),
        );

        builder.add_connectivity_manager(
            seeds,
            peers_and_metadata,
            MAX_FULLNODE_OUTBOUND_CONNECTIONS,
            CONNECTION_BACKOFF_BASE,
            MAX_CONNECTION_DELAY_MS,
            CONNECTIVITY_CHECK_INTERVAL_MS,
            NETWORK_CHANNEL_SIZE,
            mutual_authentication,
            true, /* enable_latency_aware_dialing */
        );

        builder
    }

    /// Create a new NetworkBuilder based on the provided configuration.
    pub fn create(
        chain_id: ChainId,
        role: RoleType,
        config: &NetworkConfig,
        time_service: TimeService,
        reconfig_subscription_service: Option<&mut EventSubscriptionService>,
        peers_and_metadata: Arc<PeersAndMetadata>,
    ) -> NetworkBuilder {
        let peer_id = config.peer_id();
        let identity_key = config.identity_key();

        let authentication_mode = if config.mutual_authentication {
            AuthenticationMode::Mutual(identity_key)
        } else {
            AuthenticationMode::MaybeMutual(identity_key)
        };

        let network_context = NetworkContext::new(role, config.network_id, peer_id);

        let mut network_builder = NetworkBuilder::new(
            chain_id,
            peers_and_metadata.clone(),
            network_context,
            time_service,
            config.listen_address.clone(),
            authentication_mode,
            config.max_frame_size,
            config.max_message_size,
            config.enable_proxy_protocol,
            config.network_channel_size,
            config.max_inbound_connections,
            TCPBufferCfg::new_configs(
                config.inbound_rx_buffer_size_bytes,
                config.inbound_tx_buffer_size_bytes,
                config.outbound_rx_buffer_size_bytes,
                config.outbound_tx_buffer_size_bytes,
            ),
        );

        network_builder.add_connection_monitoring(
            config.ping_interval_ms,
            config.ping_timeout_ms,
            config.ping_failures_tolerated,
            config.max_parallel_deserialization_tasks,
        );

        // Always add a connectivity manager to keep track of known peers
        let seeds = merge_seeds(config);

        network_builder.add_connectivity_manager(
            seeds,
            peers_and_metadata,
            config.max_outbound_connections,
            config.connection_backoff_base,
            config.max_connection_delay_ms,
            config.connectivity_check_interval_ms,
            config.network_channel_size,
            config.mutual_authentication,
            config.enable_latency_aware_dialing,
        );

        network_builder.discovery_listeners = Some(Vec::new());
        network_builder.setup_discovery(config, reconfig_subscription_service);

        // Ensure there are no duplicate source types
        let set: HashSet<_> = network_builder
            .discovery_listeners
            .as_ref()
            .unwrap()
            .iter()
            .map(|listener| listener.discovery_source())
            .collect();
        assert_eq!(
            set.len(),
            network_builder.discovery_listeners.as_ref().unwrap().len()
        );

        network_builder
    }

    /// Create the configured Networking components.
    pub fn build(&mut self, executor: Handle) -> &mut Self {
        assert_eq!(self.state, State::CREATED);
        self.state = State::BUILT;
        self.executor = Some(executor);
        self.peer_manager_builder
            .build(self.executor.as_mut().expect("Executor must exist"));
        self
    }

    /// Start the built Networking components.
    pub fn start(&mut self) -> &mut Self {
        assert_eq!(self.state, State::BUILT);
        self.state = State::STARTED;

        let executor = self.executor.as_mut().expect("Executor must exist");
        self.peer_manager_builder.start(executor);
        debug!(
            NetworkSchema::new(&self.network_context),
            "{} Started peer manager", self.network_context
        );

        if let Some(conn_mgr_builder) = self.connectivity_manager_builder.as_mut() {
            conn_mgr_builder.start(executor);
            debug!(
                NetworkSchema::new(&self.network_context),
                "{} Started conn manager", self.network_context
            );
        }

        if let Some(health_checker_builder) = self.health_checker_builder.as_mut() {
            health_checker_builder.start(executor);
            debug!(
                NetworkSchema::new(&self.network_context),
                "{} Started health checker", self.network_context
            );
        }

        if let Some(discovery_listeners) = self.discovery_listeners.take() {
            discovery_listeners
                .into_iter()
                .for_each(|listener| listener.start(executor))
        }
        self
    }

    pub fn network_context(&self) -> NetworkContext {
        self.network_context
    }

    pub fn conn_mgr_reqs_tx(&self) -> Option<velor_channels::Sender<ConnectivityRequest>> {
        self.connectivity_manager_builder
            .as_ref()
            .map(|conn_mgr_builder| conn_mgr_builder.conn_mgr_reqs_tx())
    }

    pub fn listen_address(&self) -> NetworkAddress {
        self.peer_manager_builder.listen_address()
    }

    /// Add a `network::connectivity_manager::ConnectivityManager` to the network.
    ///
    /// `network::connectivity_manager::ConnectivityManager` is responsible for ensuring that we are connected
    /// to a node iff. it is an eligible node and maintaining persistent
    /// connections with all eligible nodes. A list of eligible nodes is received
    /// at initialization, and updates are received on changes to system membership.
    ///
    /// Note: a connectivity manager should only be added if the network is
    /// permissioned.
    pub fn add_connectivity_manager(
        &mut self,
        seeds: PeerSet,
        peers_and_metadata: Arc<PeersAndMetadata>,
        max_outbound_connections: usize,
        connection_backoff_base: u64,
        max_connection_delay_ms: u64,
        connectivity_check_interval_ms: u64,
        channel_size: usize,
        mutual_authentication: bool,
        enable_latency_aware_dialing: bool,
    ) -> &mut Self {
        let pm_conn_mgr_notifs_rx = self.peer_manager_builder.add_connection_event_listener();
        let outbound_connection_limit = if !self.network_context.network_id().is_validator_network()
        {
            Some(max_outbound_connections)
        } else {
            None
        };

        self.connectivity_manager_builder = Some(ConnectivityManagerBuilder::create(
            self.network_context(),
            self.time_service.clone(),
            peers_and_metadata,
            seeds,
            connectivity_check_interval_ms,
            connection_backoff_base,
            max_connection_delay_ms,
            channel_size,
            ConnectionRequestSender::new(self.peer_manager_builder.connection_reqs_tx()),
            pm_conn_mgr_notifs_rx,
            outbound_connection_limit,
            mutual_authentication,
            enable_latency_aware_dialing,
        ));
        self
    }

    fn setup_discovery(
        &mut self,
        config: &NetworkConfig,
        mut reconfig_subscription_service: Option<&mut EventSubscriptionService>,
    ) {
        let conn_mgr_reqs_tx = self
            .conn_mgr_reqs_tx()
            .expect("ConnectivityManager must exist");
        for discovery_method in config.discovery_methods() {
            let listener = match discovery_method {
                DiscoveryMethod::Onchain => {
                    let reconfig_events = reconfig_subscription_service
                        .as_mut()
                        .expect("An event subscription service is required for on-chain discovery!")
                        .subscribe_to_reconfigurations()
                        .expect("On-chain discovery is unable to subscribe to reconfigurations!");
                    let identity_key = config.identity_key();
                    let pubkey = identity_key.public_key();
                    DiscoveryChangeListener::validator_set(
                        self.network_context,
                        conn_mgr_reqs_tx.clone(),
                        pubkey,
                        reconfig_events,
                    )
                },
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
                },
            };
            self.discovery_listeners
                .as_mut()
                .expect("Can only add listeners before starting")
                .push(listener);
        }
    }

    /// Add a HealthChecker to the network.
    fn add_connection_monitoring(
        &mut self,
        ping_interval_ms: u64,
        ping_timeout_ms: u64,
        ping_failures_tolerated: u64,
        max_parallel_deserialization_tasks: Option<usize>,
    ) -> &mut Self {
        // Initialize and start HealthChecker.
        let (hc_network_tx, hc_network_rx) = self.add_client_and_service(
            &health_checker::health_checker_network_config(),
            max_parallel_deserialization_tasks,
            true,
        );
        self.health_checker_builder = Some(HealthCheckerBuilder::new(
            self.network_context(),
            self.time_service.clone(),
            ping_interval_ms,
            ping_timeout_ms,
            ping_failures_tolerated,
            hc_network_tx,
            hc_network_rx,
            self.peers_and_metadata.clone(),
        ));
        debug!(
            NetworkSchema::new(&self.network_context),
            "{} Created health checker", self.network_context
        );
        self
    }

    /// Register a new client and service application with the network. Return
    /// the client interface for sending messages and the service interface
    /// for handling network requests.
    pub fn add_client_and_service<SenderT: NewNetworkSender, EventsT: NewNetworkEvents>(
        &mut self,
        config: &NetworkApplicationConfig,
        max_parallel_deserialization_tasks: Option<usize>,
        allow_out_of_order_delivery: bool,
    ) -> (SenderT, EventsT) {
        (
            self.add_client(&config.network_client_config),
            self.add_service(
                &config.network_service_config,
                max_parallel_deserialization_tasks,
                allow_out_of_order_delivery,
            ),
        )
    }

    /// Register a new client application with the network. Return the client
    /// interface for sending messages.
    fn add_client<SenderT: NewNetworkSender>(&mut self, config: &NetworkClientConfig) -> SenderT {
        let (peer_mgr_reqs_tx, connection_reqs_tx) = self.peer_manager_builder.add_client(config);
        SenderT::new(peer_mgr_reqs_tx, connection_reqs_tx)
    }

    /// Register a new service application with the network. Return the service
    /// interface for handling network requests.
    // TODO(philiphayes): return new NetworkService (name TBD) interface?
    fn add_service<EventsT: NewNetworkEvents>(
        &mut self,
        config: &NetworkServiceConfig,
        max_parallel_deserialization_tasks: Option<usize>,
        allow_out_of_order_delivery: bool,
    ) -> EventsT {
        let peer_mgr_reqs_rx = self.peer_manager_builder.add_service(config);
        EventsT::new(
            peer_mgr_reqs_rx,
            max_parallel_deserialization_tasks,
            allow_out_of_order_delivery,
        )
    }
}

/// Retrieve and merge seeds so that they have all keys associated
fn merge_seeds(config: &NetworkConfig) -> PeerSet {
    config.verify_seeds().expect("Seeds must be well formed");
    let mut seeds = config.seeds.clone();

    // Merge old seed configuration with new seed configuration
    // TODO(gnazario): Once fully migrated, remove `seed_addrs`
    config
        .seed_addrs
        .iter()
        .map(|(peer_id, addrs)| {
            (
                peer_id,
                Peer::from_addrs(PeerRole::ValidatorFullNode, addrs.clone()),
            )
        })
        .for_each(|(peer_id, peer)| {
            seeds
                .entry(*peer_id)
                // Sad clone due to Rust not realizing these are two distinct paths
                .and_modify(|seed| seed.extend(peer.clone()).unwrap())
                .or_insert(peer);
        });

    // Pull public keys out of addresses
    seeds.values_mut().for_each(
        |Peer {
             addresses, keys, ..
         }| {
            addresses
                .iter()
                .filter_map(NetworkAddress::find_noise_proto)
                .for_each(|pubkey| {
                    keys.insert(pubkey);
                });
        },
    );
    seeds
}
