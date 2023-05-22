// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

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
    peer::{Peer, PeerNotification, PeerRequest},
    transport::{
        Connection, ConnectionId, ConnectionMetadata, TSocket as TransportTSocket,
        TRANSPORT_TIMEOUT,
    },
    ProtocolId,
};
use aptos_channels::{self, aptos_channel, message_queues::QueueStyle};
use aptos_config::network_id::{NetworkContext, PeerNetworkId};
use aptos_logger::prelude::*;
use aptos_netcore::transport::{ConnectionOrigin, Transport};
use aptos_short_hex_str::AsShortHexStr;
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_types::{network_address::NetworkAddress, PeerId};
use futures::{channel::oneshot, io::{AsyncRead, AsyncWrite, AsyncWriteExt}, sink::SinkExt, Stream, stream::StreamExt};
use std::{
    collections::{hash_map::Entry, HashMap},
    marker::PhantomData,
    sync::Arc,
    time::Duration,
};
use std::cell::RefCell;
use std::ops::DerefMut;
use std::pin::Pin;
use std::task::{Context, Poll};
use futures_util::stream::FusedStream;
use tokio::runtime::Handle;
use tokio::sync::mpsc::{Receiver, Sender};
use aptos_channels::fused_receiver_stream::FusedReceiverStream;

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
    protocols::network::SerializedRequest,
};
use aptos_config::config::PeerRole;
use aptos_types::account_address::AccountAddress;
pub use senders::*;
pub use types::*;
use crate::protocols::network::{IncomingMessage, IncomingRpcRequest};
use crate::protocols::rpc::OutboundRpcs;
use crate::protocols::wire::messaging::v1::{DirectSendMsg, NetworkMessage, RpcRequest, RpcResponse};

/// PmPeer is the state that the PeerManager holds about a peer.
/// This is different than the Peer object which is used by threads active in servicing the peer.
struct PmPeer {
    conn_metadata: ConnectionMetadata,
    sender: Sender<NetworkMessage>,
    outbound_rpcs: OutboundRpcs,
}

