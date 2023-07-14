// Copyright © Aptos Foundation

use crate::{
    network_controller::{error::Error, Message, MessageType, NetworkMessage},
    NetworkServer,
};
use aptos_logger::error;
use crossbeam_channel::Sender;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    thread,
};

#[allow(dead_code)]
pub struct InboundHandler {
    server: Arc<Mutex<NetworkServer>>,
    // Used to route incoming messages to correct channel.
    inbound_handlers: Arc<Mutex<HashMap<MessageType, Sender<Message>>>>,
}

impl InboundHandler {
    pub fn new(service: &'static str, listen_addr: SocketAddr, timeout_ms: u64) -> Self {
        Self {
            server: Arc::new(Mutex::new(NetworkServer::new(
                service,
                listen_addr,
                timeout_ms,
            ))),
            inbound_handlers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register_handler(&self, message_type: String, sender: Sender<Message>) {
        assert!(!self
            .inbound_handlers
            .lock()
            .unwrap()
            .contains_key(&MessageType::new(message_type.clone())));
        let mut inbound_handlers = self.inbound_handlers.lock().unwrap();
        inbound_handlers.insert(MessageType::new(message_type), sender);
    }

    pub fn start(&mut self) {
        let inbound_handlers = self.inbound_handlers.clone(); // Clone the hashmap for the thread
        let server_clone = self.server.clone(); // Clone the server to move into the thread
        // Spawn a thread to handle incoming messages
        thread::spawn(move || {
            loop {
                // Receive incoming messages from the server
                if let Err(e) = Self::process_one_incoming_message(&server_clone, &inbound_handlers)
                {
                    error!("Error processing message: {:?}", e);
                }
            }
        });
    }

    fn process_one_incoming_message(
        network_server: &Arc<Mutex<NetworkServer>>,
        inbound_handlers: &Arc<Mutex<HashMap<MessageType, Sender<Message>>>>,
    ) -> Result<(), Error> {
        let message = network_server.lock().unwrap().read()?;
        let network_msg: NetworkMessage = bcs::from_bytes(&message)?;
        // Get the sender's SocketAddr from the received message
        let sender = network_msg.sender;
        let msg = network_msg.message;
        let message_type = network_msg.message_type;

        // Check if there is a registered handler for the sender
        if let Some(handler) = inbound_handlers.lock().unwrap().get(&message_type) {
            // Send the message to the registered handler
            handler.send(msg)?;
        } else {
            error!("No handler registered for sender: {:?}", sender);
        }
        Ok(())
    }
}
