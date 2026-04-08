// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! [`Peer`] manages a single connection to a remote peer after the initial connection
//! establishment and handshake.
//!
//! Its responsibilities include sending and receiving [`NetworkMessage`]s
//! over-the-wire, maintaining a completion queue of pending RPC requests (through
//! the [`InboundRpcs`] and [`OutboundRpcs`] completion queues), and eventually
//! shutting down when the [`PeerManager`] requests it or the connection is lost.
//!
//! [`Peer`] owns the actual underlying connection socket and is reponsible for
//! the socket's shutdown, graceful or otherwise.
//!
//! [`PeerManager`]: crate::peer_manager::PeerManager

use crate::{
    counters::{
        self, network_application_inbound_traffic, network_application_outbound_traffic,
        DECLINED_LABEL, FAILED_LABEL, RECEIVED_LABEL, SENT_LABEL, UNKNOWN_LABEL,
    },
    logging::NetworkSchema,
    peer::rate_limiter::InboundMessageRateLimiter,
    peer_manager::{PeerManagerError, TransportNotification},
    protocols::{
        direct_send::Message,
        network::ReceivedMessage,
        rpc::{error::RpcError, InboundRpcs, OutboundRpcRequest, OutboundRpcs},
        stream::{InboundStreamBuffer, OutboundStream, StreamMessage},
        wire::messaging::v1::{
            DirectSendMsg, ErrorCode, MultiplexMessage, MultiplexMessageSink,
            MultiplexMessageStream, NetworkMessage, Priority, ReadError, RpcResponse, WriteError,
        },
    },
    transport::{self, Connection, ConnectionMetadata},
    ProtocolId,
};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::network_id::{NetworkContext, PeerNetworkId};
use aptos_logger::prelude::*;
use aptos_short_hex_str::AsShortHexStr;
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_types::PeerId;
use futures::{
    self,
    channel::oneshot,
    io::{AsyncRead, AsyncWrite},
    stream::{FusedStream, StreamExt},
    FutureExt, SinkExt,
};
use futures_util::stream::select;
use serde::Serialize;
use std::{collections::HashMap, fmt, panic, sync::Arc, time::Duration};
use tokio::{runtime::Handle, sync::watch, time::timeout};
use tokio_util::compat::{
    FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt,
};

#[cfg(test)]
mod test;

#[cfg(any(test, feature = "fuzzing"))]
pub mod fuzzing;

pub(crate) mod rate_limiter;

/// Requests [`Peer`] receives from the [`PeerManager`](crate::peer_manager::PeerManager).
#[derive(Debug)]
pub enum PeerRequest {
    /// Send an RPC request to peer.
    SendRpc(OutboundRpcRequest),
    /// Fire-and-forget style message send to peer.
    SendDirectSend(Message),
}

/// The reason for closing a network connection
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum DisconnectReason {
    ConnectionClosed, // The connection was gracefully closed (e.g., by the peer)
    InputOutputError, // An I/O error occurred on the connection (e.g., when reading messages)
    NetworkHealthCheckFailure, // The connection failed the network health check (e.g., pings)
    PeerMonitoringPingFailure, // The connection failed the peer monitoring ping check
    RequestedByPeerManager, // The peer manager requested the connection to be closed
    StaleConnection,  // The connection is stale (e.g., when a validator leaves the validator set)
}

impl DisconnectReason {
    /// Returns a string label for the disconnect reason
    pub fn get_label(&self) -> String {
        let label = match self {
            DisconnectReason::ConnectionClosed => "ConnectionClosed",
            DisconnectReason::InputOutputError => "InputOutputError",
            DisconnectReason::NetworkHealthCheckFailure => "NetworkHealthCheckFailure",
            DisconnectReason::PeerMonitoringPingFailure => "PeerMonitoringPingFailure",
            DisconnectReason::RequestedByPeerManager => "RequestedByPeerManager",
            DisconnectReason::StaleConnection => "StaleConnection",
        };
        label.to_string()
    }
}

impl fmt::Display for DisconnectReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get_label())
    }
}

