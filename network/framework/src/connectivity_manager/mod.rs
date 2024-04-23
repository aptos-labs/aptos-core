// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! The ConnectivityManager actor is responsible for ensuring that we are
//! connected to a node if and only if it is an eligible node.
//!
//! A list of eligible nodes is received at initialization, and updates are
//! received on changes to system membership. In our current system design, the
//! Consensus actor informs the ConnectivityManager of eligible nodes.
//!
//! Different discovery sources notify the ConnectivityManager of updates to
//! peers' addresses. Currently, there are 2 discovery sources (ordered by
//! decreasing dial priority, i.e., first is highest priority):
//!
//! 1. Onchain discovery protocol
//! 2. Seed peers from config
//!
//! In other words, if a we have some addresses discovered via onchain discovery
//! and some seed addresses from our local config, we will try the onchain
//! discovery addresses first and the local seed addresses after.
//!
//! When dialing a peer with a given list of addresses, we attempt each address
//! in order with a capped exponential backoff delay until we eventually connect
//! to the peer. The backoff is capped since, for validators specifically, it is
//! absolutely important that we maintain connectivity with all peers and heal
//! any partitions asap, as we aren't currently gossiping consensus messages or
//! using a relay protocol.

use crate::{
    application::{
        storage::{ConnectionNotification, PeersAndMetadata},
        ApplicationCollector,
    },
    counters,
    logging::NetworkSchema,
    protocols::network::{OutboundPeerConnections, PeerStub},
    transport::AptosNetTransportActual,
};
use aptos_config::{
    config::{NetworkConfig, Peer, PeerRole, PeerSet},
    network_id::{NetworkContext, PeerNetworkId},
};
use aptos_crypto::x25519;
use aptos_infallible::RwLock;
use aptos_logger::prelude::*;
use aptos_netcore::transport::ConnectionOrigin;
use aptos_num_variants::NumVariants;
use aptos_short_hex_str::AsShortHexStr;
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_types::{account_address::AccountAddress, network_address::NetworkAddress, PeerId};
use futures::{
    channel::oneshot,
    future::{BoxFuture, FutureExt},
    stream::{FuturesUnordered, StreamExt},
};
use futures_util::{future::join_all, stream::Fuse};
use itertools::Itertools;
use ordered_float::OrderedFloat;
use rand_latest::Rng;
use serde::Serialize;
use std::{
    cmp::{min, Ordering},
    collections::{hash_map::Entry, HashMap, HashSet},
    fmt, mem,
    net::{Shutdown, TcpStream, ToSocketAddrs},
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};
use tokio::{runtime::Handle, task::JoinHandle};
use tokio_retry::strategy::jitter;
use tokio_stream::wrappers::ReceiverStream;

mod selection;

#[cfg(test)]
mod test;

/// In addition to the backoff strategy, we also add some small random jitter to
/// the delay before each dial. This jitter helps reduce the probability of
/// simultaneous dials, especially in non-production environments where most nodes
/// are spun up around the same time. Similarly, it smears the dials out in time
/// to avoid spiky load / thundering herd issues where all dial requests happen
/// around the same time at startup.
const MAX_CONNECTION_DELAY_JITTER: Duration = Duration::from_millis(100);

/// The maximum amount of time to wait before timing out a connection attempt.
/// This should be relatively small to avoid blocking dials for too long.
const MAX_CONNECTION_TIMEOUT_SECS: u64 = 2;

/// The maximum number of socket addresses to ping for a single address
const MAX_SOCKET_ADDRESSES_TO_PING: usize = 2;

/// The amount of time to try other peers until dialing this peer again.
///
/// It's currently set to 5 minutes to ensure rotation through all (or most) peers
const TRY_DIAL_BACKOFF_TIME: Duration = Duration::from_secs(300);

/// The ConnectivityManager actor.
pub struct ConnectivityManager<TBackoff> {
    config: NetworkConfig,
    network_context: NetworkContext,
    /// A handle to a time service for easily mocking time-related operations.
    time_service: TimeService,
    /// Peers and metadata
    peers_and_metadata: Arc<PeersAndMetadata>,
    /// All information about peers from discovery sources.
    discovered_peers: Arc<RwLock<DiscoveredPeerSet>>,
    /// Channel over which we receive requests from other actors.
    requests_rx: Fuse<ReceiverStream<ConnectivityRequest>>, //futures::Stream<Item=ConnectivityRequest>,
    /// Peers queued to be dialed, potentially with some delay. The dial can be canceled by
    /// sending over (or dropping) the associated oneshot sender.
    dial_queue: HashMap<PeerId, oneshot::Sender<()>>,
    /// The state of any currently executing dials. Used to keep track of what
    /// the next dial delay and dial address should be for a given peer.
    dial_states: HashMap<PeerId, DialState<TBackoff>>,
    /// Trigger connectivity checks every interval.
    connectivity_check_interval: Duration,
    /// Backoff strategy.
    backoff_strategy: TBackoff,
    /// Maximum delay b/w 2 consecutive attempts to connect with a disconnected peer.
    max_delay: Duration,
    /// A local counter incremented on receiving an incoming message. Printing this in debugging
    /// allows for easy debugging.
    event_id: u32,
    /// A way to limit the number of connected peers by outgoing dials.
    outbound_connection_limit: Option<usize>,
    /// how to connect to new peers
    transport: AptosNetTransportActual,
    /// routing by ProtocolId to application code, for passing to created peers
    apps: Arc<ApplicationCollector>,
    /// for created peers
    peer_senders: Arc<OutboundPeerConnections>,
    peer_senders_cache: HashMap<PeerNetworkId, PeerStub>,
    peer_senders_generation: u32,
    /// Whether or not to enable latency aware peer dialing
    enable_latency_aware_dialing: bool,
}

/// Different sources for peer addresses, ordered by priority (Onchain=highest,
/// Config=lowest).
#[repr(u8)]
#[derive(Copy, Clone, Eq, Hash, PartialEq, Ord, PartialOrd, NumVariants, Serialize)]
pub enum DiscoverySource {
    OnChainValidatorSet,
    File,
    Rest,
    Config,
}