impl PmPeer {
    fn new(
        conn_metadata: ConnectionMetadata,
        sender: Sender<NetworkMessage>,
        network_context: NetworkContext,
        time_service: TimeService,
        max_concurrent_outbound_rpcs: u32,
    ) -> Self {
        let remote_peer_id = conn_metadata.remote_peer_id;
        Self {
            conn_metadata,
            sender,
            outbound_rpcs: OutboundRpcs::new(
                network_context,
                time_service,
                remote_peer_id,
                max_concurrent_outbound_rpcs,
            ),
        }
    }
}

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
    /// Map from PeerId to corresponding Peer object.
    /// TODO: make the value side a struct PmPeer{ConnectionMetadata,Sender<NetworkMessage>,OutboundRpcs} and remove OutboundRpcs from Peer
    active_peers: HashMap<PeerId, PmPeer>,
    /// Shared metadata storage about trusted peers and metadata
    peers_and_metadata: Arc<PeersAndMetadata>,
    /// Channel to receive requests from other actors.
    requests_rx: aptos_channel::Receiver<(PeerId, ProtocolId), PeerManagerRequest>,
    /// Upstream handlers for RPC and DirectSend protocols. The handlers are promised fair delivery
    /// of messages across (PeerId, ProtocolId).
    // upstream_handlers:
    //     HashMap<ProtocolId, aptos_channel::Sender<(PeerId, ProtocolId), PeerManagerNotification>>,
    direct_map : HashMap<ProtocolId, tokio::sync::mpsc::Sender<IncomingMessage>>, // TODO: not HashMap but LUT
    rpc_map : HashMap<ProtocolId, tokio::sync::mpsc::Sender<IncomingRpcRequest>>, // TODO: not HashMap but LUT
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
    /// Maximum concurrent network requests to any peer.
    max_concurrent_network_reqs: usize,
    /// Size of channels between different actors.
    channel_size: usize,
    /// Max network frame size
    max_frame_size: usize,
    /// Max network message size
    max_message_size: usize,
    /// Inbound connection limit separate of outbound connections
    inbound_connection_limit: usize,
    /// rpc response messages in from peers
    rpc_response_tx: tokio::sync::mpsc::Sender<(PeerId, RpcResponse)>,
    //rpc_response_rx: Box<dyn FusedRpcReceiver<Item=()>>,//tokio_stream::stream_ext::fuse::Fuse<RpcResponse>,
    rpc_response_rx: FusedReceiverStream<(PeerId, RpcResponse)>,//tokio_stream::stream_ext::fuse::Fuse<RpcResponse>,
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
        // upstream_handlers: HashMap<
        //     ProtocolId,
        //     aptos_channel::Sender<(PeerId, ProtocolId), PeerManagerNotification>,
        // >,
        direct_map : HashMap<ProtocolId, tokio::sync::mpsc::Sender<IncomingMessage>>, // TODO: not HashMap but LUT
        rpc_map : HashMap<ProtocolId, tokio::sync::mpsc::Sender<IncomingRpcRequest>>, // TODO: not HashMap but LUT
        connection_event_handlers: Vec<conn_notifs_channel::Sender>,
        channel_size: usize,
        max_concurrent_network_reqs: usize,
        max_frame_size: usize,
        max_message_size: usize,
        inbound_connection_limit: usize,
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
        let (rpc_response_tx, rpc_response_rx) = tokio::sync::mpsc::channel(1024); // TODO: configure
        let mut rpc_response_rx = FusedReceiverStream::new(rpc_response_rx);//Box::pin(FusedReceiverStream::new(rpc_response_rx).fuse());

        Self {
            network_context,
            executor,
            time_service,
            listen_addr,
            transport_handler: Some(transport_handler),
            active_peers: HashMap::new(),
            peers_and_metadata,
            requests_rx,
            connection_reqs_rx,
            transport_reqs_tx,
            transport_notifs_tx,
            transport_notifs_rx,
            outstanding_disconnect_requests: HashMap::new(),
            phantom_transport: PhantomData,
            //upstream_handlers,
            direct_map,
            rpc_map,
            connection_event_handlers,
            max_concurrent_network_reqs,
            channel_size,
            max_frame_size,
            max_message_size,
            inbound_connection_limit,
            rpc_response_tx,
            rpc_response_rx,
        }
    }

    pub fn update_connected_peers_metrics(&self) {
        let total = self.active_peers.len();
        let inbound = self
            .active_peers
            .iter()
            .filter(|(_,pmp)| pmp.conn_metadata.origin == ConnectionOrigin::Inbound)
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
                .active_peers
                .values()
                .map(|x| {
                    (
                        x.conn_metadata.remote_peer_id,
                        x.conn_metadata.addr.clone(),
                        x.conn_metadata.origin,
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

    /// Get the [`NetworkAddress`] we're listening for incoming connections on
    pub fn listen_addr(&self) -> &NetworkAddress {
        &self.listen_addr
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
                peer_id_response = self.rpc_response_rx.select_next_some() => {
                    let (peer_id, response) = peer_id_response;
                    self.handle_rpc_response(peer_id, response).await;
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
                // If the active connection with the peer is lost, remove it from `active_peers`.
                if let Entry::Occupied(entry) = self.active_peers.entry(peer_id) {
                    let pmp = entry.get();
                    let connection_id = pmp.conn_metadata.connection_id;
                    if connection_id == lost_conn_metadata.connection_id {
                        // We lost an active connection.
                        entry.remove();
                        self.remove_peer_from_metadata(peer_id, connection_id);
                    }
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

                // Notify upstream if there's still no active connection. This might be redundant,
                // but does not affect correctness.
                if !self.active_peers.contains_key(&peer_id) {
                    let notif = ConnectionNotification::LostPeer(
                        lost_conn_metadata,
                        self.network_context,
                        reason,
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

        // Verify that we have not reached the max connection limit for unknown inbound peers
        if conn.metadata.origin == ConnectionOrigin::Inbound {
            // Everything below here is meant for unknown peers only. The role comes from
            // the Noise handshake and if it's not `Unknown` then it is trusted.
            if conn.metadata.role == PeerRole::Unknown {
                // TODO: Keep track of somewhere else to not take this hit in case of DDoS
                // Count unknown inbound connections
                let unknown_inbound_conns = self
                    .active_peers
                    .iter()
                    .filter(|(peer_id, pmp)| {
                        pmp.conn_metadata.origin == ConnectionOrigin::Inbound
                            && trusted_peers
                                .read()
                                .get(peer_id)
                                .map_or(true, |peer| peer.role == PeerRole::Unknown)
                    })
                    .count();

                // Reject excessive inbound connections made by unknown peers
                // We control outbound connections with Connectivity manager before we even send them
                // and we must allow connections that already exist to pass through tie breaking.
                if !self
                    .active_peers
                    .contains_key(&conn.metadata.remote_peer_id)
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
                // Only dial peers which we aren't already connected with
                    if let Some(pmp) = self.active_peers.get(&requested_peer_id) {
                            let error = PeerManagerError::AlreadyConnected(pmp.conn_metadata.addr.clone());
                            debug!(
                        NetworkSchema::new(&self.network_context)
                            .connection_metadata_with_address(&pmp.conn_metadata),
                        "{} Already connected to Peer {} with connection {:?}. Not dialing address {}",
                        self.network_context,
                        requested_peer_id.short_str(),
                        pmp.conn_metadata,
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
                            let request = TransportRequest::DialPeer(requested_peer_id, addr, response_tx);
                            self.transport_reqs_tx.send(request).await.unwrap();
                        //}

            },
            ConnectionRequest::DisconnectPeer(peer_id, resp_tx) => {
                // Send a CloseConnection request to Peer and drop the send end of the
                // PeerRequest channel.
                if let Some(pmp) = self.active_peers.remove(&peer_id) {
                    let connection_id = pmp.conn_metadata.connection_id;
                    self.remove_peer_from_metadata(pmp.conn_metadata.remote_peer_id, connection_id);

                    // This triggers a disconnect.
                    drop(pmp.sender);
                    // Add to outstanding disconnect requests.
                    self.outstanding_disconnect_requests
                        .insert(connection_id, resp_tx);
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
                (peer_id, msg.protocol_id(), NetworkMessage::DirectSendMsg(DirectSendMsg{
                    protocol_id: msg.protocol_id(),
                    priority: 0,
                    raw_msg: msg.mdata.to_vec(),
                }))
            },
            PeerManagerRequest::SendRpc(peer_id, req) => {
                if let Some(pmp) = self.active_peers.get_mut(&peer_id) {
                    pmp.outbound_rpcs.handle_outbound_request(req, &mut pmp.sender);
                } else {
                    // TODO: err log, count
                }
                return;
                // (peer_id, req.protocol_id(), NetworkMessage::RpcRequest(RpcRequest{
                //     protocol_id: req.protocol_id(),
                //     request_id: 0, // TODO: set request_id?
                //     priority: 0,
                //     raw_request: req.data.to_vec(),
                // }))
            },
            PeerManagerRequest::SendRpcReply(peer_id, request_id, msg) => {
                (peer_id, msg.protocol_id(), NetworkMessage::RpcResponse(RpcResponse{
                    request_id,
                    priority: 0,
                    raw_response: msg.mdata.to_vec(),
                }))
            }
        };

        if let Some(pmp) = self.active_peers.get_mut(&peer_id) {
            if let Err(err) = pmp.sender.try_send(peer_request) {
                info!(
                    NetworkSchema::new(&self.network_context).connection_metadata(&pmp.conn_metadata),
                    protocol_id = %protocol_id,
                    error = ?err,
                    "{} Failed to forward outbound message to downstream actor. Error: {:?}",
                    self.network_context, err
                );
            }
        } else {
            warn!(
                NetworkSchema::new(&self.network_context).remote_peer(&peer_id),
                protocol_id = %protocol_id,
                "{} Can't send message to peer.  Peer {} is currently not connected",
                self.network_context,
                peer_id.short_str()
            );
        }
    }

    async fn handle_rpc_response(&mut self, peer_id: PeerId, response: RpcResponse) {
        if let Some(pmp) = self.active_peers.get_mut(&peer_id) {
            pmp.outbound_rpcs.handle_inbound_response(response)
        } else {
            // TODO: log, counter (response with no internal accounting to match it to)
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

        let mut send_new_peer_notification = true;

        // Check for and handle simultaneous dialing
        if let Entry::Occupied(active_entry) = self.active_peers.entry(peer_id) {
            let pmp = active_entry.get();
            if Self::simultaneous_dial_tie_breaking(
                self.network_context.peer_id(),
                peer_id,
                pmp.conn_metadata.origin,
                conn_meta.origin,
            ) {
                let pmp = active_entry.remove();
                // Drop the existing connection and replace it with the new connection
                drop(pmp.sender);
                info!(
                    NetworkSchema::new(&self.network_context).remote_peer(&peer_id),
                    "{} Closing existing connection with Peer {} to mitigate simultaneous dial",
                    self.network_context,
                    peer_id.short_str()
                );
                send_new_peer_notification = false;
            } else {
                info!(
                    NetworkSchema::new(&self.network_context).remote_peer(&peer_id),
                    "{} Closing incoming connection with Peer {} to mitigate simultaneous dial",
                    self.network_context,
                    peer_id.short_str()
                );
                // Drop the new connection and keep the one already stored in active_peers
                self.disconnect(connection);
                return Ok(());
            }
        }

        // TODO: Add label for peer.
        // messages towards network peer
        let (peer_reqs_tx, peer_reqs_rx) = tokio::sync::mpsc::channel::<NetworkMessage>(self.channel_size);
        let peer_reqs_rx = vec![RefCell::new(peer_reqs_rx)];
        // TODO: Add label for peer.
        // messages from network peer
        let (peer_notifs_tx, peer_notifs_rx) = tokio::sync::mpsc::channel::<NetworkMessage>(self.channel_size);

        // Initialize a new Peer actor for this connection.
        let peer = Peer::new(
            self.network_context,
            self.executor.clone(),
            self.time_service.clone(),
            connection,
            self.transport_notifs_tx.clone(),
            peer_notifs_tx,
            Duration::from_millis(constants::INBOUND_RPC_TIMEOUT_MS),
            constants::MAX_CONCURRENT_INBOUND_RPCS,
            constants::MAX_CONCURRENT_OUTBOUND_RPCS,
            self.max_frame_size,
            self.max_message_size,
        );
        self.executor.spawn(peer.start(peer_reqs_rx));

        // Start background task to handle events (RPCs and DirectSend messages) received from
        // peer.
        self.spawn_peer_network_events_handler(peer_id, peer_notifs_rx);
        // Save PeerRequest sender to `active_peers`.
        self.active_peers
            .insert(peer_id, PmPeer::new(conn_meta.clone(), peer_reqs_tx, self.network_context, self.time_service.clone(), constants::MAX_CONCURRENT_OUTBOUND_RPCS));
        self.peers_and_metadata.insert_connection_metadata(
            PeerNetworkId::new(self.network_context.network_id(), peer_id),
            conn_meta.clone(),
        )?;
        // Send NewPeer notification to connection event handlers.
        if send_new_peer_notification {
            let notif = ConnectionNotification::NewPeer(conn_meta, self.network_context);
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

    fn spawn_peer_network_events_handler(
        &self,
        peer_id: PeerId,
        network_events: tokio::sync::mpsc::Receiver<NetworkMessage>,
    ) {
        //let mut upstream_handlers = self.upstream_handlers.clone();
        let mut direct_map = self.direct_map.clone();
        let mut rpc_map = self.rpc_map.clone();
        let network_context = self.network_context;
        let ne_stream = FusedReceiverStream::new(network_events);
        let mut rpc_response = self.rpc_response_tx.clone();
        self.executor.spawn(ne_stream.for_each_concurrent(
            self.max_concurrent_network_reqs,
            move |inbound_event| {
                handle_inbound_request(
                    network_context,
                    inbound_event,
                    peer_id,
                    //&mut upstream_handlers,
                    &mut direct_map,
                    &mut rpc_map,
                    &mut rpc_response,
                );
                futures::future::ready(())
            },
        ));
    }
}



/// A task for consuming inbound network messages
fn handle_inbound_request(
    network_context: NetworkContext,
    inbound_event: NetworkMessage,
    peer_id: PeerId,
    // upstream_handlers: &mut HashMap<
    //     ProtocolId,
    //     aptos_channel::Sender<(PeerId, ProtocolId), PeerManagerNotification>,
    // >,
    direct_map: &mut HashMap<ProtocolId, Sender<IncomingMessage>>,
    rpc_map: &mut HashMap<ProtocolId, Sender<IncomingRpcRequest>>,
    rpc_response: &mut Sender<(PeerId, RpcResponse)>,
) {
    match inbound_event {
        NetworkMessage::DirectSendMsg(msg) => {
            if let Some(handler) = direct_map.get_mut(&msg.protocol_id) {
                let im = IncomingMessage {
                    sender: PeerNetworkId::new(network_context.network_id(), peer_id),
                    protocol_id: msg.protocol_id,
                    priority: msg.priority,
                    message: msg.raw_msg.into(),
                };
                if let Err(e) = handler.try_send(im) {
                    // TODO: log, counter
                }
            } else {
                // TODO: debug msg, counter
            }
        }
        NetworkMessage::RpcRequest(msg) => {
            let handler = match rpc_map.get_mut(&msg.protocol_id) {
                Some(x) => x,
                None => {
                    // TODO: log, counter
                    return;
                }
            };
            let im = IncomingRpcRequest {
                core: IncomingMessage {
                    sender: PeerNetworkId::new(network_context.network_id(), peer_id),
                    protocol_id: msg.protocol_id,
                    priority: msg.priority,
                    message: msg.raw_request.into(),
                },
                request_id: msg.request_id,
            };
            if let Err(e) = handler.try_send(im) {
                // TODO: log, counter
            }
        }
        NetworkMessage::RpcResponse(res) => {
            if let Err(e) = rpc_response.try_send((peer_id, res)) {
                // TODO: log, counter (internal queue was full, message dropped)
            }
        }
        NetworkMessage::Error(_) => {return} // don't care, drop. TODO: counter? log?
    };
    // let (protocol_id, notification) = match inbound_event {
        // PeerNotification::RecvMessage(msg) => (
        //     msg.protocol_id(),
        //     PeerManagerNotification::RecvMessage(peer_id, msg),
        // ),
        // PeerNotification::RecvRpc(req) => (
        //     req.protocol_id(),
        //     PeerManagerNotification::RecvRpc(peer_id, req),
        // ),
    // };

    // if let Some(handler) = upstream_handlers.get_mut(&protocol_id) {
    //     let notification = PeerManagerNotification::RecvMessage(peer_id, Message {});
    //     // Send over aptos channel for fairness.
    //     if let Err(err) = handler.push((peer_id, protocol_id), notification) {
    //         warn!(
    //             NetworkSchema::new(&network_context),
    //             error = ?err,
    //             protocol_id = protocol_id,
    //             "{} Upstream handler unable to handle message for protocol: {}. Error: {:?}",
    //             network_context, protocol_id, err
    //         );
    //     }
    // } else {
    //     debug!(
    //         NetworkSchema::new(&network_context),
    //         protocol_id = protocol_id,
    //         message = format!("{:?}", notification),
    //         "{} Received network message for unregistered protocol: {:?}",
    //         network_context,
    //         notification,
    //     );
    // }
}
