// Copyright © Aptos Foundation

use crate::{error::Error, NetworkClient, NetworkServer};
use aptos_logger::error;
use crossbeam_channel::{select, unbounded, Receiver, Select, Sender};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NetworkMessage {
    pub sender: SocketAddr,
    pub message: Message,
}

impl NetworkMessage {
    pub fn new(sender: SocketAddr, message: Message) -> Self {
        Self { sender, message }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Message {
    pub data: Vec<u8>,
}

impl Message {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

pub struct NetworkController {
    server: Arc<Mutex<NetworkServer>>,
    address: SocketAddr,
    outbound_handlers: Arc<Mutex<Vec<(Receiver<Message>, NetworkClient)>>>,
    inbound_handlers: Arc<Mutex<HashMap<SocketAddr, Sender<Message>>>>,
}

impl NetworkController {
    pub fn new(service: &'static str, listen_addr: SocketAddr, timeout_ms: u64) -> Self {
        Self {
            server: Arc::new(Mutex::new(NetworkServer::new(
                service,
                listen_addr,
                timeout_ms,
            ))),
            address: listen_addr,
            outbound_handlers: Arc::new(Mutex::new(vec![])),
            inbound_handlers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn start(&mut self) {
        self.start_incoming_msg_handler();
    }

    pub fn start_incoming_msg_handler(&mut self) {
        let inbound_handlers = self.inbound_handlers.clone(); // Clone the hashmap for the thread
                                                              // Spawn a thread to handle incoming messages
        let server_clone = self.server.clone(); // Clone the server to move into the thread
        thread::spawn(move || {
            loop {
                // Receive incoming messages from the server
                if let Err(e) = Self::process_one_message(&server_clone, &inbound_handlers) {
                    error!("Error processing message: {:?}", e);
                }
            }
        });
    }

    fn process_one_message(
        network_server: &Arc<Mutex<NetworkServer>>,
        inbound_handlers: &Arc<Mutex<HashMap<SocketAddr, Sender<Message>>>>,
    ) -> Result<(), Error> {
        let message = network_server.lock().unwrap().read()?;
        let network_msg: NetworkMessage = bcs::from_bytes(&message)?;
        // Get the sender's SocketAddr from the received message
        let sender = network_msg.sender;
        let msg = network_msg.message;

        // Check if there is a registered handler for the sender
        if let Some(handler) = inbound_handlers.lock().unwrap().get(&sender) {
            // Send the message to the registered handler
            handler.send(msg)?;
        } else {
            error!("No handler registered for sender: {:?}", sender);
        }
        Ok(())
    }

    pub fn start_outgoing_msg_handler(&mut self) {
        let outbound_handlers = self.outbound_handlers.clone();
        let address = self.address;
        thread::spawn(move || {
            let mut sel = Select::new();
            let handlers = outbound_handlers.lock().unwrap();

            for (receiver, _) in handlers.iter() {
                sel.recv(receiver);
            }
            loop {
                if let Err(e) =
                    Self::process_one_outgoing_message(&mut sel, &outbound_handlers, &address)
                {
                    error!("Error processing outgoing message: {:?}", e);
                }
            }
        });
    }

    fn process_one_outgoing_message(
        select: &mut Select,
        outbound_handlers: &Arc<Mutex<Vec<(Receiver<Message>, NetworkClient)>>>,
        socket_addr: &SocketAddr,
    ) -> Result<(), Error> {
        let oper = select.select();
        let index = oper.index();
        let msg = oper.recv(&outbound_handlers.lock().unwrap()[index].0)?;
        let client = &mut outbound_handlers.lock().unwrap()[index].1;
        let msg = bcs::to_bytes(&NetworkMessage::new(*socket_addr, msg))?;
        client.write(&msg)?;
        Ok(())
    }
}