/// The `Peer` actor manages a single connection to another remote peer after
/// the initial connection establishment and handshake.
pub struct Peer<TSocket> {
    /// The network instance this Peer actor is running under.
    network_context: NetworkContext,
    /// A handle to a tokio executor.
    executor: Handle,
    /// A handle to a time service for easily mocking time-related operations.
    time_service: TimeService,
    /// Connection specific information.
    connection_metadata: ConnectionMetadata,
    /// Underlying connection.
    connection: Option<TSocket>,
    /// Channel to notify PeerManager that we've disconnected.
    connection_notifs_tx: aptos_channels::Sender<TransportNotification<TSocket>>,
    /// Channel to receive requests from PeerManager to send messages and rpcs.
    peer_reqs_rx: aptos_channel::Receiver<ProtocolId, PeerRequest>,
    /// Where to send inbound messages and rpcs.
    upstream_handlers:
        Arc<HashMap<ProtocolId, aptos_channel::Sender<(PeerId, ProtocolId), ReceivedMessage>>>,
    /// Inbound rpc request queue for handling requests from remote peer.
    inbound_rpcs: InboundRpcs,
    /// Outbound rpc request queue for sending requests to remote peer and handling responses.
    outbound_rpcs: OutboundRpcs,
    /// The maximum size of an inbound or outbound request frame
    max_frame_size: usize,
    /// The maximum size of an inbound or outbound request message
    max_message_size: usize,
    /// Inbound stream buffer
    inbound_stream: InboundStreamBuffer,
    /// Optional per-peer inbound rate limiter
    inbound_rate_limiter: Option<InboundMessageRateLimiter>,
}