impl fmt::Debug for DiscoverySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for DiscoverySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            DiscoverySource::OnChainValidatorSet => "OnChainValidatorSet",
            DiscoverySource::File => "File",
            DiscoverySource::Config => "Config",
            DiscoverySource::Rest => "Rest",
        })
    }
}

/// Requests received by the [`ConnectivityManager`] manager actor from upstream modules.
#[derive(Debug, Serialize)]
pub enum ConnectivityRequest {
    /// Update set of discovered peers and associated info
    UpdateDiscoveredPeers(DiscoverySource, PeerSet),
    /// Gets current size of connected peers. Only used in test code.
    #[serde(skip)]
    GetConnectedSize(oneshot::Sender<usize>),
    /// Gets current size of dial queue. Only used in test code.
    #[serde(skip)]
    GetDialQueueSize(oneshot::Sender<usize>),
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
struct DiscoveredPeerSet {
    peer_set: HashMap<PeerId, DiscoveredPeer>,
}

impl DiscoveredPeerSet {
    #[cfg(test)]
    /// Creates a new discovered peer set from the
    /// specified peer set. This is used for testing.
    pub fn new_from_peer_set(peer_set: HashMap<PeerId, DiscoveredPeer>) -> Self {
        Self { peer_set }
    }

    /// Gets the eligible peers from the discovered peer set
    fn get_eligible_peers(&self) -> PeerSet {
        self.peer_set
            .iter()
            .filter(|(_, peer)| peer.is_eligible())
            .map(|(peer_id, peer)| (*peer_id, peer.into()))
            .collect()
    }

    /// Removes the specified peer from the set if the state is empty
    fn remove_peer_if_empty(&mut self, peer_id: &PeerId) {
        if let Entry::Occupied(entry) = self.peer_set.entry(*peer_id) {
            if entry.get().is_empty() {
                entry.remove();
            }
        }
    }

    /// Updates the last dial time for the specified peer (if one was found)
    fn update_last_dial_time(&mut self, peer_id: &PeerId) {
        if let Some(discovered_peer) = self.peer_set.get_mut(peer_id) {
            discovered_peer.update_last_dial_time()
        }
    }

    /// Returns the ping latency for the specified peer (if one was found)
    fn get_ping_latency_secs(&self, peer_id: &PeerId) -> Option<f64> {
        if let Some(discovered_peer) = self.peer_set.get(peer_id) {
            discovered_peer.ping_latency_secs
        } else {
            None
        }
    }

    /// Updates the ping latency for the specified peer (if one was found)
    fn update_ping_latency_secs(&mut self, peer_id: &PeerId, latency_secs: f64) {
        if let Some(discovered_peer) = self.peer_set.get_mut(peer_id) {
            discovered_peer.set_ping_latency_secs(latency_secs)
        }
    }
}

/// Represents all the information for a discovered peer
#[derive(Clone, Debug, PartialEq, Serialize)]
struct DiscoveredPeer {
    role: PeerRole,
    addrs: Addresses,
    keys: PublicKeys,
    /// The last time the node was dialed
    last_dial_time: SystemTime,
    /// The calculated peer ping latency (secs)
    ping_latency_secs: Option<f64>,
}

impl DiscoveredPeer {
    pub fn new(role: PeerRole) -> Self {
        Self {
            role,
            addrs: Addresses::default(),
            keys: PublicKeys::default(),
            last_dial_time: SystemTime::UNIX_EPOCH,
            ping_latency_secs: None,
        }
    }

    /// Peers without keys are not able to be mutually authenticated to
    pub fn is_eligible(&self) -> bool {
        !self.keys.is_empty()
    }

    /// Peers without addresses can't be dialed to
    pub fn is_eligible_to_be_dialed(&self) -> bool {
        self.is_eligible() && !self.addrs.is_empty()
    }

    /// Returns true iff the peer's addresses and keys are empty
    pub fn is_empty(&self) -> bool {
        self.addrs.is_empty() && self.keys.is_empty()
    }

    /// Updates the last time we tried to connect to this node
    pub fn update_last_dial_time(&mut self) {
        self.last_dial_time = SystemTime::now();
    }

    /// Updates the ping latency for this peer
    pub fn set_ping_latency_secs(&mut self, latency_secs: f64) {
        self.ping_latency_secs = Some(latency_secs);
    }

    /// Based on input, backoff on amount of time to dial a peer again
    pub fn has_dialed_recently(&self) -> bool {
        if let Ok(duration_since_last_dial) = self.last_dial_time.elapsed() {
            duration_since_last_dial < TRY_DIAL_BACKOFF_TIME
        } else {
            false
        }
    }
}

impl PartialOrd for DiscoveredPeer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let self_dialed_recently = self.has_dialed_recently();
        let other_dialed_recently = other.has_dialed_recently();

        // Less recently dialed is prioritized over recently dialed
        if !self_dialed_recently && other_dialed_recently {
            Some(Ordering::Less)
        } else if self_dialed_recently && !other_dialed_recently {
            Some(Ordering::Greater)
        } else {
            self.role.partial_cmp(&other.role)
        }
    }
}

impl From<&DiscoveredPeer> for Peer {
    fn from(peer: &DiscoveredPeer) -> Self {
        Peer::new(peer.addrs.union(), peer.keys.union(), peer.role)
    }
}

/// A set of `NetworkAddress`'s for a single peer, bucketed by DiscoverySource in
/// priority order.
#[derive(Clone, Default, PartialEq, Serialize)]
struct Addresses([Vec<NetworkAddress>; DiscoverySource::NUM_VARIANTS]);

/// Sets of `x25519::PublicKey`s for a single peer, bucketed by DiscoverySource
/// in priority order.
#[derive(Clone, Default, PartialEq, Serialize)]
struct PublicKeys([HashSet<x25519::PublicKey>; DiscoverySource::NUM_VARIANTS]);

#[derive(Debug)]
enum DialResult {
    Success,
    Cancelled,
    _AlreadyConnected,
    Failed,
    // Failed(PeerManagerError),
}

/// The state needed to compute the next dial delay and dial addr for a given
/// peer.
#[derive(Debug, Clone)]
struct DialState<TBackoff> {
    /// The current state of this peer's backoff delay.
    backoff: TBackoff,
    /// The index of the next address to dial. Index of an address in the `DiscoveredPeer`'s
    /// `addrs` entry.
    addr_idx: usize,
}

