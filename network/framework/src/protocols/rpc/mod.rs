// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Implementation of the unary RPC protocol as per [AptosNet wire protocol v1].
//!
//! ## Design:
//!
//! The unary RPC protocol is implemented here as two independent async completion
//! queues: [`InboundRpcs`] and [`OutboundRpcs`].
//!
//! The `InboundRpcs` queue is responsible for handling inbound rpc requests
//! off-the-wire, forwarding the request to the application layer, waiting for
//! the application layer's response, and then enqueuing the rpc response to-be
//! written over-the-wire.
//!
//! Likewise, the `OutboundRpcs` queue is responsible for handling outbound rpc
//! requests from the application layer, enqueuing the request for writing onto
//! the wire, waiting for a corresponding rpc response, and then notifying the
//! requestor of the arrived response message.
//!
//! Both `InboundRpcs` and `OutboundRpcs` are owned and driven by the [`Peer`]
//! actor. This has a few implications. First, it means that each connection has
//! its own pair of local rpc completion queues; the queues are _not_ shared
//! across connections. Second, the queues don't do any IO work. They're purely
//! driven by the owning `Peer` actor, who calls `handle_` methods on new
//! [`NetworkMessage`] arrivals and polls for completed rpc requests. The queues
//! also do not write to the wire directly; instead, they're given a reference to
//! the [`Peer`] actor's write queue, which they can enqueue a new outbound
//! [`NetworkMessage`] onto.
//!
//! ## Timeouts:
//!
//! Both inbound and outbound requests have mandatory timeouts. The tasks in the
//! async completion queues are each wrapped in a `timeout` future, which causes
//! the task to complete with an error if the task isn't fulfilled before the
//! deadline.
//!
//! ## Limits:
//!
//! We limit the number of pending inbound and outbound RPC tasks to ensure that
//! resource usage is bounded.
//!
//! [AptosNet wire protocol v1]: https://github.com/aptos-labs/aptos-core/blob/main/specifications/network/messaging-v1.md
//! [`Peer`]: crate::peer::Peer

use crate::{
    counters::{
        self, network_application_inbound_traffic, network_application_outbound_traffic,
        CANCELED_LABEL, DECLINED_LABEL, EXPIRED_LABEL, FAILED_LABEL, INBOUND_LABEL, OUTBOUND_LABEL,
        RECEIVED_LABEL, REQUEST_LABEL, RESPONSE_LABEL, SENT_LABEL,
    },
    logging::NetworkSchema,
    protocols::{
        network::{ReceivedMessage, SerializedRequest},
        wire::messaging::v1::{NetworkMessage, Priority, RequestId, RpcRequest, RpcResponse},
    },
    ProtocolId,
};
use anyhow::anyhow;
use aptos_channels::aptos_channel;
use aptos_config::network_id::NetworkContext;
use aptos_id_generator::{IdGenerator, U32IdGenerator};
use aptos_logger::prelude::*;
use aptos_short_hex_str::AsShortHexStr;
use aptos_time_service::{timeout, TimeService, TimeServiceTrait};
use aptos_types::PeerId;
use bytes::Bytes;
use error::RpcError;
use futures::{
    channel::oneshot,
    future::{BoxFuture, FusedFuture, FutureExt},
    stream::{FuturesUnordered, StreamExt},
};
use serde::Serialize;
use std::{
    cmp::PartialEq,
    collections::HashMap,
    fmt::Debug,
    sync::Arc,
    time::{Duration, SystemTime},
};

pub mod error;

