// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network::IncomingRandGenRequest,
    pipeline::buffer_manager::{OrderedBlocks, ResetRequest},
};
use aptos_consensus_types::common::Author;
use aptos_types::epoch_state::EpochState;
use std::sync::Arc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub type Sender<T> = UnboundedSender<T>;
pub type Receiver<T> = UnboundedReceiver<T>;

pub struct RandManager {
    author: Author,
    epoch_state: Arc<EpochState>,

    incoming_blocks: Receiver<OrderedBlocks>,
    reset_rx: Receiver<ResetRequest>,
    rand_msg_rx: aptos_channels::aptos_channel::Receiver<Author, IncomingRandGenRequest>,

    outgoing_blocks: Sender<OrderedBlocks>,
}