impl<TSocket> Peer<TSocket>
where
    TSocket: AsyncRead + AsyncWrite + Send + 'static,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        network_context: NetworkContext,
        executor: Handle,
        time_service: TimeService,
        connection: Connection<TSocket>,
        connection_notifs_tx: aptos_channels::Sender<TransportNotification<TSocket>>,
        peer_reqs_rx: aptos_channel::Receiver<ProtocolId, PeerRequest>,
        upstream_handlers: Arc<
            HashMap<ProtocolId, aptos_channel::Sender<(PeerId, ProtocolId), ReceivedMessage>>,
        >,
        inbound_rpc_timeout: Duration,
        max_concurrent_inbound_rpcs: u32,
        max_concurrent_outbound_rpcs: u32,
        max_frame_size: usize,
        max_message_size: usize,
        inbound_rate_limiter: Option<InboundMessageRateLimiter>,
    ) -> Self {
        let Connection {
            metadata: connection_metadata,
            socket,
        } = connection;
        let remote_peer_id = connection_metadata.remote_peer_id;
        let max_fragments = max_message_size / max_frame_size;
        // Build sub-components first, then move time_service into Self.
        // This keeps clone count the same (2 clones for InboundRpcs/OutboundRpcs)
        // but lets Self receive the original rather than a third clone.
        let inbound_rpcs = InboundRpcs::new(
            network_context,
            time_service.clone(),
            remote_peer_id,
            inbound_rpc_timeout,
            max_concurrent_inbound_rpcs,
        );
        let outbound_rpcs = OutboundRpcs::new(
            network_context,
            time_service.clone(),
            remote_peer_id,
            max_concurrent_outbound_rpcs,
        );
        Self {
            network_context,
            executor,
            time_service,
            connection_metadata,
            connection: Some(socket),
            connection_notifs_tx,
            peer_reqs_rx,
            upstream_handlers,
            inbound_rpcs,
            outbound_rpcs,
            max_frame_size,
            max_message_size,
            inbound_stream: InboundStreamBuffer::new(max_fragments),
            inbound_rate_limiter,
        }
    }

    fn remote_peer_id(&self) -> PeerId {
        self.connection_metadata.remote_peer_id
    }

    pub async fn start(mut self) {
        let remote_peer_id = self.remote_peer_id();
        trace!(
            NetworkSchema::new(&self.network_context)
                .connection_metadata(&self.connection_metadata),
            "{} Starting Peer actor for peer: {}",
            self.network_context,
            remote_peer_id.short_str()
        );

        // Split the connection into a ReadHalf and a WriteHalf.
        let (read_socket, write_socket) =
            tokio::io::split(self.connection.take().unwrap().compat());

        let reader = MultiplexMessageStream::new(read_socket.compat(), self.max_frame_size).fuse();
        let writer = MultiplexMessageSink::new(write_socket.compat_write(), self.max_frame_size);

        // Start writer "process" as a separate task. We receive two handles to
        // communicate with the task:
        //   1. `write_reqs_tx`: Queue of pending NetworkMessages to write.
        //   2. `close_tx`: Handle to close the task and underlying connection.
        let (write_reqs_tx, writer_close_tx) = Self::start_writer_task(
            &self.executor,
            self.time_service.clone(),
            self.connection_metadata.clone(),
            self.network_context,
            writer,
            self.max_frame_size,
            self.max_message_size,
        );

        // Channels for routing messages between the three event-loop tasks.
        let (inbound_rpc_req_tx, inbound_rpc_req_rx) =
            tokio::sync::mpsc::channel::<ReceivedMessage>(1024);
        let (rpc_resp_tx, rpc_resp_rx) = tokio::sync::mpsc::channel::<RpcResponse>(1024);

        // Shutdown coordination: all tasks share the same watch channel.
        // Any task can signal shutdown by sending Some(reason); all others observe
        // via shutdown_rx.changed().
        let (shutdown_tx, shutdown_rx) = watch::channel::<Option<DisconnectReason>>(None);
        let shutdown_tx = Arc::new(shutdown_tx);

        // Task 1: outbound handler — spawned on a separate tokio thread.
        // Drives PeerManager requests and outbound-RPC completions.
        self.executor.spawn(Self::run_outbound_handler_task(
            self.network_context,
            self.connection_metadata.clone(),
            self.peer_reqs_rx,
            rpc_resp_rx,
            write_reqs_tx.clone(),
            self.outbound_rpcs,
            shutdown_rx.clone(),
            Arc::clone(&shutdown_tx),
        ));

        // Task 2: inbound reader — spawned on a separate tokio thread.
        // Reads raw bytes off the wire, rate-limits, and dispatches by message type.
        self.executor.spawn(Self::run_inbound_reader_task(
            self.network_context,
            self.connection_metadata.clone(),
            reader,
            self.upstream_handlers.clone(),
            write_reqs_tx.clone(),
            inbound_rpc_req_tx,
            rpc_resp_tx,
            self.inbound_stream,
            self.inbound_rate_limiter,
            shutdown_rx.clone(),
            Arc::clone(&shutdown_tx),
        ));

        // Task 3: inbound RPC completions — runs inline on this thread.
        // Running inline (not spawned) preserves mock-time timing: tokio::task::yield_now()
        // in MockTimeService only yields to tasks on the same thread. If this ran on a worker
        // thread, time-advance in tests could race with RPC completion handling.
        drop(shutdown_tx);
        Self::run_inbound_rpc_task(
            self.network_context,
            self.connection_metadata.clone(),
            inbound_rpc_req_rx,
            write_reqs_tx.clone(),
            self.upstream_handlers,
            self.inbound_rpcs,
            shutdown_rx.clone(),
        )
        .await;

        let reason = shutdown_rx
            .borrow()
            .unwrap_or(DisconnectReason::ConnectionClosed);
        Self::do_shutdown(
            write_reqs_tx,
            writer_close_tx,
            reason,
            self.network_context,
            self.connection_metadata,
            self.connection_notifs_tx,
        )
        .await;
    }

    // Start a new task on the given executor which is responsible for writing outbound messages on
    // the wire. The function returns two channels which can be used to send instructions to the
    // task:
    // 1. The first channel is used to send outbound NetworkMessages to the task
    // 2. The second channel is used to instruct the task to close the connection and terminate.
    // If outbound messages are queued when the task receives a close instruction, it discards
    // them and immediately closes the connection.
    fn start_writer_task(
        executor: &Handle,
        time_service: TimeService,
        connection_metadata: ConnectionMetadata,
        network_context: NetworkContext,
        mut writer: MultiplexMessageSink<impl AsyncWrite + Unpin + Send + 'static>,
        max_frame_size: usize,
        max_message_size: usize,
    ) -> (
        aptos_channel::Sender<(), NetworkMessage>,
        oneshot::Sender<()>,
    ) {
        let remote_peer_id = connection_metadata.remote_peer_id;
        let (write_reqs_tx, mut write_reqs_rx): (aptos_channel::Sender<(), NetworkMessage>, _) =
            aptos_channel::new(
                QueueStyle::KLAST,
                1024,
                Some(&counters::PENDING_WIRE_MESSAGES),
            );
        let (close_tx, mut close_rx) = oneshot::channel();

        let (mut msg_tx, msg_rx) = aptos_channels::new(1024, &counters::PENDING_MULTIPLEX_MESSAGE);
        let (stream_msg_tx, stream_msg_rx) =
            aptos_channels::new(1024, &counters::PENDING_MULTIPLEX_STREAM);

        // this task ends when the multiplex task ends (by dropping the senders) or receiving a close instruction
        let writer_task = async move {
            let mut stream = select(msg_rx, stream_msg_rx);
            let log_context =
                NetworkSchema::new(&network_context).connection_metadata(&connection_metadata);
            loop {
                futures::select! {
                    message = stream.select_next_some() => {
                        if let Err(err) = timeout(transport::TRANSPORT_TIMEOUT,writer.send(&message)).await {
                            warn!(
                                log_context,
                                error = %err,
                                "{} Error in sending message to peer: {}",
                                network_context,
                                remote_peer_id.short_str(),
                            );
                        }
                    }
                    _ = close_rx => {
                        break;
                    }
                }
            }
            info!(
                log_context,
                "{} Closing connection to peer: {}",
                network_context,
                remote_peer_id.short_str()
            );
            let flush_and_close = async {
                writer.flush().await?;
                writer.close().await?;
                Ok(()) as Result<(), WriteError>
            };
            match time_service
                .timeout(transport::TRANSPORT_TIMEOUT, flush_and_close)
                .await
            {
                Err(_) => {
                    info!(
                        log_context,
                        "{} Timeout in flush/close of connection to peer: {}",
                        network_context,
                        remote_peer_id.short_str()
                    );
                },
                Ok(Err(err)) => {
                    info!(
                        log_context,
                        error = %err,
                        "{} Failure in flush/close of connection to peer: {}, error: {}",
                        network_context,
                        remote_peer_id.short_str(),
                        err
                    );
                },
                Ok(Ok(())) => {
                    info!(
                        log_context,
                        "{} Closed connection to peer: {}",
                        network_context,
                        remote_peer_id.short_str()
                    );
                },
            }
        };
        // the task ends when the write_reqs_tx is dropped
        let multiplex_task = async move {
            let mut outbound_stream =
                OutboundStream::new(max_frame_size, max_message_size, stream_msg_tx);
            while let Some(message) = write_reqs_rx.next().await {
                // either channel full would block the other one
                let result = if outbound_stream.should_stream(&message) {
                    outbound_stream.stream_message(message).await
                } else {
                    msg_tx
                        .send(MultiplexMessage::Message(message))
                        .await
                        .map_err(|_| anyhow::anyhow!("Writer task ended"))
                };
                if let Err(err) = result {
                    warn!(
                        error = %err,
                        "{} Error in sending message to peer: {}",
                        network_context,
                        remote_peer_id.short_str(),
                    );
                }
            }
        };
        executor.spawn(writer_task);
        executor.spawn(multiplex_task);
        (write_reqs_tx, close_tx)
    }

    /// Runs the outbound handler task on a separate tokio thread.
    ///
    /// Handles three types of events:
    /// - Outbound requests from the PeerManager (`peer_reqs_rx`)
    /// - Inbound RPC responses forwarded by the reader task (`rpc_resp_rx`)
    /// - Completed outbound RPC tasks from the completion queue
    async fn run_outbound_handler_task(
        network_context: NetworkContext,
        connection_metadata: ConnectionMetadata,
        mut peer_reqs_rx: aptos_channel::Receiver<ProtocolId, PeerRequest>,
        mut rpc_resp_rx: tokio::sync::mpsc::Receiver<RpcResponse>,
        mut write_reqs_tx: aptos_channel::Sender<(), NetworkMessage>,
        mut outbound_rpcs: OutboundRpcs,
        mut shutdown_rx: watch::Receiver<Option<DisconnectReason>>,
        shutdown_tx: Arc<watch::Sender<Option<DisconnectReason>>>,
    ) {
        loop {
            futures::select! {
                maybe_request = peer_reqs_rx.next() => {
                    match maybe_request {
                        Some(request) => Self::handle_outbound_request_static(
                            &network_context,
                            &connection_metadata,
                            request,
                            &mut write_reqs_tx,
                            &mut outbound_rpcs,
                        ),
                        // The PeerManager is requesting this connection to close
                        // by dropping the corresponding peer_reqs_tx handle.
                        None => {
                            shutdown_tx
                                .send(Some(DisconnectReason::RequestedByPeerManager))
                                .ok();
                            break;
                        },
                    }
                },
                maybe_response = rpc_resp_rx.recv().fuse() => {
                    match maybe_response {
                        Some(response) => outbound_rpcs.handle_inbound_response(response),
                        // Reader task has exited; no more RPC responses will arrive.
                        None => break,
                    }
                },
                (request_id, maybe_completed) = outbound_rpcs.next_completed_request() => {
                    outbound_rpcs.handle_completed_request(request_id, maybe_completed);
                },
                result = shutdown_rx.changed().fuse() => {
                    if result.is_err() || shutdown_rx.borrow().is_some() {
                        break;
                    }
                },
            }
        }
    }

    /// Runs the inbound reader task on a separate tokio thread.
    ///
    /// Reads raw bytes off the wire, applies optional rate limiting, then
    /// dispatches by message type:
    /// - DirectSendMsg → upstream handlers
    /// - Error         → log
    /// - RpcRequest    → inbound RPC task via `inbound_rpc_req_tx`
    /// - RpcResponse   → outbound handler task via `rpc_resp_tx`
    async fn run_inbound_reader_task<R>(
        network_context: NetworkContext,
        connection_metadata: ConnectionMetadata,
        mut reader: R,
        upstream_handlers: Arc<
            HashMap<ProtocolId, aptos_channel::Sender<(PeerId, ProtocolId), ReceivedMessage>>,
        >,
        mut write_reqs_tx: aptos_channel::Sender<(), NetworkMessage>,
        inbound_rpc_req_tx: tokio::sync::mpsc::Sender<ReceivedMessage>,
        rpc_resp_tx: tokio::sync::mpsc::Sender<RpcResponse>,
        mut inbound_stream: InboundStreamBuffer,
        mut inbound_rate_limiter: Option<InboundMessageRateLimiter>,
        mut shutdown_rx: watch::Receiver<Option<DisconnectReason>>,
        shutdown_tx: Arc<watch::Sender<Option<DisconnectReason>>>,
    ) where
        R: FusedStream<Item = Result<MultiplexMessage, ReadError>> + Send + Unpin + 'static,
    {
        let remote_peer_id = connection_metadata.remote_peer_id;
        loop {
            futures::select! {
                // Handle a new inbound MultiplexMessage read off the wire.
                maybe_message = reader.next() => {
                    match maybe_message {
                        Some(message) => {
                            // Apply inbound rate limiting (if required)
                            if let Some(rate_limiter) = &mut inbound_rate_limiter {
                                if let Err(err) = rate_limiter.throttle(&message).await {
                                    warn!(
                                        NetworkSchema::new(&network_context)
                                            .connection_metadata(&connection_metadata),
                                        error = %err,
                                        "{} Error handling inbound message from peer {} during rate limiting! Dropping message!",
                                        network_context,
                                        remote_peer_id.short_str(),
                                    );
                                    continue;
                                }
                            }

                            if let Err(err) = Self::handle_inbound_message_in_reader(
                                &network_context,
                                &connection_metadata,
                                message,
                                &mut write_reqs_tx,
                                &upstream_handlers,
                                &inbound_rpc_req_tx,
                                &rpc_resp_tx,
                                &mut inbound_stream,
                                &shutdown_tx,
                            )
                            .await
                            {
                                warn!(
                                    NetworkSchema::new(&network_context)
                                        .connection_metadata(&connection_metadata),
                                    error = %err,
                                    "{} Error in handling inbound message from peer: {}, error: {}",
                                    network_context,
                                    remote_peer_id.short_str(),
                                    err
                                );
                            }
                        },
                        // The socket was gracefully closed by the remote peer.
                        None => {
                            shutdown_tx
                                .send(Some(DisconnectReason::ConnectionClosed))
                                .ok();
                            break;
                        },
                    }
                },
                result = shutdown_rx.changed().fuse() => {
                    if result.is_err() || shutdown_rx.borrow().is_some() {
                        break;
                    }
                },
            }
        }
    }

    /// Runs the inbound RPC completion task inline on the calling thread.
    ///
    /// Handles two types of events:
    /// - New inbound RPC requests forwarded by the reader task (`inbound_rpc_req_rx`)
    /// - Completed inbound RPC responses ready to be sent back to the remote peer
    async fn run_inbound_rpc_task(
        network_context: NetworkContext,
        connection_metadata: ConnectionMetadata,
        mut inbound_rpc_req_rx: tokio::sync::mpsc::Receiver<ReceivedMessage>,
        mut write_reqs_tx: aptos_channel::Sender<(), NetworkMessage>,
        upstream_handlers: Arc<
            HashMap<ProtocolId, aptos_channel::Sender<(PeerId, ProtocolId), ReceivedMessage>>,
        >,
        mut inbound_rpcs: InboundRpcs,
        mut shutdown_rx: watch::Receiver<Option<DisconnectReason>>,
    ) {
        loop {
            futures::select! {
                maybe_request = inbound_rpc_req_rx.recv().fuse() => {
                    match maybe_request {
                        Some(request) => {
                            let protocol_id = match &request.message {
                                NetworkMessage::RpcRequest(rpc_request) => rpc_request.protocol_id,
                                _ => continue,
                            };
                            match upstream_handlers.get(&protocol_id) {
                                None => {
                                    // Handler disappeared between reader-task check and now; drop request.
                                },
                                Some(handler) => {
                                    if let Err(err) =
                                        inbound_rpcs.handle_inbound_request(handler, request)
                                    {
                                        warn!(
                                            NetworkSchema::new(&network_context)
                                                .connection_metadata(&connection_metadata),
                                            error = %err,
                                            "{} Error handling inbound rpc request: {}",
                                            network_context,
                                            err
                                        );
                                    }
                                },
                            }
                        },
                        // Reader task has exited; no more RPC requests will arrive.
                        None => break,
                    }
                },
                // Drive the queue of pending inbound rpcs. When one is fulfilled
                // by an upstream protocol, send the response to the remote peer.
                maybe_response = inbound_rpcs.next_completed_response() => {
                    let message_metadata = match &maybe_response {
                        Ok((response, protocol_id)) => Some((response.request_id, *protocol_id)),
                        _ => None,
                    };
                    if let Err(error) =
                        inbound_rpcs.send_outbound_response(&mut write_reqs_tx, maybe_response)
                    {
                        let network_schema = NetworkSchema::new(&network_context)
                            .connection_metadata(&connection_metadata);
                        let error_string = format!(
                            "{} Error in handling inbound rpc request (metadata: {:?}), error: {}",
                            network_context, message_metadata, error
                        );
                        match error {
                            RpcError::UnexpectedResponseChannelCancel => {
                                debug!(
                                    network_schema,
                                    error = %error,
                                    "{}", error_string
                                );
                            },
                            error => {
                                warn!(
                                    network_schema,
                                    error = %error,
                                    "{}", error_string
                                );
                            },
                        }
                    }
                },
                result = shutdown_rx.changed().fuse() => {
                    if result.is_err() || shutdown_rx.borrow().is_some() {
                        break;
                    }
                },
            }
        }
    }

    /// Handles a single inbound message in the reader task.
    ///
    /// Parses the `Result<MultiplexMessage, ReadError>`, dispatches to the
    /// appropriate sub-handler, and signals shutdown on unrecoverable I/O errors.
    async fn handle_inbound_message_in_reader(
        network_context: &NetworkContext,
        connection_metadata: &ConnectionMetadata,
        message: Result<MultiplexMessage, ReadError>,
        write_reqs_tx: &mut aptos_channel::Sender<(), NetworkMessage>,
        upstream_handlers: &HashMap<
            ProtocolId,
            aptos_channel::Sender<(PeerId, ProtocolId), ReceivedMessage>,
        >,
        inbound_rpc_req_tx: &tokio::sync::mpsc::Sender<ReceivedMessage>,
        rpc_resp_tx: &tokio::sync::mpsc::Sender<RpcResponse>,
        inbound_stream: &mut InboundStreamBuffer,
        shutdown_tx: &Arc<watch::Sender<Option<DisconnectReason>>>,
    ) -> Result<(), PeerManagerError> {
        trace!(
            NetworkSchema::new(network_context).connection_metadata(connection_metadata),
            "{} Received message from peer {}",
            network_context,
            connection_metadata.remote_peer_id.short_str()
        );

        let message = match message {
            Ok(message) => message,
            Err(err) => match err {
                ReadError::DeserializeError(_, _, ref frame_prefix) => {
                    // DeserializeErrors are recoverable: notify the remote peer and log,
                    // but keep the connection open.
                    let message_type = frame_prefix.as_ref().first().unwrap_or(&0);
                    let protocol_id = frame_prefix.as_ref().get(1).unwrap_or(&0);
                    let error_code = ErrorCode::parsing_error(*message_type, *protocol_id);
                    let message = NetworkMessage::Error(error_code);
                    write_reqs_tx.push((), message)?;
                    return Err(err.into());
                },
                ReadError::IoError(_) => {
                    // IoErrors are mostly unrecoverable; close the connection.
                    shutdown_tx
                        .send(Some(DisconnectReason::InputOutputError))
                        .ok();
                    return Err(err.into());
                },
            },
        };

        match message {
            MultiplexMessage::Message(message) => {
                Self::handle_inbound_network_message_in_reader(
                    network_context,
                    connection_metadata,
                    upstream_handlers,
                    inbound_rpc_req_tx,
                    rpc_resp_tx,
                    message,
                )
                .await
            },
            MultiplexMessage::Stream(message) => {
                Self::handle_inbound_stream_message_in_reader(
                    network_context,
                    connection_metadata,
                    upstream_handlers,
                    inbound_rpc_req_tx,
                    rpc_resp_tx,
                    inbound_stream,
                    message,
                )
                .await
            },
        }
    }

    /// Dispatches a fully-assembled `NetworkMessage` from the reader task.
    async fn handle_inbound_network_message_in_reader(
        network_context: &NetworkContext,
        connection_metadata: &ConnectionMetadata,
        upstream_handlers: &HashMap<
            ProtocolId,
            aptos_channel::Sender<(PeerId, ProtocolId), ReceivedMessage>,
        >,
        inbound_rpc_req_tx: &tokio::sync::mpsc::Sender<ReceivedMessage>,
        rpc_resp_tx: &tokio::sync::mpsc::Sender<RpcResponse>,
        message: NetworkMessage,
    ) -> Result<(), PeerManagerError> {
        let remote_peer_id = connection_metadata.remote_peer_id;
        let network_id = network_context.network_id();
        let peer_network_id = PeerNetworkId::new(network_id, remote_peer_id);

        match message {
            NetworkMessage::DirectSendMsg(direct) => {
                let data_len = direct.raw_msg.len();
                let protocol_id = direct.protocol_id;
                network_application_inbound_traffic(*network_context, protocol_id, data_len as u64);
                let message = NetworkMessage::DirectSendMsg(direct);
                match upstream_handlers.get(&protocol_id) {
                    None => {
                        counters::direct_send_messages(network_context, UNKNOWN_LABEL).inc();
                        counters::direct_send_bytes(network_context, UNKNOWN_LABEL)
                            .inc_by(data_len as u64);
                    },
                    Some(handler) => {
                        let key = (remote_peer_id, protocol_id);
                        match handler.push(key, ReceivedMessage::new(message, peer_network_id)) {
                            Err(_err) => {
                                // NOTE: aptos_channel never returns other than Ok(()), but we might switch to tokio::sync::mpsc and then this would work
                                counters::direct_send_messages(network_context, DECLINED_LABEL)
                                    .inc();
                                counters::direct_send_bytes(network_context, DECLINED_LABEL)
                                    .inc_by(data_len as u64);
                            },
                            Ok(_) => {
                                counters::direct_send_messages(network_context, RECEIVED_LABEL)
                                    .inc();
                                counters::direct_send_bytes(network_context, RECEIVED_LABEL)
                                    .inc_by(data_len as u64);
                            },
                        }
                    },
                }
            },
            NetworkMessage::Error(error_msg) => {
                warn!(
                    NetworkSchema::new(network_context).connection_metadata(connection_metadata),
                    error_msg = ?error_msg,
                    "{} Peer {} sent an error message: {:?}",
                    network_context,
                    remote_peer_id.short_str(),
                    error_msg,
                );
            },
            NetworkMessage::RpcRequest(request) => {
                let protocol_id = request.protocol_id;
                let raw_request_len = request.raw_request.len() as u64;
                if upstream_handlers.contains_key(&protocol_id) {
                    // Forward to the inbound RPC task for processing.
                    let received =
                        ReceivedMessage::new(NetworkMessage::RpcRequest(request), peer_network_id);
                    if inbound_rpc_req_tx.send(received).await.is_err() {
                        warn!(
                            NetworkSchema::new(network_context)
                                .connection_metadata(connection_metadata),
                            "{} Failed to forward inbound RPC request to handler task for peer: {}",
                            network_context,
                            remote_peer_id.short_str(),
                        );
                    }
                } else {
                    counters::direct_send_messages(network_context, UNKNOWN_LABEL).inc();
                    counters::direct_send_bytes(network_context, UNKNOWN_LABEL)
                        .inc_by(raw_request_len);
                }
            },
            NetworkMessage::RpcResponse(response) => {
                // Forward to the outbound handler task which owns OutboundRpcs.
                if rpc_resp_tx.send(response).await.is_err() {
                    warn!(
                        NetworkSchema::new(network_context).connection_metadata(connection_metadata),
                        "{} Failed to forward inbound RPC response to outbound handler task for peer: {}",
                        network_context,
                        remote_peer_id.short_str(),
                    );
                }
            },
        }
        Ok(())
    }

    /// Handles a stream fragment/header in the reader task.
    /// If a stream is completed, the reassembled `NetworkMessage` is dispatched.
    async fn handle_inbound_stream_message_in_reader(
        network_context: &NetworkContext,
        connection_metadata: &ConnectionMetadata,
        upstream_handlers: &HashMap<
            ProtocolId,
            aptos_channel::Sender<(PeerId, ProtocolId), ReceivedMessage>,
        >,
        inbound_rpc_req_tx: &tokio::sync::mpsc::Sender<ReceivedMessage>,
        rpc_resp_tx: &tokio::sync::mpsc::Sender<RpcResponse>,
        inbound_stream: &mut InboundStreamBuffer,
        message: StreamMessage,
    ) -> Result<(), PeerManagerError> {
        match message {
            StreamMessage::Header(header) => {
                inbound_stream.new_stream(header)?;
            },
            StreamMessage::Fragment(fragment) => {
                if let Some(message) = inbound_stream.append_fragment(fragment)? {
                    Self::handle_inbound_network_message_in_reader(
                        network_context,
                        connection_metadata,
                        upstream_handlers,
                        inbound_rpc_req_tx,
                        rpc_resp_tx,
                        message,
                    )
                    .await?;
                }
            },
        }
        Ok(())
    }

    /// Handles a single outbound request from the PeerManager.
    fn handle_outbound_request_static(
        network_context: &NetworkContext,
        connection_metadata: &ConnectionMetadata,
        request: PeerRequest,
        write_reqs_tx: &mut aptos_channel::Sender<(), NetworkMessage>,
        outbound_rpcs: &mut OutboundRpcs,
    ) {
        trace!(
            "Peer {} PeerRequest::{:?}",
            connection_metadata.remote_peer_id.short_str(),
            request
        );
        match request {
            // To send an outbound DirectSendMsg, we just bump some counters and
            // push it onto our outbound writer queue.
            PeerRequest::SendDirectSend(message) => {
                let message_len = message.mdata.len();
                let protocol_id = message.protocol_id;
                let message = NetworkMessage::DirectSendMsg(DirectSendMsg {
                    protocol_id,
                    priority: Priority::default(),
                    raw_msg: Vec::from(message.mdata.as_ref()),
                });

                match write_reqs_tx.push((), message) {
                    Ok(_) => {
                        Self::update_outbound_direct_send_metrics_static(
                            network_context,
                            protocol_id,
                            message_len as u64,
                        );
                    },
                    Err(e) => {
                        counters::direct_send_messages(network_context, FAILED_LABEL).inc();
                        warn!(
                            NetworkSchema::new(network_context)
                                .connection_metadata(connection_metadata),
                            error = ?e,
                            "Failed to send direct send message for protocol {} to peer: {}. Error: {:?}",
                            protocol_id,
                            connection_metadata.remote_peer_id.short_str(),
                            e,
                        );
                    },
                }
            },
            PeerRequest::SendRpc(request) => {
                let protocol_id = request.protocol_id;
                if let Err(e) = outbound_rpcs.handle_outbound_request(request, write_reqs_tx) {
                    sample!(
                        SampleRate::Duration(Duration::from_secs(10)),
                        warn!(
                            NetworkSchema::new(network_context)
                                .connection_metadata(connection_metadata),
                            error = %e,
                            "[sampled] Failed to send outbound rpc request for protocol {} to peer: {}. Error: {}",
                            protocol_id,
                            connection_metadata.remote_peer_id.short_str(),
                            e,
                        )
                    );
                }
            },
        }
    }

    /// Updates the outbound direct send metrics (e.g., messages and bytes sent)
    fn update_outbound_direct_send_metrics_static(
        network_context: &NetworkContext,
        protocol_id: ProtocolId,
        data_len: u64,
    ) {
        counters::direct_send_messages(network_context, SENT_LABEL).inc();
        counters::direct_send_bytes(network_context, SENT_LABEL).inc_by(data_len);
        network_application_outbound_traffic(*network_context, protocol_id, data_len);
    }

    async fn do_shutdown(
        write_req_tx: aptos_channel::Sender<(), NetworkMessage>,
        writer_close_tx: oneshot::Sender<()>,
        reason: DisconnectReason,
        network_context: NetworkContext,
        connection_metadata: ConnectionMetadata,
        mut connection_notifs_tx: aptos_channels::Sender<TransportNotification<TSocket>>,
    ) {
        // Drop the sender to shut down multiplex task.
        drop(write_req_tx);

        // Send a close instruction to the writer task. On receipt of this
        // instruction, the writer task drops all pending outbound messages and
        // closes the connection.
        if let Err(e) = writer_close_tx.send(()) {
            info!(
                NetworkSchema::new(&network_context).connection_metadata(&connection_metadata),
                error = ?e,
                "{} Failed to send close instruction to writer task. It must already be terminating/terminated. Error: {:?}",
                network_context,
                e
            );
        }

        let remote_peer_id = connection_metadata.remote_peer_id;
        // Send a PeerDisconnected event to PeerManager.
        if let Err(e) = connection_notifs_tx
            .send(TransportNotification::Disconnected(
                connection_metadata.clone(),
                reason,
            ))
            .await
        {
            warn!(
                NetworkSchema::new(&network_context).connection_metadata(&connection_metadata),
                error = ?e,
                "{} Failed to notify upstream about disconnection of peer: {}; error: {:?}",
                network_context,
                remote_peer_id.short_str(),
                e
            );
        }

        trace!(
            NetworkSchema::new(&network_context).connection_metadata(&connection_metadata),
            "{} Peer actor for '{}' terminated",
            network_context,
            remote_peer_id.short_str()
        );
    }
}