/////////////////////////
// ConnectivityManager //
/////////////////////////

impl<TBackoff> ConnectivityManager<TBackoff>
where
    TBackoff: Iterator<Item = Duration> + Clone,
{
    /// Creates a new instance of the [`ConnectivityManager`] actor.
    pub fn new(
        config: NetworkConfig,
        network_context: NetworkContext,
        time_service: TimeService,
        peers_and_metadata: Arc<PeersAndMetadata>,
        seeds: PeerSet,
        requests_rx: tokio::sync::mpsc::Receiver<ConnectivityRequest>,
        backoff_strategy: TBackoff,
        transport: AptosNetTransportActual,
        apps: Arc<ApplicationCollector>,
        peer_senders: Arc<OutboundPeerConnections>,
    ) -> Self {
        let connectivity_check_interval =
            Duration::from_millis(config.connectivity_check_interval_ms);
        let max_delay = Duration::from_millis(config.max_connection_delay_ms);
        let outbound_connection_limit = if network_context.network_id().is_validator_network() {
            None
        } else {
            Some(config.max_outbound_connections)
        };
        let enable_latency_aware_dialing = config.enable_latency_aware_dialing;

        // Verify that the trusted peers set exists and that it is empty
        let trusted_peers = peers_and_metadata
            .get_trusted_peers(&network_context.network_id())
            .unwrap_or_else(|error| {
                panic!("Trusted peers must exist, but found error: {:?}", error)
            });
        assert!(
            trusted_peers.read().is_empty(),
            "Trusted peers must be empty. Found: {:?}",
            trusted_peers
        );

        info!(
            NetworkSchema::new(&network_context),
            "{} Initialized connectivity manager", network_context
        );

        let requests_rx = ReceiverStream::new(requests_rx).fuse();

        let mut connmgr = Self {
            config,
            network_context,
            time_service,
            peers_and_metadata,
            discovered_peers: Arc::new(RwLock::new(DiscoveredPeerSet::default())),
            requests_rx,
            dial_queue: HashMap::new(),
            dial_states: HashMap::new(),
            connectivity_check_interval,
            backoff_strategy,
            max_delay,
            event_id: 0,
            outbound_connection_limit,
            transport,
            apps,
            peer_senders,
            peer_senders_cache: HashMap::new(),
            peer_senders_generation: 0,
            enable_latency_aware_dialing,
        };

        // Set the initial seed config addresses and public keys
        connmgr.handle_update_discovered_peers(DiscoverySource::Config, seeds);
        connmgr
    }

    /// Starts the [`ConnectivityManager`] actor.
    pub async fn start(mut self, handle: Handle) {
        // The ConnectivityManager actor is interested in 3 kinds of events:
        // 1. Ticks to trigger connecitvity check. These are implemented using a clock based
        //    trigger in production.
        // 2. Incoming requests to connect or disconnect with a peer.
        // 3. Notifications from PeerManager when we establish a new connection or lose an existing
        //    connection with a peer.
        let mut pending_dials = FuturesUnordered::new();

        let ticker = self.time_service.interval(self.connectivity_check_interval);
        tokio::pin!(ticker);

        info!(
            NetworkSchema::new(&self.network_context),
            "{} Starting ConnectivityManager actor", self.network_context
        );

        let connection_notifs_rx = self.peers_and_metadata.subscribe();
        let mut connection_notifs_rx = ReceiverStream::new(connection_notifs_rx).fuse();

        loop {
            self.event_id = self.event_id.wrapping_add(1);
            futures::select! {
                _ = ticker.select_next_some() => {
                    info!(
                        NetworkSchema::new(&self.network_context),
                        "tick check_connectivity",
                    );
                    self.check_connectivity(&mut pending_dials, &handle).await;
                },
                req = self.requests_rx.select_next_some() => {
                    self.handle_request(req);
                },
                maybe_notif = connection_notifs_rx.next() => {
                    // Shutdown the connectivity manager when the PeerManager
                    // shuts down.
                    match maybe_notif {
                        Some(notif) => {
                            self.handle_control_notification(notif.clone());
                        },
                        None => break,
                    }
                },
                peer_id = pending_dials.select_next_some() => {
                    trace!(
                        NetworkSchema::new(&self.network_context)
                            .remote_peer(&peer_id),
                        "{} Dial complete to {}",
                        self.network_context,
                        peer_id.short_str(),
                    );
                    self.dial_queue.remove(&peer_id);
                },
            }
        }

        warn!(
            NetworkSchema::new(&self.network_context),
            "{} ConnectivityManager actor terminated", self.network_context
        );
    }

    #[cfg(test)]
    pub async fn test_start(self) {
        self.start(Handle::current()).await
    }

    /// Returns the trusted peers for the current network context.
    /// If no set exists, an error is logged and None is returned.
    fn get_trusted_peers(&self) -> Option<Arc<RwLock<PeerSet>>> {
        let network_id = self.network_context.network_id();
        match self.peers_and_metadata.get_trusted_peers(&network_id) {
            Ok(trusted_peers) => Some(trusted_peers),
            Err(error) => {
                error!(
                    NetworkSchema::new(&self.network_context),
                    "Failed to find trusted peers for network context: {:?}, error: {:?}",
                    self.network_context,
                    error
                );
                None
            },
        }
    }

    /// Disconnect from all peers that are no longer eligible.
    ///
    /// For instance, a validator might leave the validator set after a
    /// reconfiguration. If we are currently connected to this validator, calling
    /// this function will close our connection to it.
    async fn close_stale_connections(&mut self) {
        // see disable below...
        if let Some(trusted_peers) = self.get_trusted_peers() {
            // Identify stale peer connections
            let trusted_peers = trusted_peers.read().clone();
            let pam_all = self.peers_and_metadata.get_all_peers_and_metadata();
            for (_network_id, netpeers) in pam_all.iter() {
                for (peer_id, metadata) in netpeers.iter() {
                    if !metadata.is_connected() {
                        continue;
                    }
                    if trusted_peers.contains_key(peer_id) {
                        continue; // trusted is never stale
                    }
                    if !self.config.mutual_authentication
                        && metadata.connection_metadata.origin == ConnectionOrigin::Inbound
                        && (metadata.connection_metadata.role == PeerRole::ValidatorFullNode
                            || metadata.connection_metadata.role == PeerRole::Unknown)
                    {
                        // aka
                        // IF (not in trusted set) AND ((mutual auth on) OR (outbound connection) OR (role is other than {VFN, Unknown})) THEN STALE
                        continue; // not stale
                    }

                    // is stale! Close...

                    match self
                        .peer_senders
                        .get_generational(self.peer_senders_generation)
                    {
                        None => {},
                        Some((new_peer_senders, new_generation)) => {
                            self.peer_senders_cache = new_peer_senders;
                            self.peer_senders_generation = new_generation;
                        },
                    }
                    #[cfg(disabled)] // TODO: actually closing 'stale' is disabled until fixed
                    match self.peer_senders_cache.get(peer_network_id) {
                        None => {
                            // already gone, nothing to do
                        },
                        Some(stub) => {
                            info!(
                                NetworkSchema::new(&self.network_context)
                                    .remote_peer(&peer_network_id.peer_id()),
                                net = self.network_context,
                                peer = peer_network_id,
                                op = "stale",
                                trusted = trusted_peers,
                                metadata = metadata,
                                "peerclose"
                            );
                            stub.close.close().await;
                        },
                    }
                }
            }
        }
    }

    /// Cancel all pending dials to peers that are no longer eligible.
    ///
    /// For instance, a validator might leave the validator set after a
    /// reconfiguration. If there is a pending dial to this validator, calling
    /// this function will remove it from the dial queue.
    async fn cancel_stale_dials(&mut self) {
        if let Some(trusted_peers) = self.get_trusted_peers() {
            // Identify stale peer dials
            let trusted_peers = trusted_peers.read().clone();
            let stale_peer_dials: Vec<AccountAddress> = self
                .dial_queue
                .keys()
                .filter(|peer_id| !trusted_peers.contains_key(peer_id))
                .cloned()
                .collect();

            // info!("{:?} stale dials to cancel", stale_peer_dials.len());
            // Remove the stale dials from the dial queue
            for stale_peer_dial in stale_peer_dials {
                debug!(
                    NetworkSchema::new(&self.network_context).remote_peer(&stale_peer_dial),
                    "{} Cancelling stale dial {}",
                    self.network_context,
                    stale_peer_dial.short_str()
                );
                self.dial_queue.remove(&stale_peer_dial).map(|x| x.send(()));
            }
        }
    }

    /// Identifies a set of peers to dial and queues them for dialing
    async fn dial_eligible_peers<'a>(
        &'a mut self,
        pending_dials: &'a mut FuturesUnordered<BoxFuture<'static, PeerId>>,
        handle: &Handle,
    ) {
        let to_connect = self.choose_peers_to_dial().await;
        info!(
            NetworkSchema::new(&self.network_context),
            "dial_eligible_peers found {:?} to connect to",
            to_connect.len(),
        );
        for (peer_id, peer) in to_connect {
            self.queue_dial_peer(peer_id, peer, pending_dials, handle);
        }
    }

    fn has_connected_peer(&self, peer_network_id: &PeerNetworkId) -> bool {
        if let Ok(metadata) = self
            .peers_and_metadata
            .get_metadata_for_peer(*peer_network_id)
        {
            metadata.is_connected()
        } else {
            false
        }
    }

    /// Selects a set of peers to dial
    async fn choose_peers_to_dial(&mut self) -> Vec<(PeerId, DiscoveredPeer)> {
        // Get the eligible peers to dial
        let network_id = self.network_context.network_id();
        let role = self.network_context.role();
        let roles_to_dial = network_id.upstream_roles(&role);
        let discovered_peers = self.discovered_peers.read().peer_set.clone();
        let num_discovered = discovered_peers.len();
        let mut eligible_peers = Vec::new();
        let mut ineligible: isize = 0;
        let mut already_connected: isize = 0;
        let mut already_in_dial_queue: isize = 0;
        let mut wrong_role: isize = 0;
        for (peer_id, peer) in discovered_peers.into_iter() {
            if !peer.is_eligible_to_be_dialed() {
                ineligible += 1;
                continue;
            }
            let peer_network_id = PeerNetworkId::new(network_id, peer_id);
            if self.has_connected_peer(&peer_network_id) {
                already_connected += 1;
                continue;
            }
            if self.dial_queue.contains_key(&peer_id) {
                already_in_dial_queue += 1;
                continue;
            }
            if !roles_to_dial.contains(&peer.role) {
                wrong_role += 1;
                continue;
            }
            eligible_peers.push((peer_id, peer));
        }
        // aptos_logger::sample!(
        //     aptos_logger::sample::SampleRate::Frequency(10),
        info!(
            NetworkSchema::new(&self.network_context),
            "peers: {} discovered, {} eligible, {} ineligible, {} already connected, {} already in dial queue, {} wrong role", num_discovered, eligible_peers.len(), ineligible, already_connected, already_in_dial_queue, wrong_role,
        );
        // );

        // Initialize the dial state for any new peers
        for (peer_id, _) in &eligible_peers {
            self.dial_states
                .entry(*peer_id)
                .or_insert_with(|| DialState::new(self.backoff_strategy.clone()));
        }

        // Limit the number of dialed connections from a fullnode. Note: this does not
        // limit the number of incoming connections. It only enforces that a fullnode
        // cannot have more outgoing connections than the limit (including in-flight dials).
        let num_eligible_peers = eligible_peers.len();
        let num_peers_to_dial =
            if let Some(outbound_connection_limit) = self.outbound_connection_limit {
                // Get the number of outbound connections
                let num_outbound_connections = self
                    .peers_and_metadata
                    .count_connected_peers(Some(ConnectionOrigin::Outbound));

                // Add any pending dials to the count
                let total_outbound_connections =
                    num_outbound_connections.saturating_add(self.dial_queue.len());

                // Calculate the potential number of peers to dial
                let num_peers_to_dial =
                    outbound_connection_limit.saturating_sub(total_outbound_connections);

                // Limit the number of peers to dial by the total number of eligible peers
                min(num_peers_to_dial, num_eligible_peers)
            } else {
                num_eligible_peers // Otherwise, we attempt to dial all eligible peers
            };

        // If we have no peers to dial, return early
        if num_peers_to_dial == 0 {
            return vec![];
        }

        // Prioritize the eligible peers and select the peers to dial
        if selection::should_select_peers_by_latency(
            &self.network_context,
            self.enable_latency_aware_dialing,
        ) {
            // Ping the eligible peers (so that we can fetch missing ping latency information)
            self.ping_eligible_peers(eligible_peers.clone()).await;

            // Choose the peers to dial (weighted by ping latency)
            selection::choose_random_peers_by_ping_latency(
                self.network_context,
                eligible_peers,
                num_peers_to_dial,
                self.discovered_peers.clone(),
            )
        } else {
            // Choose the peers randomly
            selection::choose_peers_to_dial_randomly(eligible_peers, num_peers_to_dial)
        }
    }

    /// Pings the eligible peers to calculate their ping latencies
    /// and updates the discovered peer state accordingly.
    async fn ping_eligible_peers(&mut self, eligible_peers: Vec<(PeerId, DiscoveredPeer)>) {
        // Identify the eligible peers that don't already have latency information
        let peers_to_ping = eligible_peers
            .into_iter()
            .filter(|(_, peer)| peer.ping_latency_secs.is_none())
            .collect::<Vec<_>>();

        // If there are no peers to ping, return early
        let num_peers_to_ping = peers_to_ping.len();
        if num_peers_to_ping == 0 {
            return;
        }

        // Spawn a task that pings each peer concurrently
        let ping_start_time = Instant::now();
        let mut ping_tasks = vec![];
        for (peer_id, peer) in peers_to_ping.into_iter() {
            // Get the network address for the peer
            let network_context = self.network_context;
            let network_address = match self.dial_states.get(&peer_id) {
                Some(dial_state) => match dial_state.random_addr(&peer.addrs) {
                    Some(network_address) => network_address.clone(),
                    None => {
                        warn!(
                            NetworkSchema::new(&network_context),
                            "Peer {} does not have a network address!",
                            peer_id.short_str()
                        );
                        continue; // Continue onto the next peer
                    },
                },
                None => {
                    warn!(
                        NetworkSchema::new(&network_context),
                        "Peer {} does not have a dial state!",
                        peer_id.short_str()
                    );
                    continue; // Continue onto the next peer
                },
            };

            // Ping the peer
            let ping_task = spawn_latency_ping_task(
                network_context,
                peer_id,
                network_address,
                self.discovered_peers.clone(),
            );

            // Add the task to the list of ping tasks
            ping_tasks.push(ping_task);
        }

        // Wait for all the ping tasks to complete (or timeout)
        let num_ping_tasks = ping_tasks.len();
        join_all(ping_tasks).await;

        // Log the peer ping latencies
        log_peer_ping_latencies(
            self.network_context,
            self.discovered_peers.clone(),
            num_peers_to_ping,
            num_ping_tasks,
            ping_start_time,
        );
    }

    /// Queues a dial to the specified peer
    fn queue_dial_peer<'a>(
        &'a mut self,
        peer_id: PeerId,
        peer: DiscoveredPeer,
        pending_dials: &'a mut FuturesUnordered<BoxFuture<'static, PeerId>>,
        handle: &Handle,
    ) {
        // If we're attempting to dial a Peer we must not be connected to it. This ensures that
        // newly eligible, but not connected to peers, have their counter initialized properly.
        counters::peer_connected(&self.network_context, &peer_id, 0);

        // Get the peer's dial state
        let dial_state = match self.dial_states.get_mut(&peer_id) {
            Some(dial_state) => dial_state,
            None => {
                // The peer should have a dial state! If not, log an error and return.
                error!(
                    NetworkSchema::new(&self.network_context).remote_peer(&peer_id),
                    "{} Peer {} does not have a dial state!",
                    self.network_context,
                    peer_id.short_str()
                );
                return;
            },
        };

        // Choose the next addr to dial for this peer. Currently, we just
        // round-robin the selection, i.e., try the sequence:
        // addr[0], .., addr[len-1], addr[0], ..
        let addr = match dial_state.next_addr(&peer.addrs) {
            Some(addr) => addr.clone(),
            None => {
                warn!(
                    NetworkSchema::new(&self.network_context).remote_peer(&peer_id),
                    "{} Peer {} does not have any network addresses!",
                    self.network_context,
                    peer_id.short_str()
                );
                return;
            },
        };

        // Using the DialState's backoff strategy, compute the delay until
        // the next dial attempt for this peer.
        let dial_delay = dial_state.next_backoff_delay(self.max_delay);
        let f_delay = self.time_service.sleep(dial_delay);
        info!(
            NetworkSchema::new(&self.network_context),
            "queue_dial_peer going to dial {} @ {} after delay {:?}",
            peer_id.short_str_lossless(),
            addr,
            dial_delay,
        );

        let (cancel_tx, cancel_rx) = oneshot::channel();

        let network_context = self.network_context;
        let remote_peer_network_id = PeerNetworkId::new(self.network_context.network_id(), peer_id);
        // transport is just config, no state, so clone away and make the async move references better below
        let transport_clone = self.transport.clone();
        let config_clone = self.config.clone();
        let apps = self.apps.clone();
        let peers_and_metadata = self.peers_and_metadata.clone();
        let peer_senders = self.peer_senders.clone();
        let handle = handle.clone();
        // Create future which completes by either dialing after calculated
        // delay or on cancellation.
        let f = async move {
            // We dial after a delay. The dial can be canceled by sending to or dropping
            // `cancel_rx`.
            let config_clone = config_clone;
            let mut transport_clone = transport_clone;
            // let addr = addr;
            let dial_result = futures::select! {
                _ = f_delay.fuse() => {
                    info!(
                        NetworkSchema::new(&network_context)
                            .remote_peer(&peer_id)
                            .network_address(&addr),
                        "{} dialing peer {} at {}",
                        network_context,
                        peer_id.short_str_lossless(),
                        addr
                    );
                    let result = transport_clone.dial(
                        remote_peer_network_id,
                        addr.clone(),
                        &config_clone,
                        apps,
                        handle,
                        peers_and_metadata,
                        peer_senders,
                        network_context,
                    ).await;
                    match result {
                        Ok(_) => {
                            info!(
                                NetworkSchema::new(&network_context)
                                    .remote_peer(&peer_id)
                                    .network_address(&addr),
                                "{} dialing peer {} ok",
                                network_context,
                                peer_id.short_str_lossless()
                            );
                            DialResult::Success
                        },
                        Err(err) => {
                            warn!(
                                NetworkSchema::new(&network_context)
                                    .remote_peer(&peer_id)
                                    .network_address(&addr),
                                "{} dialing peer {} err {}",
                                network_context,
                                peer_id.short_str_lossless(),
                                err,
                            );
                            DialResult::Failed
                        }
                    }
                },
                _ = cancel_rx.fuse() => {
                    info!(
                        NetworkSchema::new(&network_context)
                            .remote_peer(&peer_id)
                            .network_address(&addr),
                        "{} dialing CANCELLED {} at {}",
                        network_context,
                        peer_id.short_str_lossless(),
                        addr
                    );
                    DialResult::Cancelled
                },
            };
            log_dial_result(network_context, peer_id, addr, dial_result);
            // Send peer_id as future result so it can be removed from dial queue.
            peer_id
        };
        pending_dials.push(f.boxed());

        // Update last dial time
        self.discovered_peers
            .write()
            .update_last_dial_time(&peer_id);
        self.dial_queue.insert(peer_id, cancel_tx);
    }

    // Note: We do not check that the connections to older incarnations of a node are broken, and
    // instead rely on the node moving to a new epoch to break connections made from older
    // incarnations.
    async fn check_connectivity<'a>(
        &'a mut self,
        pending_dials: &'a mut FuturesUnordered<BoxFuture<'static, PeerId>>,
        handle: &Handle,
    ) {
        trace!(
            NetworkSchema::new(&self.network_context),
            "{} Checking connectivity",
            self.network_context
        );

        // Log the eligible peers with addresses from discovery
        // sample!(SampleRate::Duration(Duration::from_secs(60)), {
        info!(
            NetworkSchema::new(&self.network_context),
            discovered_peers = ?self.discovered_peers,
            "Active discovered peers"
        );
        // });

        // Cancel dials to peers that are no longer eligible.
        self.cancel_stale_dials().await;
        // Disconnect from connected peers that are no longer eligible.
        self.close_stale_connections().await;
        // Dial peers which are eligible but are neither connected nor queued for dialing in the
        // future.
        self.dial_eligible_peers(pending_dials, handle).await;

        // Update the metrics for any peer ping latencies
        self.update_ping_latency_metrics();
    }

    /// Updates the metrics for tracking pre-dial and connected peer ping latencies
    fn update_ping_latency_metrics(&self) {
        // Update the pre-dial peer ping latencies
        for (_, peer) in self.discovered_peers.read().peer_set.iter() {
            if let Some(ping_latency_secs) = peer.ping_latency_secs {
                counters::observe_pre_dial_ping_time(&self.network_context, ping_latency_secs);
            }
        }

        // Update the connected peer ping latencies
        let pam_all = self.peers_and_metadata.get_all_peers_and_metadata();
        for (_network_id, netpeers) in pam_all.iter() {
            for (peer_id, peer_metadata) in netpeers.iter() {
                if !peer_metadata.is_connected() {
                    continue;
                }

                if let Some(ping_latency_secs) =
                    self.discovered_peers.read().get_ping_latency_secs(peer_id)
                {
                    counters::observe_connected_ping_time(&self.network_context, ping_latency_secs);
                }
            }
        }
    }

    fn handle_request(&mut self, req: ConnectivityRequest) {
        trace!(
            NetworkSchema::new(&self.network_context),
            connectivity_request = req,
            "{} Handling ConnectivityRequest",
            self.network_context
        );

        match req {
            ConnectivityRequest::UpdateDiscoveredPeers(src, discovered_peers) => {
                trace!(
                    NetworkSchema::new(&self.network_context),
                    "{} Received updated list of discovered peers: src: {:?}",
                    self.network_context,
                    src,
                );
                self.handle_update_discovered_peers(src, discovered_peers);
            },
            // GetDialQueueSize only used by test code
            ConnectivityRequest::GetDialQueueSize(sender) => {
                sender.send(self.dial_queue.len()).unwrap();
            },
            // GetConnectedSize only used by test code
            ConnectivityRequest::GetConnectedSize(sender) => {
                let count = self.peers_and_metadata.count_connected_peers(None);
                sender.send(count).unwrap();
            },
        }
    }

    /// Handles an update for newly discovered peers. This typically
    /// occurs at node startup, and on epoch changes.
    fn handle_update_discovered_peers(
        &mut self,
        src: DiscoverySource,
        new_discovered_peers: PeerSet,
    ) {
        // Log the update event
        info!(
            NetworkSchema::new(&self.network_context),
            "{} Received updated list of discovered peers! Source: {:?}, num peers: {:?}",
            self.network_context,
            src,
            new_discovered_peers.len()
        );

        // Remove peers that no longer have relevant network information
        let mut keys_updated = false;
        let mut peers_to_check_remove = Vec::new();
        for (peer_id, peer) in self.discovered_peers.write().peer_set.iter_mut() {
            let new_peer = new_discovered_peers.get(peer_id);
            let check_remove = if let Some(new_peer) = new_peer {
                if new_peer.keys.is_empty() {
                    keys_updated |= peer.keys.clear_src(src);
                }
                if new_peer.addresses.is_empty() {
                    peer.addrs.clear_src(src);
                }
                new_peer.addresses.is_empty() && new_peer.keys.is_empty()
            } else {
                keys_updated |= peer.keys.clear_src(src);
                peer.addrs.clear_src(src);
                true
            };
            if check_remove {
                peers_to_check_remove.push(*peer_id);
            }
        }

        // Remove peers that no longer have state
        for peer_id in peers_to_check_remove {
            self.discovered_peers.write().remove_peer_if_empty(&peer_id);
        }

        // Make updates to the peers accordingly
        for (peer_id, discovered_peer) in new_discovered_peers {
            // Don't include ourselves, because we don't need to dial ourselves
            if peer_id == self.network_context.peer_id() {
                continue;
            }

            // Create the new `DiscoveredPeer`, role is set when a `Peer` is first discovered
            let mut discovered_peers = self.discovered_peers.write();
            let peer = discovered_peers
                .peer_set
                .entry(peer_id)
                .or_insert_with(|| DiscoveredPeer::new(discovered_peer.role));

            // Update the peer's pubkeys
            let mut peer_updated = false;
            if peer.keys.update(src, discovered_peer.keys) {
                info!(
                    NetworkSchema::new(&self.network_context)
                        .remote_peer(&peer_id)
                        .discovery_source(&src),
                    "{} pubkey sets updated for peer: {}, pubkeys: {}",
                    self.network_context,
                    peer_id.short_str(),
                    peer.keys
                );
                keys_updated = true;
                peer_updated = true;
            }

            // Update the peer's addresses
            if peer.addrs.update(src, discovered_peer.addresses) {
                info!(
                    NetworkSchema::new(&self.network_context).remote_peer(&peer_id),
                    network_addresses = &peer.addrs,
                    "{} addresses updated for peer: {}, update src: {:?}, addrs: {}",
                    self.network_context,
                    peer_id.short_str(),
                    src,
                    &peer.addrs,
                );
                peer_updated = true;
            }

            // If we're currently trying to dial this peer, we reset their
            // dial state. As a result, we will begin our next dial attempt
            // from the first address (which might have changed) and from a
            // fresh backoff (since the current backoff delay might be maxed
            // out if we can't reach any of their previous addresses).
            if peer_updated {
                if let Some(dial_state) = self.dial_states.get_mut(&peer_id) {
                    *dial_state = DialState::new(self.backoff_strategy.clone());
                }
            }
        }

        // update eligible peers accordingly
        if keys_updated {
            // For each peer, union all of the pubkeys from each discovery source
            // to generate the new eligible peers set.
            let new_eligible = self.discovered_peers.read().get_eligible_peers();

            // Swap in the new eligible peers set. Drop the old set after releasing
            // the write lock.
            if let Some(trusted_peers) = self.get_trusted_peers() {
                let _old_eligible = {
                    let mut trusted_peers = trusted_peers.write();
                    mem::replace(&mut *trusted_peers, new_eligible)
                };
            }
        }
    }

    fn handle_control_notification(&mut self, notif: ConnectionNotification) {
        trace!(
            NetworkSchema::new(&self.network_context),
            connection_notification = notif,
            "Connection notification"
        );
        match notif {
            ConnectionNotification::NewPeer(metadata, _network_id) => {
                let peer_id = metadata.remote_peer_id;
                counters::peer_connected(&self.network_context, &peer_id, 1);

                // Cancel possible queued dial to this peer.
                self.dial_states.remove(&peer_id);
                self.dial_queue.remove(&peer_id).map(|x| x.send(()));
            },
            ConnectionNotification::LostPeer(metadata, _network_id) => {
                let peer_id = metadata.remote_peer_id;
                counters::peer_connected(&self.network_context, &peer_id, 0);

                info!(
                    NetworkSchema::new(&self.network_context)
                        .remote_peer(&peer_id)
                        .connection_metadata(&metadata),
                    "{} Removing peer '{}' event metadata: {}",
                    self.network_context,
                    peer_id.short_str(),
                    metadata
                );
                // Cancel possible queued dial to this peer.
                self.dial_states.remove(&peer_id);
                self.dial_queue.remove(&peer_id).map(|x| x.send(()));
            },
        }
    }
}