/// A wrapper struct for an inbound rpc request and its associated context.
#[derive(Debug)]
pub struct InboundRpcRequest {
    /// The [`ProtocolId`] for which of our upstream application modules should
    /// handle (i.e., deserialize and then respond to) this inbound rpc request.
    ///
    /// For example, if `protocol_id == ProtocolId::ConsensusRpcBcs`, then this
    /// inbound rpc request will be dispatched to consensus for handling.
    pub protocol_id: ProtocolId,
    /// The serialized request data received from the sender. At this layer in
    /// the stack, the request data is just an opaque blob and will only be fully
    /// deserialized later in the handling application module.
    pub data: Bytes,
    /// Channel over which the rpc response is sent from the upper application
    /// layer to the network rpc layer.
    ///
    /// The rpc actor holds onto the receiving end of this channel, awaiting the
    /// response from the upper layer. If there is an error in, e.g.,
    /// deserializing the request, the upper layer should send an [`RpcError`]
    /// down the channel to signify that there was an error while handling this
    /// rpc request. Currently, we just log these errors and drop the request.
    ///
    /// The upper client layer should be prepared for `res_tx` to be disconnected
    /// when trying to send their response, as the rpc call might have timed out
    /// while handling the request.
    pub res_tx: oneshot::Sender<Result<Bytes, RpcError>>,
}

impl SerializedRequest for InboundRpcRequest {
    fn protocol_id(&self) -> ProtocolId {
        self.protocol_id
    }

    fn data(&self) -> &Bytes {
        &self.data
    }
}

/// A wrapper struct for an outbound rpc request and its associated context.
#[derive(Debug, Serialize)]
pub struct OutboundRpcRequest {
    /// The time at which the request was sent by the application
    application_send_time: SystemTime,
    /// The remote peer's application module that should handle our outbound rpc
    /// request.
    ///
    /// For example, if `protocol_id == ProtocolId::ConsensusRpcBcs`, then this
    /// outbound rpc request should be handled by the remote peer's consensus
    /// application module.
    protocol_id: ProtocolId,
    /// The serialized request data to be sent to the receiver. At this layer in
    /// the stack, the request data is just an opaque blob.
    #[serde(skip)]
    data: Bytes,
    /// Channel over which the rpc response is sent from the rpc layer to the
    /// upper client layer.
    ///
    /// If there is an error while performing the rpc protocol, e.g., the remote
    /// peer drops the connection, we will send an [`RpcError`] over the channel.
    #[serde(skip)]
    res_tx: oneshot::Sender<Result<Bytes, RpcError>>,
    /// The timeout duration for the entire rpc call. If the timeout elapses, the
    /// rpc layer will send an [`RpcError::TimedOut`] error over the
    /// `res_tx` channel to the upper client layer.
    timeout: Duration,
}

impl OutboundRpcRequest {
    pub fn new(
        protocol_id: ProtocolId,
        data: Bytes,
        res_tx: oneshot::Sender<Result<Bytes, RpcError>>,
        timeout: Duration,
    ) -> Self {
        Self {
            application_send_time: SystemTime::now(),
            protocol_id,
            data,
            res_tx,
            timeout,
        }
    }

    /// Consumes the request and returns the protocol id, data, channel, and timeout
    pub fn into_parts(
        self,
    ) -> (
        ProtocolId,
        Bytes,
        oneshot::Sender<Result<Bytes, RpcError>>,
        Duration,
    ) {
        (self.protocol_id, self.data, self.res_tx, self.timeout)
    }
}

impl SerializedRequest for OutboundRpcRequest {
    fn protocol_id(&self) -> ProtocolId {
        self.protocol_id
    }

    fn data(&self) -> &Bytes {
        &self.data
    }
}

impl PartialEq for InboundRpcRequest {
    fn eq(&self, other: &Self) -> bool {
        self.protocol_id == other.protocol_id && self.data == other.data
    }
}

/// `InboundRpcs` handles new inbound rpc requests off the wire, notifies the
/// `PeerManager` of the new request, and stores the pending response on a queue.
/// If the response eventually completes, `InboundRpc` records some metrics and
/// enqueues the response message onto the outbound write queue.
///
/// There is one `InboundRpcs` handler per [`Peer`](crate::peer::Peer).
pub struct InboundRpcs {
    /// The network instance this Peer actor is running under.
    network_context: NetworkContext,
    /// A handle to a time service for easily mocking time-related operations.
    time_service: TimeService,
    /// The PeerId of this connection's remote peer. Used for logging.
    remote_peer_id: PeerId,
    /// The core async queue of pending inbound rpc tasks. The tasks are driven
    /// to completion by the `InboundRpcs::next_completed_response()` method.
    inbound_rpc_tasks:
        FuturesUnordered<BoxFuture<'static, Result<(RpcResponse, ProtocolId), RpcError>>>,
    /// A blanket timeout on all inbound rpc requests. If the application handler
    /// doesn't respond to the request before this timeout, the request will be
    /// dropped.
    inbound_rpc_timeout: Duration,
    /// Only allow this many concurrent inbound rpcs at one time from this remote
    /// peer.  New inbound requests exceeding this limit will be dropped.
    max_concurrent_inbound_rpcs: u32,
}

