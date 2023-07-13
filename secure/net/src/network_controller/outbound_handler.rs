// Copyright © Aptos Foundation

use crate::{
    network_controller::{error::Error, Message, MessageType, NetworkMessage},
    NetworkClient,
};
use aptos_logger::error;
use crossbeam_channel::{Receiver, Select};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    thread,
};

#[allow(dead_code)]
pub struct OutboundHandler {
    network_clients: Arc<Mutex<HashMap<SocketAddr, NetworkClient>>>,
    address: SocketAddr,
    // Used to route outgoing messages to correct network client with the correct message type
    handlers: Arc<Mutex<Vec<(Receiver<Message>, SocketAddr, MessageType)>>>,
}

impl OutboundHandler {
    pub fn new(listen_addr: SocketAddr) -> Self {
        Self {
            network_clients: Arc::new(Mutex::new(HashMap::new())),
            address: listen_addr,
            handlers: Arc::new(Mutex::new(Vec::new())),
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
        thread::spawn(move || loop {
            if let Err(e) = Self::process_one_outgoing_message(
                outbound_handlers.clone(),
                network_clients.clone(),
                &address,
            ) {
                error!("Error processing outgoing message: {:?}", e);
            }
        });
    }

    fn process_one_outgoing_message(
        outbound_handlers: Arc<Mutex<Vec<(Receiver<Message>, SocketAddr, MessageType)>>>,
        network_clients: Arc<Mutex<HashMap<SocketAddr, NetworkClient>>>,
        socket_addr: &SocketAddr,
    ) -> Result<(), Error> {
        let mut select = Select::new();
        let handlers = outbound_handlers.lock().unwrap();

        for (receiver, _, _) in handlers.iter() {
            select.recv(receiver);
        }
        let oper = select.select();
        let index = oper.index();
        let msg = oper.recv(&handlers[index].0)?;
        let remote_addr = &handlers[index].1;
        let message_type = &handlers[index].2;
        let mut binding = network_clients.lock().unwrap();
        let network_client = binding.get_mut(remote_addr).unwrap();
        let msg = bcs::to_bytes(&NetworkMessage::new(
            *socket_addr,
            msg,
            message_type.clone(),
        ))?;
        network_client.write(&msg)?;
        Ok(())
    }
}