fn log_dial_result(
    network_context: NetworkContext,
    peer_id: PeerId,
    addr: NetworkAddress,
    dial_result: DialResult,
) {
    match dial_result {
        DialResult::Success => {
            info!(
                NetworkSchema::new(&network_context)
                    .remote_peer(&peer_id)
                    .network_address(&addr),
                "{} Successfully connected to peer: {} at address: {}",
                network_context,
                peer_id.short_str(),
                addr
            );
        },
        DialResult::Cancelled => {
            info!(
                NetworkSchema::new(&network_context).remote_peer(&peer_id),
                "{} Cancelled pending dial to peer: {}",
                network_context,
                peer_id.short_str()
            );
        },
        DialResult::_AlreadyConnected => {
            unreachable!("nobody uses DialResult AlreadyConnected");
            // info!(
            // NetworkSchema::new(&network_context)
            // .remote_peer(&peer_id)
            // .network_address(&addr),
            // "{} Already connected to peer: {}",
            // network_context,
            // peer_id.short_str(),
            // // a
            // );
        },
        DialResult::Failed => {
            info!(
                NetworkSchema::new(&network_context)
                    .remote_peer(&peer_id)
                    .network_address(&addr),
                // error = %e,
                "{} Failed to connect to peer: {} at address: {}",
                network_context,
                peer_id.short_str(),
                addr,
                //e
            );
        },
    }
}