impl InboundRpcs {
    pub fn new(
        network_context: NetworkContext,
        time_service: TimeService,
        remote_peer_id: PeerId,
        inbound_rpc_timeout: Duration,
        max_concurrent_inbound_rpcs: u32,
    ) -> Self {
        Self {
            network_context,
            time_service,
            remote_peer_id,
            inbound_rpc_tasks: FuturesUnordered::new(),
            inbound_rpc_timeout,
            max_concurrent_inbound_rpcs,
        }
    }

    /// Handle a new inbound `RpcRequest` message off the wire.
    pub fn handle_inbound_request(
        &mut self,
        peer_notifs_tx: &aptos_channel::Sender<(PeerId, ProtocolId), ReceivedMessage>,
        mut request: ReceivedMessage,
    ) -> Result<(), RpcError> {
        let network_context = &self.network_context;

        // Drop new inbound requests if our completion queue is at capacity.
        if self.inbound_rpc_tasks.len() as u32 == self.max_concurrent_inbound_rpcs {
            // Increase counter of declined requests
            counters::rpc_messages(
                network_context,
                REQUEST_LABEL,
                INBOUND_LABEL,
                DECLINED_LABEL,
            )
            .inc();
            return Err(RpcError::TooManyPending(self.max_concurrent_inbound_rpcs));
        }

        let peer_id = request.sender.peer_id();
        let NetworkMessage::RpcRequest(rpc_request) = &request.message else {
            return Err(RpcError::InvalidRpcResponse);
        };
        let protocol_id = rpc_request.protocol_id;
        let request_id = rpc_request.request_id;
        let priority = rpc_request.priority;

        trace!(
            NetworkSchema::new(network_context).remote_peer(&self.remote_peer_id),
            "{} Received inbound rpc request from peer {} with request_id {} and protocol_id {}",
            network_context,
            self.remote_peer_id.short_str(),
            request_id,
            protocol_id,
        );
        self.update_inbound_rpc_request_metrics(protocol_id, rpc_request.raw_request.len() as u64);

        let timer =
            counters::inbound_rpc_handler_latency(network_context, protocol_id).start_timer();

        // Forward request to PeerManager for handling.
        let (response_tx, response_rx) = oneshot::channel();
        request.rpc_replier = Some(Arc::new(response_tx));
        if let Err(err) = peer_notifs_tx.push((peer_id, protocol_id), request) {
            counters::rpc_messages(network_context, REQUEST_LABEL, INBOUND_LABEL, FAILED_LABEL)
                .inc();
            return Err(err.into());
        }

        // Create a new task that waits for a response from the upper layer with a timeout.
        let inbound_rpc_task = self
            .time_service
            .timeout(self.inbound_rpc_timeout, response_rx)
            .map(move |result| {
                // Flatten the errors
                let maybe_response = match result {
                    Ok(Ok(Ok(response_bytes))) => {
                        let rpc_response = RpcResponse {
                            request_id,
                            priority,
                            raw_response: Vec::from(response_bytes.as_ref()),
                        };
                        Ok((rpc_response, protocol_id))
                    },
                    Ok(Ok(Err(err))) => Err(err),
                    Ok(Err(oneshot::Canceled)) => Err(RpcError::UnexpectedResponseChannelCancel),
                    Err(timeout::Elapsed) => Err(RpcError::TimedOut),
                };
                // Only record latency of successful requests
                match maybe_response {
                    Ok(_) => timer.stop_and_record(),
                    Err(_) => timer.stop_and_discard(),
                };
                maybe_response
            })
            .boxed();

        // Add that task to the inbound completion queue. These tasks are driven
        // forward by `Peer` awaiting `self.next_completed_response()`.
        self.inbound_rpc_tasks.push(inbound_rpc_task);

        Ok(())
    }

