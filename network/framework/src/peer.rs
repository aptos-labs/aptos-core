// Copyright © Aptos Foundation

use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::sync::mpsc::Receiver;
use crate::protocols::wire::messaging::v1::{ErrorCode, MultiplexMessage, MultiplexMessageSink, MultiplexMessageStream, NetworkMessage, STREAM_FRAGMENT_OVERHEAD_BYTES, STREAM_HEADER_OVERHEAD_BYTES};
use futures::io::{AsyncRead,AsyncReadExt,AsyncWrite};
use futures::StreamExt;
use futures::SinkExt;
use futures::stream::Fuse;
use tokio::sync::mpsc::error::TryRecvError;
use aptos_config::config::{NetworkConfig, RoleType};
use aptos_config::network_id::{NetworkContext, NetworkId, PeerNetworkId};
use aptos_logger::{error, info, warn};
use aptos_metrics_core::{IntCounter, IntCounterVec, register_int_counter_vec};
use crate::application::ApplicationCollector;
use crate::application::interface::{OpenRpcRequestState, OutboundRpcMatcher};
use crate::application::storage::PeersAndMetadata;
use crate::ProtocolId;
use crate::protocols::network::{Closer, OutboundPeerConnections, PeerStub, ReceivedMessage};
use crate::protocols::stream::{StreamFragment, StreamHeader, StreamMessage};
use crate::transport::ConnectionMetadata;
use once_cell::sync::Lazy;
use crate::counters;

pub fn start_peer<TSocket>(
    config: &NetworkConfig,
    socket: TSocket,
    connection_metadata: ConnectionMetadata,
    apps: Arc<ApplicationCollector>,
    handle: Handle,
    remote_peer_network_id: PeerNetworkId,
    peers_and_metadata: Arc<PeersAndMetadata>,
    peer_senders: Arc<OutboundPeerConnections>,
    network_context: NetworkContext,
)
where
    TSocket: crate::transport::TSocket
{
    let role_type = network_context.role();
    let (sender, to_send) = tokio::sync::mpsc::channel::<(NetworkMessage,u64)>(config.network_channel_size);
    let (sender_high_prio, to_send_high_prio) = tokio::sync::mpsc::channel::<(NetworkMessage,u64)>(config.network_channel_size);
    let open_outbound_rpc = OutboundRpcMatcher::new();
    let max_frame_size = config.max_frame_size;
    let (read_socket, write_socket) = socket.split();
    let reader =
        MultiplexMessageStream::new(read_socket, max_frame_size).fuse();
    let writer = MultiplexMessageSink::new(write_socket, max_frame_size);
    let closed = Closer::new();
    let network_id = remote_peer_network_id.network_id();
    handle.spawn(open_outbound_rpc.clone().cleanup(Duration::from_millis(100), closed.clone()));
    handle.spawn(writer_task(network_id, to_send, to_send_high_prio, writer, max_frame_size, role_type, closed.clone()));
    handle.spawn(reader_task(reader, apps, remote_peer_network_id, open_outbound_rpc.clone(), handle.clone(), closed.clone(), role_type));
    let stub = PeerStub::new(sender, sender_high_prio, open_outbound_rpc, closed.clone());
    if let Err(err) = peers_and_metadata.insert_connection_metadata(remote_peer_network_id, connection_metadata.clone()) {
        error!("start_peer PeersAndMetadata could not insert metadata: {:?}", err);
    }
    peer_senders.insert(remote_peer_network_id, stub);
    handle.spawn(peer_cleanup_task(remote_peer_network_id, connection_metadata, closed, peers_and_metadata, peer_senders));
}

/// state needed in writer_task()
struct WriterContext<WriteThing: AsyncWrite + Unpin + Send> {
    network_id: NetworkId,
    /// increment for each new fragment stream
    stream_request_id : u32,
    /// remaining payload bytes of curretnly fragmenting large message
    large_message: Option<Vec<u8>>,
    /// index into chain of fragments
    large_fragment_id: u8,
    /// toggle to send normal msg or send fragment of large message
    send_large: bool,
    /// if we have a large message in flight and another arrives, stash it here
    next_large_msg: Option<NetworkMessage>,
    /// messages above this size get broken into a series of chunks interleaved with other messages
    max_frame_size: usize,
    /// RoleType for metrics
    role_type: RoleType,

    /// messages from apps to send to the peer
    to_send: Receiver<(NetworkMessage,u64)>,
    to_send_high_prio: Receiver<(NetworkMessage,u64)>,
    /// encoder wrapper around socket write half
    writer: MultiplexMessageSink<WriteThing>,
}