/// Logs the total and individual ping latencies
fn log_peer_ping_latencies(
    network_context: NetworkContext,
    discovered_peers: Arc<RwLock<DiscoveredPeerSet>>,
    total_peers_to_ping: usize,
    num_peers_pinged: usize,
    ping_start_time: Instant,
) {
    // Log the total ping latency time
    let ping_latency_duration = Instant::now().duration_since(ping_start_time);
    info!(
        NetworkSchema::new(&network_context),
        "Finished pinging eligible peers! Total peers to ping: {}, num peers pinged: {}, time: {} secs",
        total_peers_to_ping,
        num_peers_pinged,
        ping_latency_duration.as_secs_f64()
    );

    // Log the ping latencies for the eligible peers (sorted by latency)
    let eligible_peers = discovered_peers.read().peer_set.clone();
    let eligible_peers_and_latencies = eligible_peers
        .into_iter()
        .map(|(peer_id, peer)| (peer_id, peer.ping_latency_secs))
        .collect::<Vec<_>>();
    let sorted_eligible_peers_and_latencies = eligible_peers_and_latencies
        .iter()
        .sorted_by_key(|(_, ping_latency_secs)| ping_latency_secs.map(OrderedFloat))
        .collect::<Vec<_>>();
    info!(
        NetworkSchema::new(&network_context),
        "Sorted eligible peers with recorded ping latencies: {:?}",
        sorted_eligible_peers_and_latencies
    );
}

