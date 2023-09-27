// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::network_controller::{Message, MessageType};
use aptos_logger::{error, info};
use aptos_protos::remote_executor::v1::{
    remote_execution_client::RemoteExecutionClient,
    remote_execution_server::{RemoteExecution, RemoteExecutionServer},
    Empty, NetworkMessage, FILE_DESCRIPTOR_SET,
};
use crossbeam_channel::Sender;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::{runtime::Runtime, sync::oneshot};
use tonic::{
    transport::{Channel, Server},
    Request, Response, Status,
};

pub struct RemoteExecutionServerWrapper {
    inbound_handlers: Arc<Mutex<HashMap<MessageType, Sender<Message>>>>,
}

impl RemoteExecutionServerWrapper {
    pub fn new(inbound_handlers: Arc<Mutex<HashMap<MessageType, Sender<Message>>>>) -> Self {
        Self { inbound_handlers }
    }

    // Note: The object is consumed here. That is once the server is started, we cannot/should not
    //       use the object anymore
    pub fn start(
        self,
        rt: &Runtime,
        _service: String,
        server_addr: SocketAddr,
        server_shutdown_rx: oneshot::Receiver<()>,
    ) {
        rt.spawn(async move {
            self.start_async(server_addr, server_shutdown_rx).await;
        });
    }

    async fn start_async(self, server_addr: SocketAddr, server_shutdown_rx: oneshot::Receiver<()>) {
        let reflection_service = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
            .build()
            .unwrap();

        info!("Starting Server async at {:?}", server_addr);
        // NOTE: serve_with_shutdown() starts the server, if successful the task does not return
        // till the server is shutdown. Hence this should be called as a separate non-blocking task.
        // Signal handler 'server_shutdown_rx' is needed to shutdown the server
        Server::builder()
            .add_service(RemoteExecutionServer::new(self))
            .add_service(reflection_service)
            .serve_with_shutdown(server_addr, async {
                server_shutdown_rx.await.ok();
            })
            .await
            .unwrap();
        info!("Server shutdown at {:?}", server_addr);
    }
}

#[tonic::async_trait]
impl RemoteExecution for RemoteExecutionServerWrapper {
    async fn simple_msg_exchange(
        &self,
        request: Request<NetworkMessage>,
    ) -> Result<Response<Empty>, Status> {
        let network_message = request.into_inner();
        let sender = network_message.sender_addr;
        let msg = Message::new(network_message.message);
        let message_type = MessageType::new(network_message.message_type);

        if let Some(handler) = self.inbound_handlers.lock().unwrap().get(&message_type) {
            // Send the message to the registered handler
            handler.send(msg).unwrap();
        } else {
            error!(
                "No handler registered for sender: {:?} and msg type {:?}",
                sender, message_type
            );
        }
        Ok(Response::new(Empty {}))
    }
}

pub struct RemoteExecutionClientWrapper {
    remote_addr: String,
    remote_channel: RemoteExecutionClient<Channel>,
}

impl RemoteExecutionClientWrapper {
    pub fn new(rt: &Runtime, remote_addr: SocketAddr) -> Self {
        Self {
            remote_addr: remote_addr.to_string(),
            remote_channel: rt
                .block_on(async { Self::get_channel(format!("http://{}", remote_addr)).await }),
        }
    }

    async fn get_channel(remote_addr: String) -> RemoteExecutionClient<Channel> {
        info!("Trying to connect to remote server at {:?}", remote_addr);
        let conn = tonic::transport::Endpoint::new(remote_addr)
            .unwrap()
            .connect_lazy();
        RemoteExecutionClient::new(conn)
    }

    pub async fn send_message(
        &mut self,
        sender_addr: SocketAddr,
        message: Message,
        mt: &MessageType,
    ) {
        let request = tonic::Request::new(NetworkMessage {
            sender_addr: sender_addr.to_string(),
            message: message.data,
            message_type: mt.get_type(),
        });
        // TODO: Retry with exponential backoff on failure
        match self.remote_channel.simple_msg_exchange(request).await {
            Ok(_) => {},
            Err(e) => {
                error!(
                    "Error '{}' sending message to {} on node {:?}",
                    e, self.remote_addr, sender_addr
                );
            },
        }
    }
}

#[test]
fn basic_test() {
    use aptos_config::utils;
    use std::{
        net::{IpAddr, Ipv4Addr},
        thread,
    };

    let server_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), utils::get_available_port());
    let message_type = "test_type".to_string();
    let server_handlers: Arc<Mutex<HashMap<MessageType, Sender<Message>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let (msg_tx, msg_rx) = crossbeam_channel::unbounded();
    server_handlers
        .lock()
        .unwrap()
        .insert(MessageType::new(message_type.clone()), msg_tx);
    let server = RemoteExecutionServerWrapper::new(server_handlers);

    let rt = Runtime::new().unwrap();
    let (server_shutdown_tx, server_shutdown_rx) = oneshot::channel();
    server.start(
        &rt,
        "unit tester".to_string(),
        server_addr,
        server_shutdown_rx,
    );

    let mut grpc_client = RemoteExecutionClientWrapper::new(&rt, server_addr);

    let client_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), utils::get_available_port());
    let test_message_content = "test1".as_bytes().to_vec();

    // wait for the server to be ready before sending messages
    // TODO: We need to implement retry on send_message failures such that we can pass this test
    //       without this sleep
    thread::sleep(std::time::Duration::from_millis(10));

    for _ in 0..2 {
        rt.block_on(async {
            grpc_client
                .send_message(
                    client_addr,
                    Message::new(test_message_content.clone()),
                    &MessageType::new(message_type.clone()),
                )
                .await;
        });
    }

    for _ in 0..2 {
        let received_msg = msg_rx.recv().unwrap();
        assert_eq!(received_msg.data, test_message_content);
    }
    server_shutdown_tx.send(()).unwrap();
}