    /// Updates the inbound RPC request metrics (e.g., messages and bytes received)
    fn update_inbound_rpc_request_metrics(&self, protocol_id: ProtocolId, data_len: u64) {
        // Update the metrics for the new RPC request
        counters::rpc_messages(
            &self.network_context,
            REQUEST_LABEL,
            INBOUND_LABEL,
            RECEIVED_LABEL,
        )
        .inc();
        counters::rpc_bytes(
            &self.network_context,
            REQUEST_LABEL,
            INBOUND_LABEL,
            RECEIVED_LABEL,
        )
        .inc_by(data_len);

        // Update the general network traffic metrics
        network_application_inbound_traffic(self.network_context, protocol_id, data_len);
    }

    /// Method for `Peer` actor to drive the pending inbound rpc tasks forward.
    /// The returned `Future` is a `FusedFuture` so it works correctly in a
    /// `futures::select!`.
    pub fn next_completed_response(
        &mut self,
    ) -> impl FusedFuture<Output = Result<(RpcResponse, ProtocolId), RpcError>> + '_ {
        self.inbound_rpc_tasks.select_next_some()
    }

    /// Handle a completed response from the application handler. If successful,
    /// we update the appropriate counters and enqueue the response message onto
    /// the outbound write queue.
    pub fn send_outbound_response(
        &mut self,
        write_reqs_tx: &mut aptos_channel::Sender<(), NetworkMessage>,
        maybe_response: Result<(RpcResponse, ProtocolId), RpcError>,
    ) -> Result<(), RpcError> {
        let network_context = &self.network_context;
        let (response, protocol_id) = match maybe_response {
            Ok(response) => response,
            Err(err) => {
                counters::rpc_messages(
                    network_context,
                    RESPONSE_LABEL,
                    OUTBOUND_LABEL,
                    FAILED_LABEL,
                )
                .inc();
                return Err(err);
            },
        };
        let res_len = response.raw_response.len() as u64;

        // Send outbound response to remote peer.
        trace!(
            NetworkSchema::new(network_context).remote_peer(&self.remote_peer_id),
            "{} Sending rpc response to peer {} for request_id {}",
            network_context,
            self.remote_peer_id.short_str(),
            response.request_id,
        );
        let message = NetworkMessage::RpcResponse(response);
        write_reqs_tx.push((), message)?;

        // Update the outbound RPC response metrics
        self.update_outbound_rpc_response_metrics(protocol_id, res_len);

        Ok(())
    }

    fn update_outbound_rpc_response_metrics(&self, protocol_id: ProtocolId, data_len: u64) {
        // Update the metrics for the new RPC response
        counters::rpc_messages(
            &self.network_context,
            RESPONSE_LABEL,
            OUTBOUND_LABEL,
            SENT_LABEL,
        )
        .inc();
        counters::rpc_bytes(
            &self.network_context,
            RESPONSE_LABEL,
            OUTBOUND_LABEL,
            SENT_LABEL,
        )
        .inc_by(data_len);

        // Update the general network traffic metrics
        network_application_outbound_traffic(self.network_context, protocol_id, data_len);
    }
}