/// Spawns a task that pings the peer at the specified
/// network address and updates the peer's ping latency.
fn spawn_latency_ping_task(
    network_context: NetworkContext,
    peer_id: AccountAddress,
    network_address: NetworkAddress,
    discovered_peers: Arc<RwLock<DiscoveredPeerSet>>,
) -> JoinHandle<()> {
    tokio::task::spawn_blocking(move || {
        // Extract the socket addresses from the network address
        let socket_addresses = match network_address.to_socket_addrs() {
            Ok(socket_addresses) => socket_addresses.collect::<Vec<_>>(),
            Err(error) => {
                warn!(
                    NetworkSchema::new(&network_context),
                    "Failed to resolve network address {:?}: {}", network_address, error
                );
                return;
            },
        };

        // If no socket addresses were found, log an error and return
        if socket_addresses.is_empty() {
            warn!(
                NetworkSchema::new(&network_context),
                "Peer {} does not have any socket addresses for network address {:?}!",
                peer_id.short_str(),
                network_address,
            );
            return;
        }

        // Limit the number of socket addresses we'll try to connect to
        let socket_addresses = socket_addresses
            .iter()
            .take(MAX_SOCKET_ADDRESSES_TO_PING)
            .collect::<Vec<_>>();

        // Attempt to connect to the socket addresses over TCP and time the connection
        for socket_address in socket_addresses {
            // Start the ping timer
            let start_time = Instant::now();

            // Attempt to connect to the socket address
            if let Ok(tcp_stream) = TcpStream::connect_timeout(
                socket_address,
                Duration::from_secs(MAX_CONNECTION_TIMEOUT_SECS),
            ) {
                // We connected successfully, update the peer's ping latency
                let ping_latency_secs = start_time.elapsed().as_secs_f64();
                discovered_peers
                    .write()
                    .update_ping_latency_secs(&peer_id, ping_latency_secs);

                // Attempt to terminate the TCP stream cleanly
                if let Err(error) = tcp_stream.shutdown(Shutdown::Both) {
                    warn!(
                        NetworkSchema::new(&network_context),
                        "Failed to terminate TCP stream to peer {} after pinging: {}",
                        peer_id.short_str(),
                        error
                    );
                }

                return;
            } else {
                // Log an error if we failed to connect to the socket address
                info!(
                    NetworkSchema::new(&network_context),
                    "Failed to ping peer {} at socket address {:?} after pinging",
                    peer_id.short_str(),
                    socket_address
                );
            }
        }
    })
}

