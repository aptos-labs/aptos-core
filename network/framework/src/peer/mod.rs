// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

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
    peer_manager::{PeerManagerError, TransportNotification},
    protocols::{
        direct_send::Message,
        network::{ReceivedMessage, SerializedRequest},
        rpc::{error::RpcError, InboundRpcs, OutboundRpcRequest, OutboundRpcs},
        stream::{InboundStreamBuffer, OutboundStream, StreamMessage},
        wire::messaging::v1::{
            metadata::{
                MessageLatencyType, MessageMetadata, MessageReceiveType, MessageSendType,
                MessageStreamType, MultiplexMessageWithMetadata, NetworkMessageWithMetadata,
                ReceivedMessageMetadata, SentMessageMetadata,
            },
            DirectSendAndMetadata, ErrorCode, IncomingRequest, MultiplexMessage,
            MultiplexMessageSink, MultiplexMessageStream, NetworkMessage, ReadError,
            RpcRequestAndMetadata, WriteError,
        },
    },
    transport::{self, Connection, ConnectionMetadata},
    ProtocolId,
};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::network_id::{NetworkContext, NetworkId, PeerNetworkId};
use aptos_logger::prelude::*;
use aptos_short_hex_str::AsShortHexStr;
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_types::PeerId;
use futures::{
    self,
    channel::oneshot,
    io::{AsyncRead, AsyncWrite},
    stream::StreamExt,
    SinkExt,
};
use futures_util::stream::select;
use serde::Serialize;
use std::{
    collections::HashMap,
    fmt, panic,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::{runtime::Handle, time::timeout};
use tokio_util::compat::{
    FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt,
};

#[cfg(test)]
mod test;

#[cfg(test)]
pub mod fuzzing;

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

enum State {
    Connected,
    ShuttingDown(DisconnectReason),
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
    /// Flag to indicate if the actor is being shut down.
    state: State,
    /// The maximum size of an inbound or outbound request frame
    max_frame_size: usize,
    /// The maximum size of an inbound or outbound request message
    max_message_size: usize,
    /// Inbound stream buffer
    inbound_stream: InboundStreamBuffer,
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
    ) -> Self {
        let Connection {
            metadata: connection_metadata,
            socket,
        } = connection;
        let remote_peer_id = connection_metadata.remote_peer_id;
        let max_fragments = max_message_size / max_frame_size;
        Self {
            network_context,
            executor,
            time_service: time_service.clone(),
            connection_metadata,
            connection: Some(socket),
            connection_notifs_tx,
            peer_reqs_rx,
            upstream_handlers,
            inbound_rpcs: InboundRpcs::new(
                network_context,
                time_service.clone(),
                remote_peer_id,
                inbound_rpc_timeout,
                max_concurrent_inbound_rpcs,
            ),
            outbound_rpcs: OutboundRpcs::new(
                network_context,
                time_service,
                remote_peer_id,
                max_concurrent_outbound_rpcs,
            ),
            state: State::Connected,
            max_frame_size,
            max_message_size,
            inbound_stream: InboundStreamBuffer::new(max_fragments),
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

        let mut reader =
            MultiplexMessageStream::new(read_socket.compat(), self.max_frame_size).fuse();
        let writer = MultiplexMessageSink::new(write_socket.compat_write(), self.max_frame_size);

        // Start writer "process" as a separate task. We receive two handles to
        // communicate with the task:
        //   1. `write_reqs_tx`: Queue of pending NetworkMessages to write.
        //   2. `close_tx`: Handle to close the task and underlying connection.
        let (mut write_reqs_tx, writer_close_tx) = Self::start_writer_task(
            &self.executor,
            self.time_service.clone(),
            self.connection_metadata.clone(),
            self.network_context,
            writer,
            self.max_frame_size,
            self.max_message_size,
        );

        // Start main Peer event loop.
        let reason = loop {
            if let State::ShuttingDown(reason) = self.state {
                break reason;
            }

            futures::select! {
                // Handle a new outbound request from the PeerManager.
                maybe_request = self.peer_reqs_rx.next() => {
                    match maybe_request {
                        Some(request) => self.handle_outbound_request(request, &mut write_reqs_tx),
                        // The PeerManager is requesting this connection to close
                        // by dropping the corresponding peer_reqs_tx handle.
                        None => self.shutdown(DisconnectReason::RequestedByPeerManager),
                    }
                },
                // Handle a new inbound MultiplexMessage that we've just read off
                // the wire from the remote peer.
                // TODO: move this to a separate thread!
                maybe_message = reader.next() => {
                    match maybe_message {
                        Some(message) =>  {
                            if let Err(err) = self.handle_inbound_message(message, &mut write_reqs_tx) {
                                warn!(
                                    NetworkSchema::new(&self.network_context)
                                        .connection_metadata(&self.connection_metadata),
                                    error = %err,
                                    "{} Error in handling inbound message from peer: {}, error: {}",
                                    self.network_context,
                                    remote_peer_id.short_str(),
                                    err
                                );
                            }
                        },
                        // The socket was gracefully closed by the remote peer.
                        None => self.shutdown(DisconnectReason::ConnectionClosed),
                    }
                },
                // Drive the queue of pending inbound rpcs. When one is fulfilled
                // by an upstream protocol, send the response to the remote peer.
                maybe_response = self.inbound_rpcs.next_completed_response() => {
                    // Extract the relevant metadata from the message
                    let message_metadata = match &maybe_response {
                        Ok((response_with_metadata, protocol_id)) => Some((response_with_metadata.request_id(), *protocol_id)),
                        _ => None,
                    };

                    // Send the response to the remote peer
                    if let Err(error) = self.inbound_rpcs.send_outbound_response(&mut write_reqs_tx, maybe_response) {
                        // It's quite common for applications to drop an RPC request.
                        // If this happens, we want to avoid logging a warning/error
                        // (as it makes the logs noisy). Otherwise, we log normally.
                        let network_schema = NetworkSchema::new(&self.network_context)
                            .connection_metadata(&self.connection_metadata);
                        let error_string = format!("{} Error in handling inbound rpc request (metadata: {:?}), error: {}", self.network_context,  message_metadata, error);
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
                            }
                        }
                    }
                },
                // Poll the queue of pending outbound rpc tasks for the next
                // successfully or unsuccessfully completed request.
                (request_id, maybe_completed_request) = self.outbound_rpcs.next_completed_request() => {
                    self.outbound_rpcs.handle_completed_request(request_id, maybe_completed_request);
                }
            }
        };

        // Finish shutting down the connection. Close the writer task and notify
        // PeerManager that this connection has shutdown.
        self.do_shutdown(write_reqs_tx, writer_close_tx, reason)
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
        aptos_channel::Sender<(), NetworkMessageWithMetadata>,
        oneshot::Sender<()>,
    ) {
        let remote_peer_id = connection_metadata.remote_peer_id;
        let (write_reqs_tx, mut write_reqs_rx): (
            aptos_channel::Sender<(), NetworkMessageWithMetadata>,
            _,
        ) = aptos_channel::new(
            QueueStyle::KLAST,
            1024,
            Some(&counters::PENDING_WIRE_MESSAGES),
        );
        let (close_tx, mut close_rx) = oneshot::channel();

        let (mut msg_tx, msg_rx) = aptos_channels::new::<MultiplexMessageWithMetadata>(
            1024,
            &counters::PENDING_MULTIPLEX_MESSAGE,
        );
        let (stream_msg_tx, stream_msg_rx) =
            aptos_channels::new(1024, &counters::PENDING_MULTIPLEX_STREAM);

        // this task ends when the multiplex task ends (by dropping the senders) or receiving a close instruction
        let writer_task = async move {
            let mut stream = select(msg_rx, stream_msg_rx);
            let log_context =
                NetworkSchema::new(&network_context).connection_metadata(&connection_metadata);
            loop {
                futures::select! {
                    message_with_metadata = stream.select_next_some() => {
                        // Extract the message and metadata
                        let (message_metadata, mut message) = message_with_metadata.into_parts();
                        let mut sent_message_metadata = match message_metadata.into_sent_metadata() {
                            Some(sent_message_metadata) => sent_message_metadata,
                            None => {
                                error!(
                                    "{} Failed to write message (metadata has the incorrect type)! Expected a sent message!",
                                    network_context,
                                );
                                continue; // Skip ahead to the next event
                            }
                        };

                        // Update the wire send start time for the message and metadata
                        match &mut message {
                            MultiplexMessage::Message(network_message) => {
                                network_message.update_wire_send_time(SystemTime::now());
                            }
                            MultiplexMessage::Stream(_stream_message) => {
                                // TODO: handle stream messages and wire send times!
                            }
                        };
                        sent_message_metadata.update_wire_send_start_time();

                        // Send the message along the wire
                        if let Err(err) = timeout(transport::TRANSPORT_TIMEOUT, writer.send(&message)).await {
                            warn!(
                                log_context,
                                error = %err,
                                "{} Error in sending message to peer: {}",
                                network_context,
                                remote_peer_id.short_str(),
                            );
                        } else {
                            // Otherwise, mark the message as sent along the wire
                            sent_message_metadata.mark_message_as_sent();
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
            while let Some(message_with_metadata) = write_reqs_rx.next().await {
                // either channel full would block the other one
                let result = if outbound_stream.should_stream(&message_with_metadata) {
                    outbound_stream.stream_message(message_with_metadata).await
                } else {
                    // Transform the message into a multiplex message
                    let multiplex_message_with_metadata =
                        message_with_metadata.into_multiplex_message();

                    // Send the message to the writer task
                    msg_tx
                        .send(multiplex_message_with_metadata)
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

    fn handle_inbound_network_message(
        &mut self,
        network_message: NetworkMessage,
        received_message_metadata: ReceivedMessageMetadata,
        streamed_message: bool,
    ) -> Result<(), PeerManagerError> {
        // Update the message transport latency metrics
        let network_id = self.network_context.network_id();
        update_transport_latency_metrics(network_id, &network_message, streamed_message);

        // Process the inbound network message
        match network_message {
            NetworkMessage::DirectSendMsg(message) => {
                let message_and_metadata = message.into_direct_send_and_metadata();
                self.process_inbound_direct_send_message(
                    message_and_metadata,
                    received_message_metadata,
                );
            },
            NetworkMessage::DirectSendAndMetadata(message_and_metadata) => {
                self.process_inbound_direct_send_message(
                    message_and_metadata,
                    received_message_metadata,
                );
            },
            NetworkMessage::RpcRequest(rpc_request) => {
                let request_and_metadata = rpc_request.into_rpc_request_and_metadata();
                self.process_inbound_rpc_request_message(
                    request_and_metadata,
                    received_message_metadata,
                );
            },
            NetworkMessage::RpcRequestAndMetadata(request_and_metadata) => {
                self.process_inbound_rpc_request_message(
                    request_and_metadata,
                    received_message_metadata,
                );
            },
            NetworkMessage::RpcResponse(response) => {
                let response_and_metadata = response.into_rpc_response_and_metadata();
                self.outbound_rpcs.process_inbound_rpc_response_message(
                    response_and_metadata,
                    received_message_metadata,
                );
            },
            NetworkMessage::RpcResponseAndMetadata(response_and_metadata) => {
                self.outbound_rpcs.process_inbound_rpc_response_message(
                    response_and_metadata,
                    received_message_metadata,
                );
            },
            NetworkMessage::Error(error_msg) => {
                warn!(
                    NetworkSchema::new(&self.network_context)
                        .connection_metadata(&self.connection_metadata),
                    error_msg = ?error_msg,
                    "{} Peer {} sent an error message: {:?}",
                    self.network_context,
                    self.remote_peer_id().short_str(),
                    error_msg,
                );
            },
        };

        Ok(())
    }

    /// Processes an inbound direct send message
    fn process_inbound_direct_send_message(
        &mut self,
        message_and_metadata: DirectSendAndMetadata,
        mut received_message_metadata: ReceivedMessageMetadata,
    ) {
        // Update the inbound traffic metrics
        let protocol_id = message_and_metadata.protocol_id();
        let message_length = message_and_metadata.data_length();
        network_application_inbound_traffic(self.network_context, protocol_id, message_length);

        // Attempt to get the handler for the protocol id
        if let Some(handler) = self.upstream_handlers.get(&protocol_id) {
            // Extract the message and context
            let key = (self.connection_metadata.remote_peer_id, protocol_id);
            let remote_peer_id = self.connection_metadata.remote_peer_id;
            let network_id = self.network_context.network_id();
            let sender = PeerNetworkId::new(network_id, remote_peer_id);

            // Update the received message metadata
            received_message_metadata
                .update_protocol_id_and_message_type(protocol_id, MessageReceiveType::DirectSend);

            // Create a new received message and forward it to the handler
            let network_message = NetworkMessage::DirectSendAndMetadata(message_and_metadata);
            let message_metadata =
                MessageMetadata::new_received_metadata(received_message_metadata);
            let received_message = ReceivedMessage::new(network_message, message_metadata, sender);

            // Forward the message to the handler
            match handler.push(key, received_message) {
                Err(_err) => {
                    // NOTE: aptos_channel never returns other than Ok(()), but we might switch
                    // to tokio::sync::mpsc and then this would work
                    counters::direct_send_messages(&self.network_context, DECLINED_LABEL).inc();
                    counters::direct_send_bytes(&self.network_context, DECLINED_LABEL)
                        .inc_by(message_length);
                },
                Ok(_) => {
                    counters::direct_send_messages(&self.network_context, RECEIVED_LABEL).inc();
                    counters::direct_send_bytes(&self.network_context, RECEIVED_LABEL)
                        .inc_by(message_length);
                },
            }
        } else {
            // No handler was found for the protocol ID!
            counters::direct_send_messages(&self.network_context, UNKNOWN_LABEL).inc();
            counters::direct_send_bytes(&self.network_context, UNKNOWN_LABEL)
                .inc_by(message_length);
        }
    }

    /// Processes an inbound RPC request message
    fn process_inbound_rpc_request_message(
        &mut self,
        request_and_metadata: RpcRequestAndMetadata,
        mut received_message_metadata: ReceivedMessageMetadata,
    ) {
        // Extract the protocol ID and message length
        let protocol_id = request_and_metadata.protocol_id();
        let message_length = request_and_metadata.data_length();

        // Attempt to get the handler for the protocol id
        if let Some(handler) = self.upstream_handlers.get(&protocol_id) {
            // Extract the message and context
            let remote_peer_id = self.connection_metadata.remote_peer_id;
            let network_id = self.network_context.network_id();
            let sender = PeerNetworkId::new(network_id, remote_peer_id);

            // Update the received message metadata
            received_message_metadata
                .update_protocol_id_and_message_type(protocol_id, MessageReceiveType::RpcRequest);

            // Create a new received message and forward it to the handler
            let network_message = NetworkMessage::RpcRequestAndMetadata(request_and_metadata);
            let message_metadata =
                MessageMetadata::new_received_metadata(received_message_metadata);
            let received_message = ReceivedMessage::new(network_message, message_metadata, sender);

            // Forward the message to the handler
            if let Err(err) = self
                .inbound_rpcs
                .handle_inbound_request(handler, received_message)
            {
                warn!(
                    NetworkSchema::new(&self.network_context)
                        .connection_metadata(&self.connection_metadata),
                    error = %err,
                    "{} Error handling inbound rpc request: {}",
                    self.network_context,
                    err
                );
            }
        } else {
            // No handler was found for the protocol ID!
            counters::direct_send_messages(&self.network_context, UNKNOWN_LABEL).inc();
            counters::direct_send_bytes(&self.network_context, UNKNOWN_LABEL)
                .inc_by(message_length);
        }
    }

    fn handle_inbound_stream_message(
        &mut self,
        message: StreamMessage,
    ) -> Result<(), PeerManagerError> {
        match message {
            StreamMessage::Header(header) => {
                self.inbound_stream.new_stream(header)?;
            },
            StreamMessage::Fragment(fragment) => {
                if let Some((stream_start_time, network_message)) =
                    self.inbound_stream.append_fragment(fragment)?
                {
                    // Create a new received message metadata
                    let mut received_message_metadata = ReceivedMessageMetadata::new(
                        self.network_context.network_id(),
                        stream_start_time,
                    );

                    // Update the message stream type
                    received_message_metadata
                        .update_message_stream_type(MessageStreamType::StreamedMessageTail);

                    // Handle the message
                    self.handle_inbound_network_message(
                        network_message,
                        received_message_metadata,
                        true,
                    )?;
                }
            },
        }
        Ok(())
    }

    fn handle_inbound_message(
        &mut self,
        message: Result<MultiplexMessage, ReadError>,
        write_reqs_tx: &mut aptos_channel::Sender<(), NetworkMessageWithMetadata>,
    ) -> Result<(), PeerManagerError> {
        trace!(
            NetworkSchema::new(&self.network_context)
                .connection_metadata(&self.connection_metadata),
            "{} Received message from peer {}",
            self.network_context,
            self.remote_peer_id().short_str()
        );

        // Unpack the message result
        let message = match message {
            Ok(message) => message,
            Err(err) => match err {
                ReadError::DeserializeError(_, _, ref frame_prefix) => {
                    // DeserializeError's are recoverable so we'll let the other
                    // peer know about the error and log the issue, but we won't
                    // close the connection.
                    let message_type = frame_prefix.as_ref().first().unwrap_or(&0);
                    let protocol_id = frame_prefix.as_ref().get(1).unwrap_or(&0);
                    let error_code = ErrorCode::parsing_error(*message_type, *protocol_id);
                    let message = NetworkMessage::Error(error_code);
                    let sent_message_metadata = SentMessageMetadata::new(
                        self.network_context.network_id(),
                        None,
                        MessageSendType::DirectSend,
                        Some(SystemTime::now()),
                    );
                    let message_with_metadata = NetworkMessageWithMetadata::new(
                        MessageMetadata::new_sent_metadata(sent_message_metadata),
                        message,
                    );

                    write_reqs_tx.push((), message_with_metadata)?;
                    return Err(err.into());
                },
                ReadError::IoError(_) => {
                    // IoErrors are mostly unrecoverable so just close the connection.
                    self.shutdown(DisconnectReason::InputOutputError);
                    return Err(err.into());
                },
            },
        };

        // Handle the message based on the type
        match message {
            MultiplexMessage::Message(message) => {
                // Create a new received message metadata
                let received_message_metadata = ReceivedMessageMetadata::new(
                    self.network_context.network_id(),
                    SystemTime::now(),
                );

                // Handle the message
                self.handle_inbound_network_message(message, received_message_metadata, false)
            },
            MultiplexMessage::Stream(message) => self.handle_inbound_stream_message(message),
        }
    }

    fn handle_outbound_request(
        &mut self,
        request: PeerRequest,
        write_reqs_tx: &mut aptos_channel::Sender<(), NetworkMessageWithMetadata>,
    ) {
        trace!(
            "Peer {} PeerRequest::{:?}",
            self.remote_peer_id().short_str(),
            request
        );
        match request {
            // To send an outbound DirectSendMsg, we just bump some counters and
            // push it onto our outbound writer queue.
            PeerRequest::SendDirectSend(message) => {
                // Get the data length and protocol id
                let data_len = message.data().len();
                let protocol_id = message.protocol_id();

                // Convert the message into a network message with metadata
                let message_with_metadata = message.into_network_message(
                    self.network_context.network_id(),
                    self.network_context.enable_messages_with_metadata(),
                );

                // Send the message to the outbound writer queue
                match write_reqs_tx.push((), message_with_metadata) {
                    Ok(_) => {
                        self.update_outbound_direct_send_metrics(protocol_id, data_len as u64);
                    },
                    Err(e) => {
                        counters::direct_send_messages(&self.network_context, FAILED_LABEL).inc();
                        warn!(
                            NetworkSchema::new(&self.network_context)
                                .connection_metadata(&self.connection_metadata),
                            error = ?e,
                            "Failed to send direct send message for protocol {} to peer: {}. Error: {:?}",
                            protocol_id,
                            self.remote_peer_id().short_str(),
                            e,
                        );
                    },
                }
            },
            PeerRequest::SendRpc(request) => {
                let protocol_id = request.protocol_id();
                if let Err(e) = self
                    .outbound_rpcs
                    .handle_outbound_request(request, write_reqs_tx)
                {
                    sample!(
                        SampleRate::Duration(Duration::from_secs(10)),
                        warn!(
                            NetworkSchema::new(&self.network_context)
                                .connection_metadata(&self.connection_metadata),
                            error = %e,
                            "[sampled] Failed to send outbound rpc request for protocol {} to peer: {}. Error: {}",
                            protocol_id,
                            self.remote_peer_id().short_str(),
                            e,
                        )
                    );
                }
            },
        }
    }

    /// Updates the outbound direct send metrics (e.g., messages and bytes sent)
    fn update_outbound_direct_send_metrics(&mut self, protocol_id: ProtocolId, data_len: u64) {
        // Update the metrics for the sent direct send message
        counters::direct_send_messages(&self.network_context, SENT_LABEL).inc();
        counters::direct_send_bytes(&self.network_context, SENT_LABEL).inc_by(data_len);

        // Update the general network traffic metrics
        network_application_outbound_traffic(self.network_context, protocol_id, data_len);
    }

    fn shutdown(&mut self, reason: DisconnectReason) {
        // Set the state of the actor to `State::ShuttingDown` to true ensures that the peer actor
        // will terminate and close the connection.
        self.state = State::ShuttingDown(reason);
    }

    async fn do_shutdown(
        mut self,
        write_req_tx: aptos_channel::Sender<(), NetworkMessageWithMetadata>,
        writer_close_tx: oneshot::Sender<()>,
        reason: DisconnectReason,
    ) {
        // Drop the sender to shut down multiplex task.
        drop(write_req_tx);

        // Send a close instruction to the writer task. On receipt of this
        // instruction, the writer task drops all pending outbound messages and
        // closes the connection.
        if let Err(e) = writer_close_tx.send(()) {
            info!(
                NetworkSchema::new(&self.network_context)
                    .connection_metadata(&self.connection_metadata),
                error = ?e,
                "{} Failed to send close instruction to writer task. It must already be terminating/terminated. Error: {:?}",
                self.network_context,
                e
            );
        }

        let remote_peer_id = self.remote_peer_id();
        // Send a PeerDisconnected event to PeerManager.
        if let Err(e) = self
            .connection_notifs_tx
            .send(TransportNotification::Disconnected(
                self.connection_metadata.clone(),
                reason,
            ))
            .await
        {
            warn!(
                NetworkSchema::new(&self.network_context)
                    .connection_metadata(&self.connection_metadata),
                error = ?e,
                "{} Failed to notify upstream about disconnection of peer: {}; error: {:?}",
                self.network_context,
                remote_peer_id.short_str(),
                e
            );
        }

        trace!(
            NetworkSchema::new(&self.network_context)
                .connection_metadata(&self.connection_metadata),
            "{} Peer actor for '{}' terminated",
            self.network_context,
            remote_peer_id.short_str()
        );
    }
}

/// Updates the transport latency metrics for the received message
fn update_transport_latency_metrics(
    network_id: NetworkId,
    message: &NetworkMessage,
    streamed_message: bool,
) {
    // Get the message receive type and metadata
    let (message_receive_type, message_wire_metadata) = match message {
        NetworkMessage::RpcRequestAndMetadata(request_and_metadata) => (
            MessageReceiveType::RpcRequest,
            request_and_metadata.message_wire_metadata(),
        ),
        NetworkMessage::RpcResponseAndMetadata(response_and_metadata) => (
            MessageReceiveType::RpcResponse,
            response_and_metadata.message_wire_metadata(),
        ),
        NetworkMessage::DirectSendAndMetadata(message_and_metadata) => (
            MessageReceiveType::DirectSend,
            message_and_metadata.message_wire_metadata(),
        ),
        _ => return, // There's no message metadata to extract
    };

    // Determine the message stream type
    let message_stream_type = if streamed_message {
        MessageStreamType::StreamedMessageTail
    } else {
        MessageStreamType::NonStreamedMessage
    };

    // Observe the application to receive time
    if let Some(application_send_time) = message_wire_metadata.application_send_time() {
        // Calculate the application to receive time
        let application_to_receive_time = application_send_time
            .elapsed()
            .unwrap_or_default()
            .as_secs_f64();

        // Update the latency metrics
        counters::observe_message_transport_latency(
            &network_id,
            &message_wire_metadata.protocol_id(),
            &message_receive_type,
            &message_stream_type,
            &MessageLatencyType::ApplicationSendToReceive,
            application_to_receive_time,
        );
    }

    // Observe the wire to receive time
    if let Some(wire_send_time) = message_wire_metadata.wire_send_time() {
        // Calculate the wire send to receive time
        let wire_send_to_receive_time = wire_send_time.elapsed().unwrap_or_default().as_secs_f64();

        // Update the latency metrics
        counters::observe_message_transport_latency(
            &network_id,
            &message_wire_metadata.protocol_id(),
            &message_receive_type,
            &message_stream_type,
            &MessageLatencyType::WireSendToReceive,
            wire_send_to_receive_time,
        );
    }
}
