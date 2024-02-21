// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_config::config::{NetworkConfig, PeerRole};
use aptos_config::network_id::{NetworkContext, PeerNetworkId};
use aptos_netcore::transport::{ConnectionOrigin, Transport};
use aptos_network2::transport::Connection;
use aptos_network2::application::ApplicationCollector;
use aptos_network2::application::storage::PeersAndMetadata;
use aptos_network2::logging::NetworkSchema;
use aptos_network2::protocols::network::OutboundPeerConnections;
use aptos_types::network_address::NetworkAddress;
use tokio::runtime::Handle;
use aptos_logger::{error, info, warn};
use aptos_network2::{counters, peer};
use aptos_network2::application::metadata::PeerMetadata;
use aptos_short_hex_str::AsShortHexStr;
use futures::{AsyncRead, AsyncWrite, AsyncWriteExt, StreamExt};
use std::marker::PhantomData;
use std::sync::Arc;

pub struct PeerListener<TTransport, TSocket>
    where
        TTransport: Transport,
        TSocket: AsyncRead + AsyncWrite,
{
    transport: TTransport,
    peers_and_metadata: Arc<PeersAndMetadata>,
    peer_cache: Vec<(PeerNetworkId,PeerMetadata)>,
    peer_cache_generation: u32,
    config: NetworkConfig,
    network_context: NetworkContext,
    apps: Arc<ApplicationCollector>,
    peer_senders: Arc<OutboundPeerConnections>,
    _ph2 : PhantomData<TSocket>,
}

impl<TTransport, TSocket> PeerListener<TTransport, TSocket>
    where
        TTransport: Transport<Output = Connection<TSocket>> + Send + 'static,
        TSocket: aptos_network2::transport::TSocket,
{
    pub fn new(
        transport: TTransport,
        peers_and_metadata: Arc<PeersAndMetadata>,
        config: NetworkConfig,
        network_context: NetworkContext,
        apps: Arc<ApplicationCollector>,
        peer_senders: Arc<OutboundPeerConnections>,
    ) -> Self {
        Self{
            transport,
            peers_and_metadata,
            peer_cache: vec![],
            peer_cache_generation: 0,
            config,
            network_context,
            apps,
            peer_senders,
            _ph2: Default::default(),
        }
    }

    fn maybe_update_peer_cache(&mut self) {
        // if no update is needed, this should be very fast
        // otherwise make copy of peers for use by this thread/task
        if let Some((update, update_generation)) = self.peers_and_metadata.get_all_peers_and_metadata_generational(self.peer_cache_generation, true, &[]) {
            self.peer_cache = update;
            self.peer_cache_generation = update_generation;
        }
    }

    pub(crate) fn listen(
        mut self,
        listen_addr: NetworkAddress,
        executor: Handle,
    ) -> Result<NetworkAddress, <TTransport>::Error> {
        let (sockets, listen_addr_actual) = executor.block_on(self.first_listen(listen_addr))?;
        info!("listener_thread to spawn ({:?})", listen_addr_actual);
        executor.spawn(self.listener_thread(sockets, executor.clone()));
        Ok(listen_addr_actual)
    }

    async fn first_listen(&mut self, listen_addr: NetworkAddress) -> Result<(<TTransport>::Listener, NetworkAddress), TTransport::Error> {
        self.transport.listen_on(listen_addr)
    }

    async fn listener_thread(mut self, mut sockets: <TTransport>::Listener, executor: Handle) {
        // TODO: leave some connection that can close and shutdown this listener?
        info!("listener_thread start");
        loop {
            let (conn_fut, remote_addr) = match sockets.next().await {
                Some(result) => match result {
                    Ok(conn) => { conn }
                    Err(err) => {
                        error!("listener_thread {:?} got err {:?}, exiting", self.config.network_id, err);
                        return;
                    }
                }
                None => {
                    error!("listener_thread {:?} got None, assuming source closed, exiting", self.config.network_id, );
                    return;
                }
            };
            match conn_fut.await {
                Ok(mut connection) => {
                    let ok = self.check_new_inbound_connection(&connection);
                    info!("listener_thread got connection {:?}, ok={:?}", remote_addr, ok);
                    if !ok {
                        // conted and logged inside check function above, just close here and be done.
                        _ = connection.socket.close().await;
                        continue;
                    }
                    let remote_peer_network_id = PeerNetworkId::new(self.network_context.network_id(), connection.metadata.remote_peer_id);
                    peer::start_peer(
                        &self.config,
                        connection.socket,
                        connection.metadata,
                        self.apps.clone(),
                        executor.clone(),
                        remote_peer_network_id,
                        self.peers_and_metadata.clone(),
                        self.peer_senders.clone(),
                        self.network_context,
                    );
                }
                Err(err) => {
                    error!(addr = remote_addr, "listener_thread {:?} connection post-processing failed (continuing): {:?}", self.config.network_id, err);
                }
            }
        }
    }

    // is the new inbound connection okay? => true
    // no, we should disconnect => false
    fn check_new_inbound_connection(&mut self, conn: &Connection<TSocket>) -> bool {
        // Everything below here is meant for unknown peers only. The role comes from
        // the Noise handshake and if it's not `Unknown` then it is trusted.
        // TODO: do more checking for 'trusted' peers
        if conn.metadata.role != PeerRole::Unknown {
            return true;
        }

        // Count unknown inbound connections
        self.maybe_update_peer_cache();
        let mut unknown_inbound_conns = 0;
        let mut already_connected = false;
        let remote_peer_id = conn.metadata.remote_peer_id;

        if remote_peer_id == self.network_context.peer_id() {
            debug_assert!(false, "Self dials shouldn't happen");
            warn!(
                NetworkSchema::new(&self.network_context)
                    .connection_metadata_with_address(&conn.metadata),
                "Received self-dial, disconnecting it"
            );
            return false;
        }

        for wat in self.peer_cache.iter() {
            if wat.0.peer_id() == remote_peer_id {
                already_connected = true;
            }
            let remote_metadata = wat.1.get_connection_metadata();
            if remote_metadata.origin == ConnectionOrigin::Inbound && remote_metadata.role == PeerRole::Unknown {
                unknown_inbound_conns += 1;
            }
        }

        // Reject excessive inbound connections made by unknown peers
        // We control outbound connections with Connectivity manager before we even send them
        // and we must allow connections that already exist to pass through tie breaking.
        if !already_connected
            && unknown_inbound_conns + 1 > self.config.max_inbound_connections
        {
            info!(
                NetworkSchema::new(&self.network_context)
                .connection_metadata_with_address(&conn.metadata),
                "{} Connection rejected due to connection limit: {}",
                self.network_context,
                conn.metadata
            );
            counters::connections_rejected(&self.network_context, conn.metadata.origin).inc();
            return false;
        }

        if already_connected {
            // old code at network/framework/src/peer_manager/mod.rs PeerManager::add_peer() line 615 had provision for sometimes keeping the new connection, but this simplifies and always _drops_ the new connection
            info!(
                NetworkSchema::new(&self.network_context)
                .connection_metadata_with_address(&conn.metadata),
                "{} Closing incoming connection with Peer {} which is already connected",
                self.network_context,
                remote_peer_id.short_str()
            );
            false
        } else {
            true
        }
    }
}