/////////////////////
// DiscoverySource //
/////////////////////

impl DiscoverySource {
    fn as_usize(self) -> usize {
        self as u8 as usize
    }
}

///////////////
// Addresses //
///////////////

impl Addresses {
    fn len(&self) -> usize {
        self.0.iter().map(Vec::len).sum()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Update the addresses for the `DiscoverySource` bucket. Return `true` if
    /// the addresses have actually changed.
    fn update(&mut self, src: DiscoverySource, addrs: Vec<NetworkAddress>) -> bool {
        let src_idx = src.as_usize();
        if self.0[src_idx] != addrs {
            self.0[src_idx] = addrs;
            true
        } else {
            false
        }
    }

    fn clear_src(&mut self, src: DiscoverySource) -> bool {
        self.update(src, Vec::new())
    }

    fn get(&self, idx: usize) -> Option<&NetworkAddress> {
        self.0.iter().flatten().nth(idx)
    }

    /// The Union isn't stable, and order is completely disregarded
    fn union(&self) -> Vec<NetworkAddress> {
        let set: HashSet<_> = self.0.iter().flatten().cloned().collect();
        set.into_iter().collect()
    }
}

impl fmt::Display for Addresses {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Write without the typical "Addresses(..)" around the output to reduce
        // debug noise.
        write!(f, "{:?}", self.0)
    }
}

