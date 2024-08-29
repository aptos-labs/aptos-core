// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use aptos_block_executor::txn_provider::sharded::{CrossShardClientForV3, CrossShardMessage};
use aptos_secure_net::network_controller::{Message, MessageType, NetworkController};
use aptos_types::{
    block_executor::partitioner::{RoundId, ShardId, MAX_ALLOWED_PARTITIONING_ROUNDS},
    vm_status::VMStatus,
};
use aptos_vm::{
    sharded_block_executor::{cross_shard_client::CrossShardClient, messages::CrossShardMsg},
};
use crossbeam_channel::{Receiver, Sender};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use aptos_secure_net::grpc_network_service::outbound_rpc_helper::OutboundRpcHelper;
use aptos_types::transaction::signature_verified_transaction::SignatureVerifiedTransaction;

pub struct RemoteCrossShardClient {
    // The senders of cross-shard messages to other shards per round.
    message_txs: Arc<Vec<Vec<Mutex<Sender<Message>>>>>,
    // The receivers of cross shard messages from other shards per round.
    message_rxs: Arc<Vec<Mutex<Receiver<Message>>>>,
}

impl RemoteCrossShardClient {
    pub fn new(controller: &mut NetworkController, shard_addresses: Vec<SocketAddr>) -> Self {
        let mut message_txs = vec![];
        let mut message_rxs = vec![];
        // Create outbound channels for each shard per round.
        for remote_address in shard_addresses.iter() {
            let mut txs = vec![];
            for round in 0..MAX_ALLOWED_PARTITIONING_ROUNDS {
                let message_type = format!("cross_shard_{}", round);
                let tx = controller.create_outbound_channel(*remote_address, message_type);
                txs.push(Mutex::new(tx));
            }
            message_txs.push(txs);
        }

        // Create inbound channels for each round
        for round in 0..MAX_ALLOWED_PARTITIONING_ROUNDS {
            let message_type = format!("cross_shard_{}", round);
            let rx = controller.create_inbound_channel(message_type);
            message_rxs.push(Mutex::new(rx));
        }

        Self {
            message_txs: Arc::new(message_txs),
            message_rxs: Arc::new(message_rxs),
        }
    }
}

impl CrossShardClient for RemoteCrossShardClient {
    fn send_global_msg(&self, _msg: CrossShardMsg) {
        todo!("Global cross shard message is not supported yet in remote execution mode")
    }

    fn send_cross_shard_msg(&self, shard_id: ShardId, round: RoundId, msg: CrossShardMsg) {
        let input_message = bcs::to_bytes(&msg).unwrap();
        let tx = self.message_txs[shard_id][round].lock().unwrap();
        tx.send(Message::new(input_message)).unwrap();
    }

    fn receive_cross_shard_msg(&self, current_round: RoundId) -> CrossShardMsg {
        let rx = self.message_rxs[current_round].lock().unwrap();
        let message = rx.recv().unwrap();
        let msg: CrossShardMsg = bcs::from_bytes(&message.to_bytes()).unwrap();
        msg
    }
}

pub struct RemoteCrossShardClientV3 {
    // The senders of cross-shard messages to other shards per round.
    message_txs: Arc<Vec<Mutex<OutboundRpcHelper>>>,
    // The receivers of cross shard messages from other shards per round.
    message_rx: Arc<Receiver<Message>>,
}

impl RemoteCrossShardClientV3 {
    pub fn new(controller: &mut NetworkController, shard_addresses: &Vec<SocketAddr>) -> Self {
        let mut message_txs = vec![];
        let self_addr = controller.get_self_addr();
        let outbound_rpc_runtime = controller.get_outbound_rpc_runtime();
        // Create outbound channels for each shard.
        for remote_address in shard_addresses.iter() {
            message_txs.push(Mutex::new(OutboundRpcHelper::new(self_addr, *remote_address, outbound_rpc_runtime.clone())));
        }

        // Create inbound channels for each round
        let cross_shard_msg_type = "cross_shard_msg".to_string();
        let message_rx = controller.create_inbound_channel(cross_shard_msg_type);

        Self {
            message_txs: Arc::new(message_txs),
            message_rx: Arc::new(message_rx),
        }
    }
}

impl CrossShardClientForV3<SignatureVerifiedTransaction, VMStatus> for RemoteCrossShardClientV3 {
    fn send(
        &self,
        shard_idx: usize,
        output: CrossShardMessage<SignatureVerifiedTransaction, VMStatus>,
    ) {
        let msg = Message::new(bcs::to_bytes(&output).unwrap());
        let cross_shard_msg_type = "cross_shard_msg".to_string();
        self.message_txs[shard_idx].lock().unwrap().send(msg, &MessageType::new(cross_shard_msg_type));
    }

    fn recv(&self) -> CrossShardMessage<SignatureVerifiedTransaction, VMStatus> {
        let message = self.message_rx.recv().unwrap();
        let result: CrossShardMessage<SignatureVerifiedTransaction, VMStatus> = bcs::from_bytes(&message.to_bytes()).unwrap();
        result
    }
}
