// Copyright © Aptos Foundation

use crate::{
    network_controller::{inbound_handler::InboundHandler, Message, MessageType, NetworkMessage},
    NetworkClient,
};
use aptos_retrier::{fixed_retry_strategy, retry};
use crossbeam_channel::{Receiver, Select};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    thread,
};

pub struct OutboundHandler {
    service: String,
    network_clients: Arc<Mutex<HashMap<SocketAddr, NetworkClient>>>,
    address: SocketAddr,
    // Used to route outgoing messages to correct network client with the correct message type
    handlers: Arc<Mutex<Vec<(Receiver<Message>, SocketAddr, MessageType)>>>,
    inbound_handler: Arc<Mutex<InboundHandler>>,
}

impl OutboundHandler {
    pub fn new(
        service: String,
        listen_addr: SocketAddr,
        inbound_handler: Arc<Mutex<InboundHandler>>,
    ) -> Self {
        Self {
            service,
            network_clients: Arc::new(Mutex::new(HashMap::new())),
            address: listen_addr,
            handlers: Arc::new(Mutex::new(Vec::new())),
            inbound_handler,
        }
    }

    pub fn register_handler(
        &self,
        message_type: String,
        remote_addr: SocketAddr,
        receiver: Receiver<Message>,
    ) {
        // Create a remote client if it doesn't exist
        self.network_clients
            .lock()
            .unwrap()
            .entry(remote_addr)
            .or_insert_with(|| NetworkClient::new(message_type.clone(), remote_addr, 5000));
        let mut handlers = self.handlers.lock().unwrap();
        handlers.push((receiver, remote_addr, MessageType::new(message_type)));
    }

    pub fn start(&mut self) {
        let outbound_handlers = self.handlers.clone();
        let address = self.address;
        let network_clients = self.network_clients.clone();
        let thread_name = format!("{}_network_outbound_handler", self.service);
        let builder = thread::Builder::new().name(thread_name);
        let inbound_handler = self.inbound_handler.clone();
        builder
            .spawn(move || loop {
                Self::process_one_outgoing_message(
                    outbound_handlers.clone(),
                    network_clients.clone(),
                    &address,
                    inbound_handler.clone(),
                )
            })
            .expect("Failed to spawn outbound handler thread");
    }

    fn process_one_outgoing_message(
        outbound_handlers: Arc<Mutex<Vec<(Receiver<Message>, SocketAddr, MessageType)>>>,
        network_clients: Arc<Mutex<HashMap<SocketAddr, NetworkClient>>>,
        socket_addr: &SocketAddr,
        inbound_handler: Arc<Mutex<InboundHandler>>,
    ) {
        let mut select = Select::new();
        let handlers = outbound_handlers.lock().unwrap();

        for (receiver, _, _) in handlers.iter() {
            select.recv(receiver);
        }
        let oper = select.select();
        let index = oper.index();
        let msg = oper.recv(&handlers[index].0).unwrap();
        let remote_addr = &handlers[index].1;
        let message_type = &handlers[index].2;
        if remote_addr == socket_addr {
            // If the remote address is the same as the local address, then we are sending a message to ourselves
            // so we should just pass it to the inbound handler
            inbound_handler
                .lock()
                .unwrap()
                .send_incoming_message_to_handler(message_type, msg);
            return;
        }
        let mut binding = network_clients.lock().unwrap();
        let network_client = binding.get_mut(remote_addr).unwrap();
        let msg = bcs::to_bytes(&NetworkMessage::new(
            *socket_addr,
            msg,
            message_type.clone(),
        ))
        .unwrap();

        retry(fixed_retry_strategy(5, 20), || network_client.write(&msg)).unwrap();
    }
}