impl fmt::Debug for Addresses {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

////////////////
// PublicKeys //
////////////////

impl PublicKeys {
    fn len(&self) -> usize {
        self.0.iter().map(HashSet::len).sum()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn update(&mut self, src: DiscoverySource, pubkeys: HashSet<x25519::PublicKey>) -> bool {
        let src_idx = src.as_usize();
        if self.0[src_idx] != pubkeys {
            self.0[src_idx] = pubkeys;
            true
        } else {
            false
        }
    }

    fn clear_src(&mut self, src: DiscoverySource) -> bool {
        self.update(src, HashSet::new())
    }

    fn union(&self) -> HashSet<x25519::PublicKey> {
        self.0.iter().flatten().copied().collect()
    }
}

impl fmt::Display for PublicKeys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Write without the typical "PublicKeys(..)" around the output to reduce
        // debug noise.
        write!(f, "{:?}", self.0)
    }
}

impl fmt::Debug for PublicKeys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

///////////////
// DialState //
///////////////

impl<TBackoff> DialState<TBackoff>
where
    TBackoff: Iterator<Item = Duration> + Clone,
{
    fn new(backoff: TBackoff) -> Self {
        Self {
            backoff,
            addr_idx: 0,
        }
    }

    /// Returns the address to dial (specified by the index) for this peer
    fn get_addr_at_index<'a>(
        &self,
        addr_index: usize,
        addrs: &'a Addresses,
    ) -> Option<&'a NetworkAddress> {
        addrs.get(addr_index % addrs.len())
    }

    /// Returns the current address to dial for this peer and updates
    /// the internal state to point to the next address.
    fn next_addr<'a>(&mut self, addrs: &'a Addresses) -> Option<&'a NetworkAddress> {
        let curr_addr = self.get_addr_at_index(self.addr_idx, addrs);
        self.addr_idx = self.addr_idx.wrapping_add(1);
        curr_addr
    }

    /// Returns a random address to dial for this peer
    fn random_addr<'a>(&self, addrs: &'a Addresses) -> Option<&'a NetworkAddress> {
        let addr_index = ::rand_latest::thread_rng().gen_range(0..addrs.len());
        self.get_addr_at_index(addr_index, addrs)
    }

    fn next_backoff_delay(&mut self, max_delay: Duration) -> Duration {
        let jitter = jitter(MAX_CONNECTION_DELAY_JITTER);

        min(max_delay, self.backoff.next().unwrap_or(max_delay)) + jitter
    }
}