/// `OutboundRpcs` handles new outbound rpc requests made from the application layer.
///
/// There is one `OutboundRpcs` handler per [`Peer`](crate::peer::Peer).
pub struct OutboundRpcs {
    /// The network instance this Peer actor is running under.
    network_context: NetworkContext,
    /// A handle to a time service for easily mocking time-related operations.
    time_service: TimeService,
    /// The PeerId of this connection's remote peer. Used for logging.
    remote_peer_id: PeerId,
    /// Generates the next RequestId to use for the next outbound RPC. Note that
    /// request ids are local to each connection.
    request_id_gen: U32IdGenerator,
    /// A completion queue of pending outbound rpc tasks. Each task waits for
    /// either a successful `RpcResponse` message, handed to it via the channel
    /// in `pending_outbound_rpcs`, or waits for a timeout or cancellation
    /// notification. After completion, the task will yield its `RequestId` and
    /// other metadata (success/failure, success latency, response length) via
    /// the future from `next_completed_request`.
    outbound_rpc_tasks:
        FuturesUnordered<BoxFuture<'static, (RequestId, Result<(f64, u64), RpcError>)>>,
    /// Maps a `RequestId` into a handle to a task in the `outbound_rpc_tasks`
    /// completion queue. When a new `RpcResponse` message comes in, we will use
    /// this map to notify the corresponding task that its response has arrived.
    pending_outbound_rpcs: HashMap<RequestId, (ProtocolId, oneshot::Sender<RpcResponse>)>,
    /// Only allow this many concurrent outbound rpcs at one time from this remote
    /// peer. New outbound requests exceeding this limit will be dropped.
    max_concurrent_outbound_rpcs: u32,
}

impl OutboundRpcs {
    pub fn new(
        network_context: NetworkContext,
        time_service: TimeService,
        remote_peer_id: PeerId,
        max_concurrent_outbound_rpcs: u32,
    ) -> Self {
        Self {
            network_context,
            time_service,
            remote_peer_id,
            request_id_gen: U32IdGenerator::new(),
            outbound_rpc_tasks: FuturesUnordered::new(),
            pending_outbound_rpcs: HashMap::new(),
            max_concurrent_outbound_rpcs,
        }
    }

    /// Handle a new outbound rpc request from the application layer.
    pub fn handle_outbound_request(
        &mut self,
        request: OutboundRpcRequest,
        write_reqs_tx: &mut aptos_channel::Sender<(), NetworkMessage>,
    ) -> Result<(), RpcError> {
        let network_context = &self.network_context;
        let peer_id = &self.remote_peer_id;

        // Unpack request.
        let (protocol_id, request_data, mut application_response_tx, timeout) =
            request.into_parts();
        let req_len = request_data.len() as u64;

        // Drop the outbound request if the application layer has already canceled.
        if application_response_tx.is_canceled() {
            counters::rpc_messages(
                network_context,
                REQUEST_LABEL,
                OUTBOUND_LABEL,
                CANCELED_LABEL,
            )
            .inc();
            return Err(RpcError::UnexpectedResponseChannelCancel);
        }

        // Drop new outbound requests if our completion queue is at capacity.
        if self.outbound_rpc_tasks.len() == self.max_concurrent_outbound_rpcs as usize {
            counters::rpc_messages(
                network_context,
                REQUEST_LABEL,
                OUTBOUND_LABEL,
                DECLINED_LABEL,
            )
            .inc();
            // Notify application that their request was dropped due to capacity.
            let err = Err(RpcError::TooManyPending(self.max_concurrent_outbound_rpcs));
            let _ = application_response_tx.send(err);
            return Err(RpcError::TooManyPending(self.max_concurrent_outbound_rpcs));
        }

        let request_id = self.request_id_gen.next();

        trace!(
            NetworkSchema::new(network_context).remote_peer(peer_id),
            "{} Sending outbound rpc request with request_id {} and protocol_id {} to {}",
            network_context,
            request_id,
            protocol_id,
            peer_id.short_str(),
        );

        // Start timer to collect outbound RPC latency.
        let timer =
            counters::outbound_rpc_request_latency(network_context, protocol_id).start_timer();

        // Enqueue rpc request message onto outbound write queue.
        let message = NetworkMessage::RpcRequest(RpcRequest {
            protocol_id,
            request_id,
            priority: Priority::default(),
            raw_request: Vec::from(request_data.as_ref()),
        });
        write_reqs_tx.push((), message)?;

        // Update the outbound RPC request metrics
        self.update_outbound_rpc_request_metrics(protocol_id, req_len);

        // Create channel over which response is delivered to outbound_rpc_task.
        let (response_tx, response_rx) = oneshot::channel::<RpcResponse>();

        // Store send-side in the pending map so we can notify outbound_rpc_task
        // when the rpc response has arrived.
        self.pending_outbound_rpcs
            .insert(request_id, (protocol_id, response_tx));

        // A future that waits for the rpc response with a timeout. We create the
        // timeout out here to start the timer as soon as we push onto the queue
        // (as opposed to whenever it first gets polled on the queue).
        let wait_for_response = self
            .time_service
            .timeout(timeout, response_rx)
            .map(|result| {
                // Flatten errors.
                match result {
                    Ok(Ok(response)) => Ok(Bytes::from(response.raw_response)),
                    Ok(Err(oneshot::Canceled)) => Err(RpcError::UnexpectedResponseChannelCancel),
                    Err(timeout::Elapsed) => Err(RpcError::TimedOut),
                }
            });

        // A future that waits for the response and sends it to the application.
        let notify_application = async move {
            // This future will complete if the application layer cancels the request.
            let mut cancellation = application_response_tx.cancellation().fuse();
            // Pin the response future to the stack so we don't have to box it.
            tokio::pin!(wait_for_response);

            futures::select! {
                maybe_response = wait_for_response => {
                    // TODO(philiphayes): Clean up RpcError. Effectively need to
                    // clone here to pass the result up to application layer, but
                    // RpcError is not currently cloneable.
                    let result_copy = match &maybe_response {
                        Ok(response) => Ok(response.len() as u64),
                        Err(err) => Err(RpcError::Error(anyhow!(err.to_string()))),
                    };
                    // Notify the application of the results.
                    application_response_tx.send(maybe_response).map_err(|_| RpcError::UnexpectedResponseChannelCancel)?;
                    result_copy
                }
                _ = cancellation => Err(RpcError::UnexpectedResponseChannelCancel),
            }
        };

        let outbound_rpc_task = async move {
            // Always return the request_id so we can garbage collect the
            // pending_outbound_rpcs map.
            match notify_application.await {
                Ok(response_len) => {
                    let latency = timer.stop_and_record();
                    (request_id, Ok((latency, response_len)))
                },
                Err(err) => {
                    // don't record
                    timer.stop_and_discard();
                    (request_id, Err(err))
                },
            }
        };

        self.outbound_rpc_tasks.push(outbound_rpc_task.boxed());
        Ok(())
    }