impl<WriteThing: AsyncWrite + Unpin + Send> WriterContext<WriteThing> {
    fn new(
        network_id: NetworkId,
        to_send: Receiver<(NetworkMessage,u64)>,
        to_send_high_prio: Receiver<(NetworkMessage,u64)>,
        writer: MultiplexMessageSink<WriteThing>,
        max_frame_size: usize,
        role_type: RoleType,
    ) -> Self {
        Self {
            network_id,
            stream_request_id: 0,
            large_message: None,
            large_fragment_id: 0,
            send_large: false,
            next_large_msg: None,
            max_frame_size,
            role_type,
            to_send,
            to_send_high_prio,
            writer,
        }
    }

    /// send a next chunk from a currently fragmenting large message
    fn next_large(&mut self) -> MultiplexMessage {
        let fragment_payload_size = self.max_frame_size - STREAM_FRAGMENT_OVERHEAD_BYTES;
        let mut blob = self.large_message.take().unwrap();
        if blob.len() > fragment_payload_size {
            let rest = blob.split_off(fragment_payload_size);
            self.large_message = Some(rest);
        }
        self.large_fragment_id += 1;
        self.send_large = false;
        MultiplexMessage::Stream(StreamMessage::Fragment(StreamFragment {
            request_id: self.stream_request_id,
            fragment_id: self.large_fragment_id,
            raw_data: blob,
        }))
    }

    fn start_large(&mut self, msg: NetworkMessage) -> MultiplexMessage {
        peer_message_large_msg_fragmented(&self.network_id, msg.protocol_id_as_str()).inc();
        self.stream_request_id += 1;
        self.send_large = false;
        self.large_fragment_id = 0;
        let header_payload_size = self.max_frame_size - STREAM_HEADER_OVERHEAD_BYTES - msg.header_len();
        let fragment_payload_size = self.max_frame_size - STREAM_FRAGMENT_OVERHEAD_BYTES;
        // let serialized_len = estimate_serialized_length(&msg);
        let payload_len = msg.data_len();
        let fragments_len = payload_len - header_payload_size;
        let mut num_fragments = (fragments_len / fragment_payload_size) + 1;
        let mut msg = msg;
        while (num_fragments - 1) * fragment_payload_size < fragments_len {
            num_fragments += 1;
        }
        if num_fragments > 0x0ff {
            panic!("huge message cannot be fragmented {:?} > 255 * {:?}", msg.data_len(), self.max_frame_size);
        }
        info!("start large: payload len {:?}, num_frag {:?}", payload_len, num_fragments);
        let num_fragments = num_fragments as u8;
        let rest = match &mut msg {
            NetworkMessage::Error(_) => {
                unreachable!("NetworkMessage::Error should always fit in a single frame")
            },
            NetworkMessage::RpcRequest(request) => {
                request.raw_request.split_off(header_payload_size)
            },
            NetworkMessage::RpcResponse(response) => {
                response.raw_response.split_off(header_payload_size)
            },
            NetworkMessage::DirectSendMsg(message) => {
                message.raw_msg.split_off(header_payload_size)
            },
        };
        self.large_message = Some(rest);
        MultiplexMessage::Stream(StreamMessage::Header(StreamHeader {
            request_id: self.stream_request_id,
            num_fragments,
            message: msg,
        }))
    }

