// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The PeerManager module is responsible for establishing connections between Peers and for
//! opening/receiving new substreams on those connections.
//!
//! ## Implementation
//!
//! The PeerManager is implemented as a number of actors:
//!  * A main event loop actor which is responsible for handling requests and sending
//!  notification about new/lost Peers to the rest of the network stack.
//!  * An actor responsible for dialing and listening for new connections.
use crate::{
    constants,
    counters::{self},
    logging::*,
    peer::{Peer, PeerRequest},
    transport::{
        Connection, ConnectionId, ConnectionMetadata, TSocket as TransportTSocket,
        TRANSPORT_TIMEOUT,
    },
    ProtocolId,
};
use aptos_channels::{self, aptos_channel, message_queues::QueueStyle};
use aptos_config::{
    config::AccessControlPolicy,
    network_id::{NetworkContext, PeerNetworkId},
};
use aptos_logger::prelude::*;
use aptos_netcore::transport::{ConnectionOrigin, Transport};
use aptos_short_hex_str::AsShortHexStr;
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_types::{network_address::NetworkAddress, PeerId};
use futures::{
    channel::oneshot,
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
    sink::SinkExt,
    stream::StreamExt,
};
use std::{collections::HashMap, marker::PhantomData, sync::Arc, time::Duration};
use tokio::runtime::Handle;

pub mod builder;
pub mod conn_notifs_channel;
mod error;
mod senders;
#[cfg(test)]
mod tests;
mod transport;
mod types;

pub use self::error::PeerManagerError;
use crate::{
    application::{error::Error, storage::PeersAndMetadata},
    peer_manager::transport::{TransportHandler, TransportRequest},
    protocols::network::{ReceivedMessage, SerializedRequest},
};
use aptos_config::config::PeerRole;
use aptos_types::account_address::AccountAddress;
pub use senders::*;
pub use types::*;

/// Responsible for handling and maintaining connections to other Peers
pub struct PeerManager<TTransport, TSocket>
where
    TTransport: Transport,
    TSocket: AsyncRead + AsyncWrite,
{
    network_context: NetworkContext,
    /// A handle to a tokio executor.
    executor: Handle,
    /// A handle to a time service for easily mocking time-related operations.
    time_service: TimeService,
    /// Address to listen on for incoming connections.
    listen_addr: NetworkAddress,
    /// Connection Listener, listening on `listen_addr`
    transport_handler: Option<TransportHandler<TTransport, TSocket>>,
    /// Primary storage: ConnectionId -> PeerConnection
    connections: HashMap<ConnectionId, PeerConnection>,
    /// Index: PeerId -> Vec<ConnectionId> (for O(1) lookup by peer)
    peer_connections: HashMap<PeerId, Vec<ConnectionId>>,
    /// Round-robin index per peer for connection selection
    peer_rr_index: HashMap<PeerId, usize>,
    /// Maximum connections allowed per peer
    max_connections_per_peer: usize,
    /// Shared metadata storage about trusted peers and metadata
    peers_and_metadata: Arc<PeersAndMetadata>,
    /// Channel to receive requests from other actors.
    requests_rx: aptos_channel::Receiver<(PeerId, ProtocolId), PeerManagerRequest>,
    /// Upstream handlers for RPC and DirectSend protocols. The handlers are promised fair delivery
    /// of messages across (PeerId, ProtocolId).
    upstream_handlers:
        Arc<HashMap<ProtocolId, aptos_channel::Sender<(PeerId, ProtocolId), ReceivedMessage>>>,
    /// Channels to send NewPeer/LostPeer notifications to.
    connection_event_handlers: Vec<conn_notifs_channel::Sender>,
    /// Channel used to send Dial requests to the ConnectionHandler actor
    transport_reqs_tx: aptos_channels::Sender<TransportRequest>,
    /// Sender for connection events.
    transport_notifs_tx: aptos_channels::Sender<TransportNotification<TSocket>>,
    /// Receiver for connection requests.
    connection_reqs_rx: aptos_channel::Receiver<PeerId, ConnectionRequest>,
    /// Receiver for connection events.
    transport_notifs_rx: aptos_channels::Receiver<TransportNotification<TSocket>>,
    /// A map of outstanding disconnect requests.
    outstanding_disconnect_requests:
        HashMap<ConnectionId, oneshot::Sender<Result<(), PeerManagerError>>>,
    /// Pin the transport type corresponding to this PeerManager instance
    phantom_transport: PhantomData<TTransport>,
    /// Size of channels between different actors.
    channel_size: usize,
    /// Max network frame size
    max_frame_size: usize,
    /// Max network message size
    max_message_size: usize,
    /// Inbound connection limit separate of outbound connections
    inbound_connection_limit: usize,
    /// Access control policy for peer connections
    access_control_policy: Option<Arc<AccessControlPolicy>>,
}

