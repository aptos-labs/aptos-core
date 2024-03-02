// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0



use futures_channel::mpsc::{UnboundedSender};
use aptos_logger::error;
use crate::pipeline::buffer_manager::OrderedBlocks;


pub struct ShoalppOrderNotifier {
    ordered_nodes_tx: UnboundedSender<OrderedBlocks>,
    receivers: Vec<tokio::sync::mpsc::UnboundedReceiver<OrderedBlocks>>,
}

impl ShoalppOrderNotifier {
    pub fn new(ordered_nodes_tx: UnboundedSender<OrderedBlocks>, receivers: Vec<tokio::sync::mpsc::UnboundedReceiver<OrderedBlocks>>) -> Self {
        Self {
            ordered_nodes_tx,
            receivers,
        }
    }

    pub async fn run(mut self) {
        // TODO: shutdown logic

        loop {
            for receiver in self.receivers.iter_mut() {
                if let Some(block) = receiver.recv().await {
                    if let Err(e) = self.ordered_nodes_tx.unbounded_send(block){
                        error!("Failed to send ordered nodes {:?}", e);
                    }
                } else {
                    // shutdown in progress, but notifier should be killed before DAG
                    error!("Failed to receive message");
                    // Panic for debugging
                    panic!();
                }
            }
        }
    }
}