    fn try_high_prio_next_msg(&mut self) -> Option<MultiplexMessage> {
        match self.to_send_high_prio.try_recv() {
            Ok((msg, enqueue_micros)) => {
                // info!("writer_thread to_send_high_prio {} bytes prot={}", msg.data_len(), msg.protocol_id_as_str());
                counters::network_application_outbound_traffic(self.role_type.as_str(), self.network_id.as_str(), msg.protocol_id_as_str(), msg.data_len() as u64);
                let queue_micros = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as u64) - enqueue_micros;
                counters::network_peer_outbound_queue_time(self.role_type.as_str(), self.network_id.as_str(), msg.protocol_id_as_str(), queue_micros);
                let serialized_len = estimate_serialized_length(&msg);
                info!("next high prio ser len {:?}", serialized_len);
                if serialized_len > self.max_frame_size {
                    // finish prior large message before starting a new large message
                    if self.large_message.is_some() {
                        self.next_large_msg = Some(msg);
                        Some(self.next_large())
                    } else {
                        Some(self.start_large(msg))
                    }
                } else {
                    // send small message now, large chunk next
                    self.send_large = true;
                    Some(MultiplexMessage::Message(msg))
                }
            }
            Err(_) => {
                None
            }
        }
    }

    fn try_next_msg(&mut self) -> Option<MultiplexMessage> {
        if let Some(mm) = self.try_high_prio_next_msg() {
            return Some(mm);
        }
        match self.to_send.try_recv() {
            Ok((msg,enqueue_micros)) => {
                counters::network_application_outbound_traffic(self.role_type.as_str(), self.network_id.as_str(), msg.protocol_id_as_str(), msg.data_len() as u64);
                let queue_micros = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as u64) - enqueue_micros;
                counters::network_peer_outbound_queue_time(self.role_type.as_str(), self.network_id.as_str(), msg.protocol_id_as_str(), queue_micros);
                let serialized_len = estimate_serialized_length(&msg);
                info!("next ser len {:?}", serialized_len);
                if serialized_len > self.max_frame_size { // TODO: FIXME, serialize msg to find out if it is bigger than max_frame_size?
                    // finish prior large message before starting a new large message
                    if self.large_message.is_some() {
                        self.next_large_msg = Some(msg);
                        Some(self.next_large())
                    } else {
                        Some(self.start_large(msg))
                    }
                } else {
                    // send small message now, large chunk next
                    self.send_large = true;
                    Some(MultiplexMessage::Message(msg))
                }
            }
            Err(err) => match err {
                TryRecvError::Empty => {
                    // ok, no next small msg, continue with chunks of large message
                    if self.large_message.is_some() {
                        Some(self.next_large())
                    } else {
                        None
                    }
                }
                TryRecvError::Disconnected => {
                    info!("writer_thread source closed");
                    None
                }
            }
        }
    }

    async fn run(mut self, mut closed: Closer) {
        let close_reason;
        loop {
            let mm = if self.large_message.is_some() {
                if self.send_large || self.next_large_msg.is_some() {
                    self.next_large()
                } else {
                    match self.try_next_msg() {
                        None => {
                            close_reason = "try_next_msg None";
                            error!("try_next_msg None where it should be Some");
                            break;
                        }
                        Some(mm) => {mm}
                    }
                }
            } else if self.next_large_msg.is_some() {
                let msg = self.next_large_msg.take().unwrap();
                self.start_large(msg)
            } else {
                // try high-prio, otherwise wait on whatever is available next
                if let Some(mm) = self.try_high_prio_next_msg() {
                    mm
                } else {
                    tokio::select! {
                        high_prio = self.to_send_high_prio.recv() => match high_prio {
                            None => {
                                close_reason = "writer_thread high prio closed";
                                info!("writer_thread high prio closed");
                                break;
                            },
                            Some((msg, enqueue_micros)) => {
                                counters::network_application_outbound_traffic(self.role_type.as_str(), self.network_id.as_str(), msg.protocol_id_as_str(), msg.data_len() as u64);
                                let queue_micros = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as u64) - enqueue_micros;
                                counters::network_peer_outbound_queue_time(self.role_type.as_str(), self.network_id.as_str(), msg.protocol_id_as_str(), queue_micros);
                                info!("selected high prio for {:?} bytes", estimate_serialized_length(&msg));
                                if estimate_serialized_length(&msg) > self.max_frame_size {
                                    // start stream
                                    self.start_large(msg)
                                } else {
                                    MultiplexMessage::Message(msg)
                                }
                            }
                        },
                        send_result = self.to_send.recv() => match send_result {
                            None => {
                                close_reason = "writer_thread source closed";
                                info!("writer_thread source closed");
                                break;
                            },
                            Some((msg, enqueue_micros)) => {
                                counters::network_application_outbound_traffic(self.role_type.as_str(), self.network_id.as_str(), msg.protocol_id_as_str(), msg.data_len() as u64);
                                let queue_micros = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as u64) - enqueue_micros;
                                counters::network_peer_outbound_queue_time(self.role_type.as_str(), self.network_id.as_str(), msg.protocol_id_as_str(), queue_micros);
                                info!("selected normal for {:?} bytes", estimate_serialized_length(&msg));
                                if estimate_serialized_length(&msg) > self.max_frame_size {
                                    // start stream
                                    self.start_large(msg)
                                } else {
                                    MultiplexMessage::Message(msg)
                                }
                            },
                        },
                        // TODO: why does select on close.wait() work below but I did this workaround here?
                        wait_result = closed.done.wait_for(|x| *x) => {
                            close_reason = "closed done";
                            info!("writer_thread wait result {:?}", wait_result);
                            break;
                        },
                    }
                }
            };
            if let MultiplexMessage::Message(NetworkMessage::Error(ErrorCode::DisconnectCommand)) = &mm {
                // TODO: clean away "peerclose" logging
                info!(
                    op = "writer_thread got DisconnectCommand",
                    "peerclose"
                );
                close_reason = "got DisconnectCommand";
                break;
            }
            let data_len = mm.data_len();
            tokio::select! {
                send_result = self.writer.send(&mm) => match send_result {
                    Ok(_) => {
                        info!("writer_thread ok sent {:?} bytes", data_len);
                        peer_message_frames_written(&self.network_id).inc();
                        peer_message_bytes_written(&self.network_id).inc_by(data_len as u64);
                    }
                    Err(err) => {
                        // TODO: counter net write err
                        close_reason = "send error";
                        warn!("writer_thread error sending [{:?}]message to peer: {:?} mm={:?}", data_len, err, mm);
                        break;
                    }
                },
                _ = closed.wait() => {
                    close_reason = "closed wait";
                    info!(
                        op = "writer_thread peer writer got closed",
                        "peerclose"
                    );
                    break;
                }
            }
        }
        info!(
            reason = close_reason,
            op = "writer_thread closing",
            "peerclose"
        );
        closed.close().await;
    }
}