impl<TTransport, TSocket> PeerManager<TTransport, TSocket>
where
    TTransport: Transport<Output = Connection<TSocket>> + Send + 'static,
    TSocket: TransportTSocket,
{
    /// Construct a new PeerManager actor
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        executor: Handle,
        time_service: TimeService,
        transport: TTransport,
        network_context: NetworkContext,
        listen_addr: NetworkAddress,
        peers_and_metadata: Arc<PeersAndMetadata>,
        requests_rx: aptos_channel::Receiver<(PeerId, ProtocolId), PeerManagerRequest>,
        connection_reqs_rx: aptos_channel::Receiver<PeerId, ConnectionRequest>,
        upstream_handlers: HashMap<
            ProtocolId,
            aptos_channel::Sender<(PeerId, ProtocolId), ReceivedMessage>,
        >,
        connection_event_handlers: Vec<conn_notifs_channel::Sender>,
        channel_size: usize,
        max_frame_size: usize,
        max_message_size: usize,
        inbound_connection_limit: usize,
        max_connections_per_peer: usize,
        access_control_policy: Option<Arc<AccessControlPolicy>>,
    ) -> Self {
        let (transport_notifs_tx, transport_notifs_rx) = aptos_channels::new(
            channel_size,
            &counters::PENDING_CONNECTION_HANDLER_NOTIFICATIONS,
        );
        let (transport_reqs_tx, transport_reqs_rx) =
            aptos_channels::new(channel_size, &counters::PENDING_PEER_MANAGER_DIAL_REQUESTS);
        //TODO now that you can only listen on a socket inside of a tokio runtime we'll need to
        // rethink how we init the PeerManager so we don't have to do this funny thing.
        let transport_notifs_tx_clone = transport_notifs_tx.clone();
        let _guard = executor.enter();
        let (transport_handler, listen_addr) = TransportHandler::new(
            network_context,
            time_service.clone(),
            transport,
            listen_addr,
            transport_reqs_rx,
            transport_notifs_tx_clone,
        );

        Self {
            network_context,
            executor,
            time_service,
            listen_addr,
            transport_handler: Some(transport_handler),
            connections: HashMap::new(),
            peer_connections: HashMap::new(),
            peer_rr_index: HashMap::new(),
            max_connections_per_peer,
            peers_and_metadata,
            requests_rx,
            connection_reqs_rx,
            transport_reqs_tx,
            transport_notifs_tx,
            transport_notifs_rx,
            outstanding_disconnect_requests: HashMap::new(),
            phantom_transport: PhantomData,
            upstream_handlers: Arc::new(upstream_handlers),
            connection_event_handlers,
            channel_size,
            max_frame_size,
            max_message_size,
            inbound_connection_limit,
            access_control_policy,
        }
    }

    pub fn update_connected_peers_metrics(&self) {
        let total = self.connections.len();
        let inbound = self
            .connections
            .values()
            .filter(|conn| conn.metadata.origin == ConnectionOrigin::Inbound)
            .count();
        let outbound = total.saturating_sub(inbound);

        counters::connections(&self.network_context, ConnectionOrigin::Inbound).set(inbound as i64);
        counters::connections(&self.network_context, ConnectionOrigin::Outbound)
            .set(outbound as i64);
    }

    fn sample_connected_peers(&self) {
        // Sample final state at most once a minute, ensuring consistent ordering
        sample!(SampleRate::Duration(Duration::from_secs(60)), {
            let peers: Vec<_> = self
                .connections
                .values()
                .map(|conn| {
                    (
                        conn.metadata.remote_peer_id,
                        conn.metadata.addr.clone(),
                        conn.metadata.origin,
                    )
                })
                .collect();
            info!(
                NetworkSchema::new(&self.network_context),
                peers = ?peers,
                "Current connected peers"
            )
        });
    }

    /// Returns the number of active connections for a peer
    fn connection_count(&self, peer_id: &PeerId) -> usize {
        self.peer_connections
            .get(peer_id)
            .map(|conns| conns.len())
            .unwrap_or(0)
    }

    /// Returns whether we can accept another connection from this peer
    fn can_accept_connection(&self, peer_id: &PeerId) -> bool {
        self.connection_count(peer_id) < self.max_connections_per_peer
    }

    /// Selects a connection to use for sending to a peer using round-robin
    fn select_connection(&mut self, peer_id: &PeerId) -> Option<&mut PeerConnection> {
        let conn_ids = self.peer_connections.get(peer_id)?;
        if conn_ids.is_empty() {
            return None;
        }

        let idx = self.peer_rr_index.entry(*peer_id).or_insert(0);
        let conn_id = conn_ids[*idx % conn_ids.len()];
        *idx = idx.wrapping_add(1);

        self.connections.get_mut(&conn_id)
    }

    /// Adds a connection to the internal data structures
    fn add_connection(&mut self, conn: PeerConnection) {
        let peer_id = conn.peer_id;
        let conn_id = conn.connection_id;

        self.connections.insert(conn_id, conn);
        self.peer_connections
            .entry(peer_id)
            .or_default()
            .push(conn_id);
    }

    /// Removes a connection from the internal data structures
    fn remove_connection(&mut self, conn_id: ConnectionId) -> Option<PeerConnection> {
        if let Some(conn) = self.connections.remove(&conn_id) {
            let peer_id = conn.peer_id;
            if let Some(conn_ids) = self.peer_connections.get_mut(&peer_id) {
                conn_ids.retain(|id| *id != conn_id);
                if conn_ids.is_empty() {
                    self.peer_connections.remove(&peer_id);
                    self.peer_rr_index.remove(&peer_id);
                }
            }
            Some(conn)
        } else {
            None
        }
    }

    /// Returns true if there is at least one connection to this peer
    fn has_connection(&self, peer_id: &PeerId) -> bool {
        self.peer_connections
            .get(peer_id)
            .map(|conns| !conns.is_empty())
            .unwrap_or(false)
    }

    /// Returns connection metadata for the first connection to this peer (if any)
    fn get_first_connection_metadata(&self, peer_id: &PeerId) -> Option<&ConnectionMetadata> {
        self.peer_connections
            .get(peer_id)
            .and_then(|conn_ids| conn_ids.first())
            .and_then(|conn_id| self.connections.get(conn_id))
            .map(|conn| &conn.metadata)
    }

    /// Get the [`NetworkAddress`] we're listening for incoming connections on
    pub fn listen_addr(&self) -> &NetworkAddress {
        &self.listen_addr
    }

    /// Checks if a peer connection should be allowed based on the access control policy.
    /// Returns successfully if the peer is allowed, otherwise returns an error with the reason.
    fn check_peer_access_lists(&self, peer_id: &PeerId) -> Result<(), String> {
        // Check if the peer is allowed based on the access control policy
        if let Some(access_control_policy) = &self.access_control_policy {
            if !access_control_policy.is_peer_allowed(peer_id) {
                // Determine the error based on the policy type
                let error = match access_control_policy.as_ref() {
                    AccessControlPolicy::AllowList(_) => {
                        format!(
                            "Peer {} is not in the network allow list!",
                            peer_id.short_str()
                        )
                    },
                    AccessControlPolicy::BlockList(_) => {
                        format!("Peer {} is in the network block list!", peer_id.short_str())
                    },
                };
                return Err(error);
            }
        }

        // Otherwise, allow all peers by default
        Ok(())
    }

    /// Start listening on the set address and return a future which runs PeerManager
    pub async fn start(mut self) {
        // Start listening for connections.
        info!(
            NetworkSchema::new(&self.network_context),
            "Start listening for incoming connections on {}", self.listen_addr
        );
        self.start_connection_listener();
        loop {
            ::futures::select! {
                connection_event = self.transport_notifs_rx.select_next_some() => {
                    self.handle_connection_event(connection_event);
                }
                connection_request = self.connection_reqs_rx.select_next_some() => {
                    self.handle_outbound_connection_request(connection_request).await;
                }
                request = self.requests_rx.select_next_some() => {
                    self.handle_outbound_request(request).await;
                }
                complete => {
                    break;
                }
            }
        }

        warn!(
            NetworkSchema::new(&self.network_context),
            "PeerManager actor terminated"
        );
    }

    fn handle_connection_event(&mut self, event: TransportNotification<TSocket>) {
        trace!(
            NetworkSchema::new(&self.network_context),
            transport_notification = format!("{:?}", event),
            "{} TransportNotification::{:?}",
            self.network_context,
            event
        );
        self.sample_connected_peers();
        match event {
            TransportNotification::NewConnection(conn) => {
                self.handle_new_connection_event(conn);
            },
            TransportNotification::Disconnected(lost_conn_metadata, reason) => {
                // See: https://github.com/aptos-labs/aptos-core/issues/3128#issuecomment-605351504 for
                // detailed reasoning on `Disconnected` events should be handled correctly.
                info!(
                    NetworkSchema::new(&self.network_context)
                        .connection_metadata_with_address(&lost_conn_metadata),
                    disconnection_reason = reason,
                    "{} Connection {} closed due to {}",
                    self.network_context,
                    lost_conn_metadata,
                    reason
                );
                let peer_id = lost_conn_metadata.remote_peer_id;
                let connection_id = lost_conn_metadata.connection_id;

                // Remove the specific connection from our data structures
                if self.remove_connection(connection_id).is_some() {
                    self.remove_peer_from_metadata(peer_id, connection_id);
                }
                self.update_connected_peers_metrics();

                // If the connection was explicitly closed by an upstream client, send an ACK.
                if let Some(oneshot_tx) = self
                    .outstanding_disconnect_requests
                    .remove(&lost_conn_metadata.connection_id)
                {
                    // The client explicitly closed the connection and it should be notified.
                    if let Err(send_err) = oneshot_tx.send(Ok(())) {
                        info!(
                            NetworkSchema::new(&self.network_context),
                            error = ?send_err,
                            "{} Failed to notify upstream client of closed connection for peer {}: {:?}",
                            self.network_context,
                            peer_id,
                            send_err
                        );
                    }
                }

                // Notify upstream only if there are no more connections to this peer
                if !self.has_connection(&peer_id) {
                    let notif = ConnectionNotification::LostPeer(
                        lost_conn_metadata,
                        self.network_context.network_id(),
                    );
                    self.send_conn_notification(peer_id, notif);
                }
            },
        }
    }

    /// Handles a new connection event
    fn handle_new_connection_event(&mut self, conn: Connection<TSocket>) {
        // Get the trusted peers
        let trusted_peers = match self
            .peers_and_metadata
            .get_trusted_peers(&self.network_context.network_id())
        {
            Ok(trusted_peers) => trusted_peers,
            Err(error) => {
                error!(
                    NetworkSchema::new(&self.network_context)
                        .connection_metadata_with_address(&conn.metadata),
                    "Failed to get trusted peers for network context: {:?}, error: {:?}",
                    self.network_context,
                    error
                );
                return;
            },
        };

        // Check the allow/block lists
        if let Err(error) = self.check_peer_access_lists(&conn.metadata.remote_peer_id) {
            warn!(
                NetworkSchema::new(&self.network_context)
                    .connection_metadata_with_address(&conn.metadata),
                "{} Connection rejected by allow/block list: {}", self.network_context, error
            );
            counters::connections_rejected(&self.network_context, conn.metadata.origin).inc();
            self.disconnect(conn);
            return;
        }

        // Verify that we have not reached the max connection limit for unknown inbound peers
        if conn.metadata.origin == ConnectionOrigin::Inbound {
            // Everything below here is meant for unknown peers only. The role comes from
            // the Noise handshake and if it's not `Unknown` then it is trusted.
            if conn.metadata.role == PeerRole::Unknown {
                // TODO: Keep track of somewhere else to not take this hit in case of DDoS
                // Count unknown inbound connections
                let unknown_inbound_conns = self
                    .connections
                    .values()
                    .filter(|conn| {
                        conn.metadata.origin == ConnectionOrigin::Inbound
                            && trusted_peers
                                .get(&conn.peer_id)
                                .is_none_or(|peer| peer.role == PeerRole::Unknown)
                    })
                    .count();

                // Reject excessive inbound connections made by unknown peers
                // We control outbound connections with Connectivity manager before we even send them
                // and we must allow connections that already exist to pass through tie breaking.
                if !self.has_connection(&conn.metadata.remote_peer_id)
                    && unknown_inbound_conns + 1 > self.inbound_connection_limit
                {
                    info!(
                        NetworkSchema::new(&self.network_context)
                            .connection_metadata_with_address(&conn.metadata),
                        "{} Connection rejected due to connection limit: {}",
                        self.network_context,
                        conn.metadata
                    );
                    counters::connections_rejected(&self.network_context, conn.metadata.origin)
                        .inc();
                    self.disconnect(conn);
                    return;
                }
            }
        }

        // Add the new peer and update the metric counters
        info!(
            NetworkSchema::new(&self.network_context)
                .connection_metadata_with_address(&conn.metadata),
            "{} New connection established: {}", self.network_context, conn.metadata
        );
        if let Err(error) = self.add_peer(conn) {
            warn!(
                NetworkSchema::new(&self.network_context),
                "Failed to add peer. Error: {:?}", error
            )
        }
        self.update_connected_peers_metrics();
    }

    fn remove_peer_from_metadata(&mut self, peer_id: AccountAddress, connection_id: ConnectionId) {
        let peer_network_id = PeerNetworkId::new(self.network_context.network_id(), peer_id);
        if let Err(error) = self
            .peers_and_metadata
            .remove_peer_metadata(peer_network_id, connection_id)
        {
            warn!(
                NetworkSchema::new(&self.network_context),
                "Failed to remove peer from peers and metadata. Peer: {:?}, error: {:?}",
                peer_network_id,
                error
            );
        }
    }

    async fn handle_outbound_connection_request(&mut self, request: ConnectionRequest) {
        trace!(
            NetworkSchema::new(&self.network_context),
            peer_manager_request = request,
            "{} PeerManagerRequest::{:?}",
            self.network_context,
            request
        );
        self.sample_connected_peers();
        match request {
            ConnectionRequest::DialPeer(requested_peer_id, addr, response_tx) => {
                // Only dial peers which we aren't already connected with (at max capacity)
                if let Some(curr_connection) =
                    self.get_first_connection_metadata(&requested_peer_id)
                {
                    // If we're at max connections per peer, reject the dial
                    if !self.can_accept_connection(&requested_peer_id) {
                        let error =
                            PeerManagerError::AlreadyConnected(curr_connection.addr.clone());
                        debug!(
                            NetworkSchema::new(&self.network_context)
                                .connection_metadata_with_address(curr_connection),
                            "{} Already at max connections to Peer {} with connection {:?}. Not dialing address {}",
                            self.network_context,
                            requested_peer_id.short_str(),
                            curr_connection,
                            addr
                        );
                        if let Err(send_err) = response_tx.send(Err(error)) {
                            info!(
                                NetworkSchema::new(&self.network_context)
                                    .remote_peer(&requested_peer_id),
                                "{} Failed to notify that peer is already connected for Peer {}: {:?}",
                                self.network_context,
                                requested_peer_id.short_str(),
                                send_err
                            );
                        }
                        return;
                    }
                }

                // Update the connection dial metrics
                counters::update_network_connection_operation_metrics(
                    &self.network_context,
                    counters::DIAL_LABEL.into(),
                    counters::DIAL_PEER_LABEL.into(),
                );

                // Send a transport request to dial the peer
                let request = TransportRequest::DialPeer(requested_peer_id, addr, response_tx);
                self.transport_reqs_tx.send(request).await.unwrap();
            },
            ConnectionRequest::DisconnectPeer(peer_id, disconnect_reason, resp_tx) => {
                // Update the connection disconnect metrics
                counters::update_network_connection_operation_metrics(
                    &self.network_context,
                    counters::DISCONNECT_LABEL.into(),
                    disconnect_reason.get_label(),
                );

                // Disconnect all connections to this peer
                if let Some(conn_ids) = self.peer_connections.get(&peer_id).cloned() {
                    if conn_ids.is_empty() {
                        info!(
                            NetworkSchema::new(&self.network_context).remote_peer(&peer_id),
                            "{} Connection with peer: {} was already closed",
                            self.network_context,
                            peer_id.short_str(),
                        );
                        if let Err(err) = resp_tx.send(Err(PeerManagerError::NotConnected(peer_id)))
                        {
                            info!(
                                NetworkSchema::new(&self.network_context),
                                error = ?err,
                                "{} Failed to notify that connection was already closed for Peer {}: {:?}",
                                self.network_context,
                                peer_id,
                                err
                            );
                        }
                        return;
                    }

                    // Get the first connection ID to track for the response
                    let first_conn_id = conn_ids[0];

                    // Close all connections to this peer
                    for conn_id in conn_ids.into_iter() {
                        if let Some(conn) = self.remove_connection(conn_id) {
                            self.remove_peer_from_metadata(conn.peer_id, conn_id);
                            // This triggers a disconnect.
                            drop(conn.sender);
                        }
                    }

                    // Track the first connection for the response
                    self.outstanding_disconnect_requests
                        .insert(first_conn_id, resp_tx);
                } else {
                    info!(
                        NetworkSchema::new(&self.network_context).remote_peer(&peer_id),
                        "{} Connection with peer: {} was already closed",
                        self.network_context,
                        peer_id.short_str(),
                    );
                    if let Err(err) = resp_tx.send(Err(PeerManagerError::NotConnected(peer_id))) {
                        info!(
                            NetworkSchema::new(&self.network_context),
                            error = ?err,
                            "{} Failed to notify that connection was already closed for Peer {}: {:?}",
                            self.network_context,
                            peer_id,
                            err
                        );
                    }
                }
            },
        }
    }

    /// Sends an outbound request for `RPC` or `DirectSend` to the peer
    async fn handle_outbound_request(&mut self, request: PeerManagerRequest) {
        trace!(
            NetworkSchema::new(&self.network_context),
            peer_manager_request = request,
            "{} PeerManagerRequest::{:?}",
            self.network_context,
            request
        );
        self.sample_connected_peers();
        let (peer_id, protocol_id, peer_request) = match request {
            PeerManagerRequest::SendDirectSend(peer_id, msg) => {
                (peer_id, msg.protocol_id(), PeerRequest::SendDirectSend(msg))
            },
            PeerManagerRequest::SendRpc(peer_id, req) => {
                (peer_id, req.protocol_id(), PeerRequest::SendRpc(req))
            },
        };

        // Use round-robin selection to pick a connection for this message
        // Note: We need to avoid borrow checker issues by extracting the connection info
        // and logging separately from the mutable borrow.
        let send_result = if let Some(conn) = self.select_connection(&peer_id) {
            let metadata = conn.metadata.clone();
            let result = conn.sender.push(protocol_id, peer_request);
            Some((metadata, result))
        } else {
            None
        };

        match send_result {
            Some((metadata, Err(err))) => {
                info!(
                    NetworkSchema::new(&self.network_context).connection_metadata(&metadata),
                    protocol_id = %protocol_id,
                    error = ?err,
                    "{} Failed to forward outbound message to downstream actor. Error: {:?}",
                    self.network_context, err
                );
            },
            None => {
                warn!(
                    NetworkSchema::new(&self.network_context).remote_peer(&peer_id),
                    protocol_id = %protocol_id,
                    "{} Can't send message to peer.  Peer {} is currently not connected",
                    self.network_context,
                    peer_id.short_str()
                );
            },
            _ => {}, // Success case - no logging needed
        }
    }

    fn start_connection_listener(&mut self) {
        let transport_handler = self
            .transport_handler
            .take()
            .expect("Transport handler already taken");
        self.executor.spawn(transport_handler.listen());
    }

    /// In the event two peers simultaneously dial each other we need to be able to do
    /// tie-breaking to determine which connection to keep and which to drop in a deterministic
    /// way. One simple way is to compare our local PeerId with that of the remote's PeerId and
    /// keep the connection where the peer with the greater PeerId is the dialer.
    ///
    /// Returns `true` if the existing connection should be dropped and `false` if the new
    /// connection should be dropped.
    fn simultaneous_dial_tie_breaking(
        own_peer_id: PeerId,
        remote_peer_id: PeerId,
        existing_origin: ConnectionOrigin,
        new_origin: ConnectionOrigin,
    ) -> bool {
        match (existing_origin, new_origin) {
            // If the remote dials while an existing connection is open, the older connection is
            // dropped.
            (ConnectionOrigin::Inbound, ConnectionOrigin::Inbound) => true,
            // We should never dial the same peer twice, but if we do drop the old connection
            (ConnectionOrigin::Outbound, ConnectionOrigin::Outbound) => true,
            (ConnectionOrigin::Inbound, ConnectionOrigin::Outbound) => remote_peer_id < own_peer_id,
            (ConnectionOrigin::Outbound, ConnectionOrigin::Inbound) => own_peer_id < remote_peer_id,
        }
    }

    fn disconnect(&mut self, connection: Connection<TSocket>) {
        let network_context = self.network_context;
        let time_service = self.time_service.clone();

        // Close connection, and drop it
        let drop_fut = async move {
            let mut connection = connection;
            let peer_id = connection.metadata.remote_peer_id;
            if let Err(e) = time_service
                .timeout(TRANSPORT_TIMEOUT, connection.socket.close())
                .await
            {
                warn!(
                    NetworkSchema::new(&network_context)
                        .remote_peer(&peer_id),
                    error = %e,
                    "{} Closing connection with Peer {} failed with error: {}",
                    network_context,
                    peer_id.short_str(),
                    e
                );
            };
        };
        self.executor.spawn(drop_fut);
    }

    fn add_peer(&mut self, connection: Connection<TSocket>) -> Result<(), Error> {
        let conn_meta = connection.metadata.clone();
        let peer_id = conn_meta.remote_peer_id;
        let connection_id = conn_meta.connection_id;

        // Make a disconnect if you've connected to yourself
        if self.network_context.peer_id() == peer_id {
            debug_assert!(false, "Self dials shouldn't happen");
            warn!(
                NetworkSchema::new(&self.network_context)
                    .connection_metadata_with_address(&conn_meta),
                "Received self-dial, disconnecting it"
            );
            self.disconnect(connection);
            return Ok(());
        }

        // Check if this is the first connection to this peer (for notification purposes)
        let is_first_connection = !self.has_connection(&peer_id);

        // Check if we're at max connections for this peer
        if !self.can_accept_connection(&peer_id) {
            // We're at max connections, need to apply tie-breaking
            // Find the oldest connection (first in the list) to compare against
            if let Some(existing_conn_ids) = self.peer_connections.get(&peer_id).cloned() {
                if let Some(&oldest_conn_id) = existing_conn_ids.first() {
                    if let Some(oldest_conn) = self.connections.get(&oldest_conn_id) {
                        let existing_origin = oldest_conn.metadata.origin;
                        if Self::simultaneous_dial_tie_breaking(
                            self.network_context.peer_id(),
                            peer_id,
                            existing_origin,
                            conn_meta.origin,
                        ) {
                            // Close the oldest existing connection and accept the new one
                            if let Some(removed_conn) = self.remove_connection(oldest_conn_id) {
                                self.remove_peer_from_metadata(peer_id, oldest_conn_id);
                                drop(removed_conn.sender);
                                info!(
                                    NetworkSchema::new(&self.network_context).remote_peer(&peer_id),
                                    "{} Closing existing connection {:?} with Peer {} to accept new connection",
                                    self.network_context,
                                    oldest_conn_id,
                                    peer_id.short_str()
                                );
                            }
                        } else {
                            // Keep existing connections, reject the new one
                            info!(
                                NetworkSchema::new(&self.network_context).remote_peer(&peer_id),
                                "{} Closing incoming connection with Peer {} (at max connections)",
                                self.network_context,
                                peer_id.short_str()
                            );
                            self.disconnect(connection);
                            return Ok(());
                        }
                    }
                }
            }
        }

        // TODO: Add label for peer.
        let (peer_reqs_tx, peer_reqs_rx) = aptos_channel::new(
            QueueStyle::FIFO,
            self.channel_size,
            Some(&counters::PENDING_NETWORK_REQUESTS),
        );

        // Initialize a new Peer actor for this connection.
        let peer = Peer::new(
            self.network_context,
            self.executor.clone(),
            self.time_service.clone(),
            connection,
            self.transport_notifs_tx.clone(),
            peer_reqs_rx,
            self.upstream_handlers.clone(),
            Duration::from_millis(constants::INBOUND_RPC_TIMEOUT_MS),
            constants::MAX_CONCURRENT_INBOUND_RPCS,
            constants::MAX_CONCURRENT_OUTBOUND_RPCS,
            self.max_frame_size,
            self.max_message_size,
        );
        self.executor.spawn(peer.start());

        // Create the PeerConnection and add it to our data structures
        let peer_connection = PeerConnection {
            connection_id,
            peer_id,
            metadata: conn_meta.clone(),
            sender: peer_reqs_tx,
        };
        self.add_connection(peer_connection);

        self.peers_and_metadata.insert_connection_metadata(
            PeerNetworkId::new(self.network_context.network_id(), peer_id),
            conn_meta.clone(),
        )?;

        // Send NewPeer notification only for the first connection to this peer
        if is_first_connection {
            let notif =
                ConnectionNotification::NewPeer(conn_meta, self.network_context.network_id());
            self.send_conn_notification(peer_id, notif);
        }

        Ok(())
    }

    /// Sends a `ConnectionNotification` to all event handlers, warns on failures
    fn send_conn_notification(&mut self, peer_id: PeerId, notification: ConnectionNotification) {
        for handler in self.connection_event_handlers.iter_mut() {
            if let Err(e) = handler.push(peer_id, notification.clone()) {
                warn!(
                    NetworkSchema::new(&self.network_context)
                        .remote_peer(&peer_id),
                    error = ?e,
                    connection_notification = notification,
                    "{} Failed to send notification {} to handler for peer: {}. Error: {:?}",
                    self.network_context,
                    notification,
                    peer_id.short_str(),
                    e
                );
            }
        }
    }
}