    /// Updates the outbound RPC request metrics (e.g., messages and bytes sent)
    fn update_outbound_rpc_request_metrics(&mut self, protocol_id: ProtocolId, data_len: u64) {
        // Update the metrics for the new RPC request
        counters::rpc_messages(
            &self.network_context,
            REQUEST_LABEL,
            OUTBOUND_LABEL,
            SENT_LABEL,
        )
        .inc();
        counters::rpc_bytes(
            &self.network_context,
            REQUEST_LABEL,
            OUTBOUND_LABEL,
            SENT_LABEL,
        )
        .inc_by(data_len);

        // Update the general network traffic metrics
        network_application_outbound_traffic(self.network_context, protocol_id, data_len);
    }

    /// Method for `Peer` actor to drive the pending outbound rpc tasks forward.
    /// The returned `Future` is a `FusedFuture` so it works correctly in a
    /// `futures::select!`.
    pub fn next_completed_request(
        &mut self,
    ) -> impl FusedFuture<Output = (RequestId, Result<(f64, u64), RpcError>)> + '_ {
        self.outbound_rpc_tasks.select_next_some()
    }

    /// Handle a newly completed task from the `self.outbound_rpc_tasks` queue.
    /// At this point, the application layer's request has already been fulfilled;
    /// we just need to clean up this request and update some counters.
    pub fn handle_completed_request(
        &mut self,
        request_id: RequestId,
        result: Result<(f64, u64), RpcError>,
    ) {
        // Remove request_id from pending_outbound_rpcs if not already removed.
        //
        // We don't care about the value from `remove` here. If the request
        // timed-out or was canceled, it will still be in the pending map.
        // Otherwise, if we received a response for our request, we will have
        // removed and triggered the oneshot from the pending map, notifying us.
        let _ = self.pending_outbound_rpcs.remove(&request_id);

        let network_context = &self.network_context;
        let peer_id = &self.remote_peer_id;

        match result {
            Ok((latency, request_len)) => {
                counters::rpc_messages(
                    network_context,
                    RESPONSE_LABEL,
                    INBOUND_LABEL,
                    RECEIVED_LABEL,
                )
                .inc();
                counters::rpc_bytes(
                    network_context,
                    RESPONSE_LABEL,
                    INBOUND_LABEL,
                    RECEIVED_LABEL,
                )
                .inc_by(request_len);

                trace!(
                    NetworkSchema::new(network_context).remote_peer(peer_id),
                    "{} Received response for request_id {} from peer {} \
                     with {:.6} seconds of latency",
                    network_context,
                    request_id,
                    peer_id.short_str(),
                    latency,
                );
            },
            Err(error) => {
                if let RpcError::UnexpectedResponseChannelCancel = error {
                    // We don't log when the application has dropped the RPC
                    // response channel because this is often expected (e.g.,
                    // on state sync subscription requests that timeout).
                    counters::rpc_messages(
                        network_context,
                        REQUEST_LABEL,
                        OUTBOUND_LABEL,
                        CANCELED_LABEL,
                    )
                    .inc();
                } else {
                    counters::rpc_messages(
                        network_context,
                        REQUEST_LABEL,
                        OUTBOUND_LABEL,
                        FAILED_LABEL,
                    )
                    .inc();
                    sample!(
                        SampleRate::Duration(Duration::from_secs(10)),
                        warn!(
                            NetworkSchema::new(network_context).remote_peer(peer_id),
                            "[sampled] {} Error making outbound RPC request to {} (request_id {}). Error: {}",
                            network_context,
                            peer_id.short_str(),
                            request_id,
                            error
                        )
                    );
                }
            },
        }
    }

    /// Handle a new inbound `RpcResponse` message. If we have a pending request
    /// with a matching request id in the `pending_outbound_rpcs` map, this will
    /// trigger that corresponding task to wake up and complete in
    /// `handle_completed_request`.
    pub fn handle_inbound_response(&mut self, response: RpcResponse) {
        let network_context = &self.network_context;
        let peer_id = &self.remote_peer_id;
        let request_id = response.request_id;

        let is_canceled = if let Some((protocol_id, response_tx)) =
            self.pending_outbound_rpcs.remove(&request_id)
        {
            self.update_inbound_rpc_response_metrics(
                protocol_id,
                response.raw_response.len() as u64,
            );
            response_tx.send(response).is_err()
        } else {
            true
        };

        if is_canceled {
            debug!(
                NetworkSchema::new(network_context).remote_peer(peer_id),
                request_id = request_id,
                "{} Received response for expired request_id {} from {}. Discarding.",
                network_context,
                request_id,
                peer_id.short_str(),
            );
            counters::rpc_messages(
                network_context,
                RESPONSE_LABEL,
                INBOUND_LABEL,
                EXPIRED_LABEL,
            )
            .inc();
        } else {
            trace!(
                NetworkSchema::new(network_context).remote_peer(peer_id),
                request_id = request_id,
                "{} Notified pending outbound rpc task of inbound response for request_id {} from {}",
                network_context,
                request_id,
                peer_id.short_str(),
            );
        }
    }

    /// Updates the inbound RPC response metrics (e.g., messages and bytes received)
    fn update_inbound_rpc_response_metrics(&self, protocol_id: ProtocolId, data_len: u64) {
        // Update the metrics for the new RPC response
        counters::rpc_messages(
            &self.network_context,
            RESPONSE_LABEL,
            INBOUND_LABEL,
            RECEIVED_LABEL,
        )
        .inc();
        counters::rpc_bytes(
            &self.network_context,
            RESPONSE_LABEL,
            INBOUND_LABEL,
            RECEIVED_LABEL,
        )
        .inc_by(data_len);

        // Update the general network traffic metrics
        network_application_inbound_traffic(self.network_context, protocol_id, data_len);
    }
}