pub static NETWORK_PEER_MESSAGE_FRAMES_WRITTEN: Lazy<IntCounterVec> = Lazy::new(||
    register_int_counter_vec!(
    "aptos_network_frames_written",
    "Number of messages written to MultiplexMessageSink",
    &["network_id"]
).unwrap()
);
pub fn peer_message_frames_written(network_id: &NetworkId) -> IntCounter {
    NETWORK_PEER_MESSAGE_FRAMES_WRITTEN.with_label_values(&[network_id.as_str()])
}

pub static NETWORK_PEER_MESSAGE_BYTES_WRITTEN: Lazy<IntCounterVec> = Lazy::new(||
    register_int_counter_vec!(
    "aptos_network_bytes_written",
    "Number of bytes written to MultiplexMessageSink",
    &["network_id"]
).unwrap()
);
pub fn peer_message_bytes_written(network_id: &NetworkId) -> IntCounter {
    NETWORK_PEER_MESSAGE_BYTES_WRITTEN.with_label_values(&[network_id.as_str()])
}

pub static NETWORK_PEER_MESSAGES_FRAGMENTED: Lazy<IntCounterVec> = Lazy::new(||
    register_int_counter_vec!(
    "aptos_network_messages_fragmented",
    "Number of large messages broken up into segments",
    &["network_id","protocol_id"]
).unwrap()
);
pub fn peer_message_large_msg_fragmented(network_id: &NetworkId, protocol_id: &'static str) -> IntCounter {
    NETWORK_PEER_MESSAGES_FRAGMENTED.with_label_values(&[network_id.as_str(), protocol_id])
}

async fn writer_task(
    network_id: NetworkId,
    to_send: Receiver<(NetworkMessage,u64)>,
    to_send_high_prio: Receiver<(NetworkMessage,u64)>,
    writer: MultiplexMessageSink<impl AsyncWrite + Unpin + Send + 'static>,
    max_frame_size: usize,
    role_type: RoleType,
    closed: Closer,
) {
    let wt = WriterContext::new(network_id, to_send, to_send_high_prio, writer, max_frame_size, role_type);
    wt.run(closed).await;
    info!("peer writer exited")
}

async fn complete_rpc(rpc_state: OpenRpcRequestState, nmsg: NetworkMessage, rx_time: u64) {
    if let NetworkMessage::RpcResponse(response) = nmsg {
        let blob = response.raw_response;
        let now = tokio::time::Instant::now(); // TODO: use a TimeService
        let dt = now.duration_since(rpc_state.started);
        let data_len = blob.len() as u64;
        match rpc_state.sender.send(Ok((blob.into(), rx_time))) {
            Ok(_) => {
                counters::rpc_message_bytes(rpc_state.network_id, rpc_state.protocol_id.as_str(), rpc_state.role_type, counters::RESPONSE_LABEL, counters::INBOUND_LABEL, counters::RECEIVED_LABEL, data_len);
                counters::outbound_rpc_request_latency(rpc_state.role_type, rpc_state.network_id, rpc_state.protocol_id).observe(dt.as_secs_f64());
            }
            Err(_) => {
                counters::rpc_message_bytes(rpc_state.network_id, rpc_state.protocol_id.as_str(), rpc_state.role_type, counters::RESPONSE_LABEL, counters::INBOUND_LABEL, "declined", data_len);
            }
        }
    } else {
        unreachable!("read_thread complete_rpc called on other than NetworkMessage::RpcResponse")
    }
}

