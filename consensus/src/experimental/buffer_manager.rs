// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use consensus_types::executed_block::ExecutedBlock;
use diem_types::ledger_info::LedgerInfoWithSignatures;
use futures::channel::{mpsc::UnboundedReceiver, oneshot};
use std::{collections::LinkedList, sync::Arc};

pub type ResetAck = ();
pub fn reset_ack_new() -> ResetAck {}

pub struct ResetRequest {
    tx: oneshot::Sender<ResetAck>,
    reconfig: bool,
}

pub struct OrderedBlocks {
    pub blocks: Vec<ExecutedBlock>,
    pub finality_proof: LedgerInfoWithSignatures,
}

/// StateManager handles the states of ordered blocks and
/// interacts with the execution phase, the signing phase, and
/// the persisting phase.
pub struct StateManager {
    buffer: LinkedList<Arc<ExecutedBlock>>,
    li_buffer: LinkedList<(LedgerInfoWithSignatures, Arc<ExecutedBlock>)>,
    // the second item is for updating aggregation_root easily

    /*
    execution_root: HashValue,
    execution_phase_tx,
    execution_phase_rx,

    signing_root: HashValue,
    signing_phase_tx,
    signing_phase_rx,

    aggregation_root: HashValue,
    commit_msg_tx,
    commit_msg_rx,

    persisting_phase_tx,
    persisting_phase_rx,
     */
    block_rx: UnboundedReceiver<OrderedBlocks>,
    reset_rx: UnboundedReceiver<ResetRequest>,
    end_epoch: bool,
}

impl StateManager {
    async fn start(self) {

        // loop receving new blocks or reset
        // while !self.end_epoch {

        // select from all rx channels,
        // if new from block_rx, push to buffer
        // if new from reset_rx, make a mark that stops all the following ops
        // if new from execution_phase_rx,
        //   if execution failure, send all the blocks to execution_phase.
        //   Otherwise,
        //     update execution_root and send the blocks from execution_root to end to execution_phase
        // if new from signing_phase_rx,
        //   update sigining_root and send the blocks from signing_root to execution_root to signing_phase
        // if new from commit_msg_rx,
        //   collect sig and update the sigs
        //   if aggregated,
        //     update aggregation_root
        // if new from persisting_phase_rx,
        //   pop blocks from buffer, and continue to post-committing ops
        //   send the blocks from aggregation_root to the end to persisting_phase

        // if not reset, retry sending the commit_vote msg via commit_msg_tx
        // }
    }
}
