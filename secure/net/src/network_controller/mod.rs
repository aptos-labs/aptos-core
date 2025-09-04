// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::network_controller::{
    inbound_handler::InboundHandler, outbound_handler::OutboundHandler,
};
use velor_logger::{info, warn};
use crossbeam_channel::{unbounded, Receiver, Sender};
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::{runtime::Runtime, sync::oneshot};

mod error;
mod inbound_handler;
pub(crate) mod metrics;
mod outbound_handler;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct NetworkMessage {
    pub sender: SocketAddr,
    pub message: Message,
    pub message_type: MessageType,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, Hash, PartialEq)]
#[allow(dead_code)]
pub struct MessageType {
    message_type: String,
}

impl MessageType {
    pub fn new(message_type: String) -> Self {
        Self { message_type }
    }

    pub fn get_type(&self) -> String {
        self.message_type.clone()
    }
}

impl NetworkMessage {
    pub fn new(sender: SocketAddr, message: Message, message_type: MessageType) -> Self {
        Self {
            sender,
            message,
            message_type,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct Message {
    pub data: Vec<u8>,
}

impl Message {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn to_bytes(self) -> Vec<u8> {
        self.data
    }
}

/// NetworkController is the main entry point for sending and receiving messages over the network.
/// 1. If a node acts as both client and server, albeit in different contexts, GRPC needs separate
///    runtimes for client context and server context. Otherwise we a hang in GRPC. This seems to be
///    an internal bug in GRPC.
/// 2. We want to use tokio runtimes because it is best for async IO and tonic GRPC
///    implementation is async. However, we want the rest of the system (remote executor service)
///    to use rayon thread pools because it is best for CPU bound tasks.
/// 3. NetworkController, InboundHandler and OutboundHandler work as a bridge between the sync and
///    async worlds.
/// 4. We need to shutdown all the async tasks spawned by the NetworkController runtimes, otherwise
///    the program will hang, or have resource leaks.
#[allow(dead_code)]
pub struct NetworkController {
    inbound_handler: Arc<Mutex<InboundHandler>>,
    outbound_handler: OutboundHandler,
    inbound_rpc_runtime: Runtime,
    outbound_rpc_runtime: Runtime,
    inbound_server_shutdown_tx: Option<oneshot::Sender<()>>,
    outbound_task_shutdown_tx: Option<Sender<Message>>,
    listen_addr: SocketAddr,
}

impl NetworkController {
    pub fn new(service: String, listen_addr: SocketAddr, timeout_ms: u64) -> Self {
        let inbound_handler = Arc::new(Mutex::new(InboundHandler::new(
            service.clone(),
            listen_addr,
            timeout_ms,
        )));
        let outbound_handler = OutboundHandler::new(service, listen_addr, inbound_handler.clone());
        info!("Network controller created for node {}", listen_addr);
        Self {
            inbound_handler,
            outbound_handler,
            inbound_rpc_runtime: Runtime::new().unwrap(),
            outbound_rpc_runtime: Runtime::new().unwrap(),
            // we initialize the shutdown handles when we start the network controller
            inbound_server_shutdown_tx: None,
            outbound_task_shutdown_tx: None,
            listen_addr,
        }
    }

    pub fn create_outbound_channel(
        &mut self,
        remote_peer_addr: SocketAddr,
        message_type: String,
    ) -> Sender<Message> {
        let (outbound_sender, outbound_receiver) = unbounded();

        self.outbound_handler
            .register_handler(message_type, remote_peer_addr, outbound_receiver);

        outbound_sender
    }

    pub fn create_inbound_channel(&mut self, message_type: String) -> Receiver<Message> {
        let (inbound_sender, inbound_receiver) = unbounded();

        self.inbound_handler
            .lock()
            .unwrap()
            .register_handler(message_type, inbound_sender);

        inbound_receiver
    }

    pub fn start(&mut self) {
        info!(
            "Starting network controller started for at {}",
            self.listen_addr
        );
        self.inbound_server_shutdown_tx = self
            .inbound_handler
            .lock()
            .unwrap()
            .start(&self.inbound_rpc_runtime);
        self.outbound_task_shutdown_tx = self.outbound_handler.start(&self.outbound_rpc_runtime);
    }

    // TODO: This is still not a very clean shutdown. We don't wait for the full shutdown after
    //       sending the signal. May not matter much for now because we shutdown before exiting the
    //       process. Ideally, we want to fix this.
    pub fn shutdown(&mut self) {
        info!("Shutting down network controller at {}", self.listen_addr);
        if let Some(shutdown_signal) = self.inbound_server_shutdown_tx.take() {
            shutdown_signal.send(()).unwrap();
        }

        if let Some(shutdown_signal) = self.outbound_task_shutdown_tx.take() {
            shutdown_signal.send(Message::new(vec![])).unwrap_or_else(|_| {
                warn!("Failed to send shutdown signal to outbound task; probably already shutdown");
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::network_controller::{Message, NetworkController};
    use velor_config::utils;
    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        thread,
    };

    #[test]
    fn test_basic_send_receive() {
        let server_port1 = utils::get_available_port();
        let server_addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), server_port1);

        let server_port2 = utils::get_available_port();
        let server_addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), server_port2);

        let mut network_controller1 =
            NetworkController::new("test1".to_string(), server_addr1, 1000);
        let mut network_controller2 =
            NetworkController::new("test2".to_string(), server_addr2, 1000);

        let test1_sender =
            network_controller2.create_outbound_channel(server_addr1, "test1".to_string());
        let test1_receiver = network_controller1.create_inbound_channel("test1".to_string());

        let test2_sender =
            network_controller1.create_outbound_channel(server_addr2, "test2".to_string());
        let test2_receiver = network_controller2.create_inbound_channel("test2".to_string());

        network_controller1.start();
        network_controller2.start();

        // wait for the server to be ready to serve
        // TODO: We need to pass this test without this sleep
        thread::sleep(std::time::Duration::from_millis(100));

        let test1_message = "test1".as_bytes().to_vec();
        test1_sender
            .send(Message::new(test1_message.clone()))
            .unwrap();

        let test2_message = "test2".as_bytes().to_vec();
        test2_sender
            .send(Message::new(test2_message.clone()))
            .unwrap();

        let received_test1_message = test1_receiver.recv().unwrap();
        assert_eq!(received_test1_message.data, test1_message);

        let received_test2_message = test2_receiver.recv().unwrap();
        assert_eq!(received_test2_message.data, test2_message);

        network_controller1.shutdown();
        network_controller2.shutdown();
        thread::sleep(std::time::Duration::from_millis(100));
    }
}