struct ReaderContext<ReadThing: AsyncRead + Unpin + Send> {
    reader: Fuse<MultiplexMessageStream<ReadThing>>,
    apps: Arc<ApplicationCollector>,
    remote_peer_network_id: PeerNetworkId,
    open_outbound_rpc: OutboundRpcMatcher,
    handle: Handle,
    role_type: RoleType, // for metrics

    // defragment context
    current_stream_id : u32,
    large_message : Option<NetworkMessage>,
    fragment_index : u8,
    num_fragments : u8,
}

impl<ReadThing: AsyncRead + Unpin + Send> ReaderContext<ReadThing> {
    fn new(
        reader: Fuse<MultiplexMessageStream<ReadThing>>,
        apps: Arc<ApplicationCollector>,
        remote_peer_network_id: PeerNetworkId,
        open_outbound_rpc: OutboundRpcMatcher,
        handle: Handle,
        role_type: RoleType,
    ) -> Self {
        Self {
            reader,
            apps,
            remote_peer_network_id,
            open_outbound_rpc,
            handle,
            role_type,

            current_stream_id: 0,
            large_message: None,
            fragment_index: 0,
            num_fragments: 0,
        }
    }

    async fn forward(&self, protocol_id: ProtocolId, nmsg: NetworkMessage) {
        match self.apps.get(&protocol_id) {
            None => {
                // TODO: counter
                error!("read_thread got rpc req for protocol {:?} we don't handle", protocol_id);
                println!("read_thread got rpc req for protocol {:?} we don't handle", protocol_id);
                // TODO: drop connection
            }
            Some(app) => {
                if app.protocol_id != protocol_id {
                    for (xpi, ac) in self.apps.iter() {
                        error!("read_thread app err {} -> {} {} {:?}", xpi.as_str(), ac.protocol_id, ac.label, &ac.sender);
                    }
                    unreachable!("read_thread apps[{}] => {} {:?}", protocol_id, app.protocol_id, &app.sender);
                }
                let data_len = nmsg.data_len() as u64;
                match app.sender.try_send(ReceivedMessage::new(nmsg, self.remote_peer_network_id)) {
                    Ok(_) => {
                        println!("forward {:?}", &protocol_id);
                        peer_read_message_bytes(&self.remote_peer_network_id.network_id(), &protocol_id, data_len);
                    }
                    Err(_) => {
                        println!("forward {:?} ERROR DROP", &protocol_id);
                        app_inbound_drop(&self.remote_peer_network_id.network_id(), &protocol_id, data_len);
                    }
                }
            }
        }
    }

    async fn handle_message(&self, nmsg: NetworkMessage) {
        counters::network_application_inbound_traffic(self.role_type.as_str(), self.remote_peer_network_id.network_id().as_str(), nmsg.protocol_id_as_str(), nmsg.data_len() as u64);
        match &nmsg {
            NetworkMessage::Error(errm) => {
                // TODO: counter
                warn!("read_thread got error message: {:?}", errm)
            }
            NetworkMessage::RpcRequest(request) => {
                let protocol_id = request.protocol_id;
                let data_len = request.raw_request.len() as u64;
                counters::rpc_message_bytes(self.remote_peer_network_id.network_id(), protocol_id.as_str(), self.role_type, counters::REQUEST_LABEL, counters::INBOUND_LABEL, counters::RECEIVED_LABEL, data_len);
                if protocol_id == ProtocolId::StorageServiceRpc {
                    info!(
                        req_id = request.request_id,
                        peer = self.remote_peer_network_id.peer_id(),
                        protocol_id = protocol_id.as_str(),
                        "RPCT req in");
                }
                self.forward(protocol_id, nmsg).await;
            }
            NetworkMessage::RpcResponse(response) => {
                match self.open_outbound_rpc.remove(&response.request_id) {
                    None => {
                        let data_len = response.raw_response.len() as u64;
                        counters::rpc_message_bytes(self.remote_peer_network_id.network_id(), "unk", self.role_type, counters::RESPONSE_LABEL, counters::INBOUND_LABEL, "miss", data_len);
                        info!(
                            req_id = response.request_id,
                            peer = self.remote_peer_network_id.peer_id(),
                            protocol_id = "DED",
                            "RPCT rsp in");
                    }
                    Some(rpc_state) => {
                        let rx_time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as u64;
                        if rpc_state.protocol_id == ProtocolId::StorageServiceRpc {
                            info!(
                                peer = self.remote_peer_network_id.peer_id(),
                                req_id = response.request_id,
                                protocol_id = rpc_state.protocol_id.as_str(),
                                "RPCT rsp in");
                        }
                        self.handle.spawn(complete_rpc(rpc_state, nmsg, rx_time));
                    }
                }
            }
            NetworkMessage::DirectSendMsg(message) => {
                let protocol_id = message.protocol_id;
                let data_len = message.raw_msg.len() as u64;
                counters::direct_send_message_bytes(self.remote_peer_network_id.network_id(), protocol_id.as_str(), self.role_type, counters::RECEIVED_LABEL, data_len);
                self.forward(protocol_id, nmsg).await;
            }
        }
    }

