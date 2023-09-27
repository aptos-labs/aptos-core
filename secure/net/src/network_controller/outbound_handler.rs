// Copyright © Aptos Foundation

use crate::{
    grpc_network_service::RemoteExecutionClientWrapper,
    network_controller::{inbound_handler::InboundHandler, Message, MessageType},
};
use aptos_logger::{info, trace};
use crossbeam_channel::{unbounded, Receiver, Select, Sender};
use std::{
    collections::{HashMap, HashSet},
    mem,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::runtime::Runtime;

pub struct OutboundHandler {
    _service: String,
    remote_addresses: HashSet<SocketAddr>,
    address: SocketAddr,
    // Used to route outgoing messages to correct network client with the correct message type
    handlers: Vec<(Receiver<Message>, SocketAddr, MessageType)>,
    inbound_handler: Arc<Mutex<InboundHandler>>,
}

impl OutboundHandler {
    pub fn new(
        service: String,
        listen_addr: SocketAddr,
        inbound_handler: Arc<Mutex<InboundHandler>>,
    ) -> Self {
        Self {
            _service: service,
            remote_addresses: HashSet::new(),
            address: listen_addr,
            handlers: Vec::new(),
            inbound_handler,
        }
    }

    pub fn register_handler(
        &mut self,
        message_type: String,
        remote_addr: SocketAddr,
        receiver: Receiver<Message>,
    ) {
        self.remote_addresses.insert(remote_addr);
        self.handlers
            .push((receiver, remote_addr, MessageType::new(message_type)));
    }

    pub fn start(&mut self, rt: &Runtime) -> Option<Sender<Message>> {
        if self.handlers.is_empty() {
            return None;
        }

        // Register a signal handler to stop the outbound task
        let (stop_signal_tx, stop_signal_rx) = unbounded();
        self.handlers.push((
            stop_signal_rx,
            self.address,
            MessageType::new("stop_task".to_string()),
        ));

        // Create a grpc client for each remote address
        let mut grpc_clients: HashMap<SocketAddr, RemoteExecutionClientWrapper> = HashMap::new();
        self.remote_addresses.iter().for_each(|remote_addr| {
            grpc_clients.insert(
                *remote_addr,
                RemoteExecutionClientWrapper::new(rt, *remote_addr),
            );
        });

        // Prepare for objects to be moved into the async block (&mut self cannot be moved into the
        // async block)
        let address = self.address;
        let inbound_handler = self.inbound_handler.clone();
        // Moving the handlers out of self is fine because once 'start()' is called we do not intend
        // to register any more handlers. A reference count like Arc<Mutex> has issues of being
        // used across sync and async boundaries, and also not the most efficient because we pay
        // the cost of the mutex when there is no contention
        let outbound_handlers = mem::take(self.handlers.as_mut());

        // TODO: Consider using multiple tasks for outbound handlers
        rt.spawn(async move {
            info!("Starting outbound handler at {}", address.to_string());
            Self::process_one_outgoing_message(
                outbound_handlers,
                &address,
                inbound_handler.clone(),
                &mut grpc_clients,
            )
            .await;
            info!("Stopping outbound handler at {}", address.to_string());
        });
        Some(stop_signal_tx)
    }

    async fn process_one_outgoing_message(
        outbound_handlers: Vec<(Receiver<Message>, SocketAddr, MessageType)>,
        socket_addr: &SocketAddr,
        inbound_handler: Arc<Mutex<InboundHandler>>,
        grpc_clients: &mut HashMap<SocketAddr, RemoteExecutionClientWrapper>,
    ) {
        loop {
            let mut select = Select::new();
            for (receiver, _, _) in outbound_handlers.iter() {
                select.recv(receiver);
            }

            let index;
            let msg;
            {
                let oper = select.select();
                index = oper.index();
                match oper.recv(&outbound_handlers[index].0) {
                    Ok(m) => {
                        msg = m;
                    },
                    Err(e) => {
                        // not necessarily an error, just means a sender closed a channel
                        trace!(
                            "{:?} on outbound handler for {:?}",
                            e.to_string(),
                            socket_addr
                        );
                        continue;
                    },
                }
            }

            let remote_addr = &outbound_handlers[index].1;
            let message_type = &outbound_handlers[index].2;

            if message_type.get_type() == "stop_task" {
                return;
            }

            if remote_addr == socket_addr {
                // If the remote address is the same as the local address, then we are sending a message to ourselves
                // so we should just pass it to the inbound handler
                inbound_handler
                    .lock()
                    .unwrap()
                    .send_incoming_message_to_handler(message_type, msg);
            } else {
                grpc_clients
                    .get_mut(remote_addr)
                    .unwrap()
                    .send_message(*socket_addr, msg, message_type)
                    .await;
            }
        }
    }
}
