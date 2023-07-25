// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, sync::Arc, thread, time::Duration};
use aptos_consensus_types::common::Author;
use aptos_crypto::bls12381::Signature;
use aptos_logger::info;
use aptos_types::{transaction::SignedTransaction, validator_verifier::ValidatorVerifier, epoch_state::EpochState};
use crate::dkg::types::{DKGNodeMetadata, DKGNode};
use serde::Serialize;
use tokio::{sync::{oneshot, mpsc}, time::Interval};
use crate::{
    quorum_store::batch_generator::BatchGeneratorCommand, block_storage::BlockReader,
};
use aptos_dkg::pvss::scrape::Transcript;
use futures::{
    future::{AbortHandle, Abortable},
    FutureExt,
};

use super::{dkg_store::DKGStore, types::{DKGAggNode, DKGNodeAckState, DKGAggNodeAckState}, dkg_reliable_broadcast::ReliableBroadcast};

// the transcript size is 3.25MB
const TRANSCRIPT_SIZE: usize = 3_250_000;
const TRANSCRIPT_COMPUTE_TIME_MS: u64 = 4760;
const TRANSCRIPT_VERIFY_TIME_MS: u64 = 555;
const TRANSCRIPT_AGGREGATE_TIME_MS: u64 = 21;

// dkg todo: use the same format for stake distribution as in PVSS library
#[derive(Debug)]
pub struct StakeDis {
    pub distribution: HashMap<Author, u64>,
}

#[derive(Debug)]
pub enum DKGManagerMessage {
    DKGReady(DKGAggNode),
}

#[derive(Clone)]
pub struct DKGManager {
    author: Author,
    epoch_state: Arc<EpochState>,
    // dkg todo: send the aggregated dkg node to proposal generator
    // Channel to send the aggregated dkg node to proposal generator
    proposal_generator_tx: mpsc::Sender<DKGManagerMessage>,
    reliable_broadcast: Arc<ReliableBroadcast>,
    rb_abort_handle: Option<AbortHandle>,
}

impl DKGManager {
    pub fn new(author: Author, epoch_state: Arc<EpochState>, proposal_generator_tx: mpsc::Sender<DKGManagerMessage>, reliable_broadcast: Arc<ReliableBroadcast>) -> Self {
        Self {
            author,
            epoch_state,
            proposal_generator_tx,
            reliable_broadcast,
            rb_abort_handle: None,
        }
    }

    pub fn start_dkg(&mut self, stake_dis: StakeDis) {
        // dkg todo: compute pvss transcript and create a DKG node
        thread::sleep(Duration::from_millis(TRANSCRIPT_COMPUTE_TIME_MS));
        // self.broadcast_node(node);
    }

    fn broadcast_node(&mut self, node: DKGNode) {
        if self.rb_abort_handle.is_some() {
            // do not rebroadcast if there is an ongoing broadcast
            return;
        }
        let rb = self.reliable_broadcast.clone();
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let ack_set = DKGNodeAckState::new(self.epoch_state.verifier.len());
        let task = self
            .reliable_broadcast
            .broadcast(node.clone(), ack_set);
        tokio::spawn(Abortable::new(task, abort_registration));
        self.rb_abort_handle.replace(abort_handle);
    }

    pub(crate) fn broadcast_agg_node(&mut self, agg_node: DKGAggNode) {
        let rb = self.reliable_broadcast.clone();
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let ack_set = DKGAggNodeAckState::new(self.epoch_state.verifier.len());
        let task = self
            .reliable_broadcast
            .broadcast(agg_node.clone(), ack_set);
        tokio::spawn(Abortable::new(task, abort_registration));
        // abort the current node broadcast
        // no concurrent agg_node broadcast guaranteed by OnceCell
        if let Some(prev_handle) = self.rb_abort_handle.replace(abort_handle) {
            prev_handle.abort();
        }
        // dkg todo: abort the broadcast when DKG is done
    }
}