    async fn handle_stream(&mut self, fragment: StreamMessage) {
        match fragment {
            StreamMessage::Header(head) => {
                if self.num_fragments != self.fragment_index {
                    warn!("fragment index = {:?} of {:?} total fragments with new stream header", self.fragment_index, self.num_fragments);
                }
                info!("read_thread shed id={}, {}b {}", head.request_id, head.message.data_len(), head.message.protocol_id_as_str());
                self.current_stream_id = head.request_id;
                self.num_fragments = head.num_fragments;
                self.large_message = Some(head.message);
                self.fragment_index = 1;
            }
            StreamMessage::Fragment(more) => {
                if more.request_id != self.current_stream_id {
                    warn!("got stream request_id={:?} while {:?} was in progress", more.request_id, self.current_stream_id);
                    // TODO: counter? disconnect from peer?
                    self.num_fragments = 0;
                    self.fragment_index = 0;
                    return;
                }
                if more.fragment_id != self.fragment_index {
                    warn!("got fragment_id {:?}, expected {:?}", more.fragment_id, self.fragment_index);
                    // TODO: counter? disconnect from peer?
                    self.num_fragments = 0;
                    self.fragment_index = 0;
                    return;
                }

                match self.large_message.as_mut() {
                    None => {
                        warn!("got fragment without header");
                        return;
                    }
                    Some(lm) => match lm {
                        NetworkMessage::Error(_) => {
                            unreachable!("stream fragment should never be NetworkMessage::Error")
                        }
                        NetworkMessage::RpcRequest(request) => {
                            request.raw_request.extend_from_slice(more.raw_data.as_slice());
                        }
                        NetworkMessage::RpcResponse(response) => {
                            response.raw_response.extend_from_slice(more.raw_data.as_slice());
                        }
                        NetworkMessage::DirectSendMsg(message) => {
                            message.raw_msg.extend_from_slice(more.raw_data.as_slice());
                        }
                    }
                }
                self.fragment_index += 1;
                if self.fragment_index == self.num_fragments {
                    info!("read_thread more id={}, {}b done", more.request_id, more.raw_data.len());
                    let large_message = self.large_message.take().unwrap();
                    self.handle_message(large_message).await;
                } else {
                    info!("read_thread more id={}, {}b", more.request_id, more.raw_data.len());
                }
            }
        }
    }

    async fn run(mut self, mut closed: Closer) {
        info!("read_thread start");
        println!("read_thread start");
        let close_reason;
        loop {
            let rrmm = tokio::select! {
                rrmm = self.reader.next() => {rrmm},
                _ = closed.done.wait_for(|x| *x) => {
                    info!(
                        op = "ReadContext::run ext",
                        reason = "closed done",
                        peer = self.remote_peer_network_id,
                        "peerclose"
                    );
                    println!("reader peerclose");
                    return;
                },
            };
            println!("reader rmm");
            match rrmm {
                Some(rmm) => match rmm {
                    Ok(msg) => match msg {
                        MultiplexMessage::Message(nmsg) => {
                            println!("read msg");
                            self.handle_message(nmsg).await;
                        }
                        MultiplexMessage::Stream(fragment) => {
                            println!("read frag");
                            self.handle_stream(fragment).await;
                        }
                    }
                    Err(err) => {
                        println!("read_thread {} err {}", self.remote_peer_network_id, err);
                        info!("read_thread {} err {}", self.remote_peer_network_id, err);
                        // Error, but not a close-worthy error?
                    }
                }
                None => {
                    println!("read_thread {} None", self.remote_peer_network_id);
                    info!("read_thread {} None", self.remote_peer_network_id);
                    close_reason = "reader next none";
                    break;
                }
            };
        }

        info!(
            op = "ReadContext::run ext",
            reason = close_reason,
            "peerclose"
        );
        closed.close().await;
    }
}

