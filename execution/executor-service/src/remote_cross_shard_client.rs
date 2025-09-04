// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0
use velor_secure_net::network_controller::{Message, NetworkController};
use velor_types::block_executor::partitioner::{RoundId, ShardId, MAX_ALLOWED_PARTITIONING_ROUNDS};
use velor_vm::sharded_block_executor::{
    cross_shard_client::CrossShardClient, messages::CrossShardMsg,
};
use crossbeam_channel::{Receiver, Sender};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

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