async fn reader_task(
    reader: Fuse<MultiplexMessageStream<impl AsyncRead + Unpin + Send>>,
    apps: Arc<ApplicationCollector>,
    remote_peer_network_id: PeerNetworkId,
    open_outbound_rpc: OutboundRpcMatcher,
    handle: Handle,
    closed: Closer,
    role_type: RoleType,
) {
    let rc = ReaderContext::new(reader, apps, remote_peer_network_id, open_outbound_rpc, handle, role_type);
    rc.run(closed).await;
    info!("peer {} reader finished", remote_peer_network_id);
}

async fn peer_cleanup_task(
    remote_peer_network_id: PeerNetworkId,
    connection_metadata: ConnectionMetadata,
    mut closed: Closer,
    peers_and_metadata: Arc<PeersAndMetadata>,
    peer_senders: Arc<OutboundPeerConnections>,
) {
    closed.wait().await;
    info!(
        peer = remote_peer_network_id,
        op = "cleanup",
        "peerclose"
    );
    peer_senders.remove(&remote_peer_network_id);
    _ = peers_and_metadata.remove_peer_metadata(remote_peer_network_id, connection_metadata.connection_id);
}

pub static NETWORK_PEER_READ_MESSAGES: Lazy<IntCounterVec> = Lazy::new(||
    register_int_counter_vec!(
    "aptos_network_peer_read_messages",
    "Number of messages read (after de-frag)",
    &["network_id", "protocol_id"]
).unwrap()
);

pub static NETWORK_PEER_READ_BYTES: Lazy<IntCounterVec> = Lazy::new(||
    register_int_counter_vec!(
    "aptos_network_peer_read_bytes",
    "Number of message bytes read (after de-frag)",
    &["network_id", "protocol_id"]
).unwrap()
);
pub fn peer_read_message_bytes(network_id: &NetworkId, protocol_id: &ProtocolId, data_len: u64) {
    let values = [network_id.as_str(), protocol_id.as_str()];
    NETWORK_PEER_READ_MESSAGES.with_label_values(&values).inc();
    NETWORK_PEER_READ_BYTES.with_label_values(&values).inc_by(data_len);
}

pub static NETWORK_APP_INBOUND_DROP_MESSAGES: Lazy<IntCounterVec> = Lazy::new(||
    register_int_counter_vec!(
    "aptos_network_app_inbound_drop_messages",
    "Number of messages received but dropped before app",
    &["network_id", "protocol_id"]
).unwrap()
);
pub static NETWORK_APP_INBOUND_DROP_BYTES: Lazy<IntCounterVec> = Lazy::new(||
    register_int_counter_vec!(
    "aptos_network_app_inbound_drop_bytes",
    "Number of bytes received but dropped before app",
    &["network_id", "protocol_id"]
).unwrap()
);
pub fn app_inbound_drop(network_id: &NetworkId, protocol_id: &ProtocolId, data_len: u64) {
    let values = [network_id.as_str(), protocol_id.as_str()];
    NETWORK_APP_INBOUND_DROP_MESSAGES.with_label_values(&values).inc();
    NETWORK_APP_INBOUND_DROP_BYTES.with_label_values(&values).inc_by(data_len);
}

/// Estimate size of NetworkMessage as wrapped in MultiplexMessage and BCS serialized
fn estimate_serialized_length(msg: &NetworkMessage) -> usize {
    msg.header_len() + msg.data_len() + 3
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_memsocket::MemorySocket;
    use rand::{rngs::OsRng, Rng};
    use crate::protocols::wire::messaging::v1::DirectSendMsg;
    use aptos_types::account_address::AccountAddress;
    use crate::application::ApplicationConnections;
    use hex::ToHex;
    use std::cmp::min;

    #[tokio::test]
    pub async fn test_stream_multiplexing() {
        aptos_logger::Logger::init_for_testing();
        // much borrowed from start_peer()
        let config = NetworkConfig::default();
        let max_frame_size = 128; // smaller than real to force fragmentation of many messages to test fragmentation
        // let (async_write, async_read) = async_pipe::pipe();
        let (outbound, inbound) = MemorySocket::new_pair();
        let reader = MultiplexMessageStream::new(inbound, max_frame_size).fuse();
        let writer = MultiplexMessageSink::new(outbound, max_frame_size);
        let role_type = RoleType::Validator;
        let closed = Closer::new();
        let network_id = NetworkId::Validator;
        let (sender, to_send) = tokio::sync::mpsc::channel::<(NetworkMessage, u64)>(config.network_channel_size);
        let (_sender_high_prio, to_send_high_prio) = tokio::sync::mpsc::channel::<(NetworkMessage, u64)>(config.network_channel_size);

        let queue_size = 101;
        let counter_label = "test";
        let mut apps = ApplicationCollector::new();
        let (app_con, mut receiver) = ApplicationConnections::build(ProtocolId::NetbenchDirectSend, queue_size, counter_label);
        apps.add(app_con);
        let apps = Arc::new(apps);
        let remote_peer_network_id = PeerNetworkId::new(network_id, AccountAddress::random());
        let open_outbound_rpc = OutboundRpcMatcher::new();

        let handle = Handle::current();
        handle.spawn(writer_task(network_id, to_send, to_send_high_prio, writer, max_frame_size, role_type, closed.clone()));
        handle.spawn(reader_task(reader, apps, remote_peer_network_id, open_outbound_rpc.clone(), handle.clone(), closed.clone(), role_type));

        let max_data_len = max_frame_size * 13;
        let mut blob = Vec::<u8>::with_capacity(max_data_len);
        let mut rng = OsRng;
        for _ in 0..max_data_len {
            blob.push(rng.gen());
        }

        let num_test_messages = 300; // this should be about 2.5x max_frame_size so we cover the unsplit case up to about 3 fragments.
        let test_len = move |i| {
            i + max_frame_size - 50
        };

        // limit the number of sends in flight, test needs to cheat and have this rate limiting between source and sink otherwise messages will get dropped and this tests wants to see perfect message sequence. Normal operation drops messages when the application can't read them as fast as they're coming in, but in test mode we can easily get to where tokio doesn't let the consumer threads run and they drop messages even though they are runnable.
        let send_sem = Arc::new(tokio::sync::Semaphore::new(50));
        let send_sem_readside = send_sem.clone();

        let sender_blob = blob.clone();
        let mut send_thread_closer = Closer::new();
        let send_thread_close_sender = send_thread_closer.clone();
        let send_thread = async move {
            for i in 0..num_test_messages {
                send_sem.acquire().await.unwrap().forget();
                let send_end = i + test_len(i);
                let raw_msg = sender_blob.get(i..send_end).unwrap();
                let raw_msg = Vec::<u8>::from(raw_msg);
                let msg = NetworkMessage::DirectSendMsg(DirectSendMsg {
                    protocol_id: ProtocolId::NetbenchDirectSend,
                    priority: 0,
                    raw_msg,
                });
                let send_size = send_end - i;
                let hexlen = min(send_size, 10);
                let hexend = i + hexlen;
                let hexwat = sender_blob.get(i..hexend).unwrap().encode_hex::<String>();
                println!("send {:?} size={:?} {:?}...", i, send_size, hexwat);
                if let Err(err) = sender.send((msg, 0)).await {
                    panic!("send err: {:?}", err);
                }
                // println!("sent {:?}", i);
            }
            println!("send thread wait");
            send_thread_closer.wait().await;
            println!("send thread exit");
        };
        handle.spawn(send_thread);

        let mut errcount = 0;
        for i in 0..num_test_messages {
            let send_end = i + test_len(i);
            let raw_msg = blob.get(i..send_end).unwrap();
            let raw_msg = Vec::<u8>::from(raw_msg);
            match receiver.recv().await {
                None => {
                    panic!("recv end early at i={:?}", i)
                }
                Some(rmsg) => {
                    match rmsg.message {
                        NetworkMessage::DirectSendMsg(msg) => {
                            if raw_msg == msg.raw_msg {
                                println!("got {:?} ok [{:?}] bytes", i, msg.raw_msg.len());
                            } else {
                                println!("msg {:?} not equal [{:?}] bytes", i, msg.raw_msg.len());
                                errcount += 1;
                                if errcount >= 10 {
                                    panic!("too many errors");
                                }
                            }
                        }
                        _ => {
                            panic!("bad message")
                        }
                    }
                }
            }
            send_sem_readside.add_permits(1);
        }
        println!("outer done");
        send_thread_close_sender.close().await;
        println!("done done");
    }
}
