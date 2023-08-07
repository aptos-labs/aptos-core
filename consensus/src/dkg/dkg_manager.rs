// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, sync::Arc, thread, time::Duration};
use aptos_consensus_types::common::Author;
use aptos_infallible::Mutex;
use aptos_types::epoch_state::EpochState;
use crate::dkg::types::DKGNode;
use futures::future::{AbortHandle, Abortable};
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_logger::error;
use tokio_retry::strategy::ExponentialBackoff;
use aptos_types::dkg::StartDKGEvent;

use super::{types::{DKGAggNode, DKGNodeAckState, DKGAggNodeAckState, DKGMessage}, dkg_store::DKGStore};

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

impl From<StartDKGEvent> for StakeDis {
    fn from(value: StartDKGEvent) -> Self {
        let distribution: HashMap<Author, u64> = value.locked_new_validator_set.into_iter().map(|vi|(vi.account_address, vi.consensus_voting_power())).collect();
        Self {
            distribution,
        }
    }
}

pub enum DKGManagerWrapper {
    NoDKG,
    WithDKG(DKGManager),
}

impl DKGManagerWrapper {
    pub async fn start_dkg(&self, _stake_dis: Option<StakeDis>) {
        match self {
            DKGManagerWrapper::NoDKG => {
                error!("[DKG] No DKG manager!");
            }
            DKGManagerWrapper::WithDKG(dkg_manager) => {
                dkg_manager.start_dkg(_stake_dis).await;
            }
        }
    }

    pub fn take_agg_node(&self) -> Option<DKGAggNode> {
        match self {
            DKGManagerWrapper::NoDKG => None,
            DKGManagerWrapper::WithDKG(dkg_manager) => dkg_manager.take_agg_node(),
        }
    }
}

#[derive(Clone)]
pub struct DKGManager {
    author: Author,
    epoch_state: Arc<EpochState>,
    reliable_broadcast: Arc<ReliableBroadcast<DKGMessage, ExponentialBackoff>>,
    rb_abort_handle: Arc<Mutex<Option<AbortHandle>>>,
    dkg_store: Arc<Mutex<DKGStore>>,
}

impl DKGManager {
    pub fn new(author: Author, epoch_state: Arc<EpochState>, reliable_broadcast: Arc<ReliableBroadcast<DKGMessage, ExponentialBackoff>>) -> Self {
        Self {
            author,
            epoch_state,
            reliable_broadcast,
            rb_abort_handle: Arc::new(Mutex::new(None)),
            dkg_store: Arc::new(Mutex::new(DKGStore::new())),
        }
    }

    pub async fn start_dkg(&self, _stake_dis: Option<StakeDis>) {
        // dkg todo: compute pvss transcript and create a DKG node
        thread::sleep(Duration::from_millis(TRANSCRIPT_COMPUTE_TIME_MS));
        // self.broadcast_node(node);
    }

    fn broadcast_node(&self, node: DKGNode) {
        if self.rb_abort_handle.lock().is_some() {
            // do not rebroadcast if there is an ongoing broadcast
            return;
        }
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let ack_set = DKGNodeAckState::new(self.epoch_state.verifier.len());
        let task = self
            .reliable_broadcast
            .broadcast(node.clone(), ack_set);
        tokio::spawn(Abortable::new(task, abort_registration));
        self.rb_abort_handle.lock().replace(abort_handle);
    }

    pub(crate) fn broadcast_agg_node(&self, agg_node: DKGAggNode) {
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let ack_set = DKGAggNodeAckState::new(self.epoch_state.verifier.len());
        let task = self
            .reliable_broadcast
            .broadcast(agg_node.clone(), ack_set);
        tokio::spawn(Abortable::new(task, abort_registration));
        // abort the current node broadcast
        // no concurrent agg_node broadcast guaranteed by OnceCell
        if let Some(prev_handle) = self.rb_abort_handle.lock().replace(abort_handle) {
            prev_handle.abort();
        }
        // dkg todo: abort the broadcast when DKG is done
    }

    pub fn add_node(&self, node: DKGNode) {
        match self.dkg_store.lock().add_node(node, &self.epoch_state.verifier) {
            Ok(agg_node) => {
                if let Some(agg_node) = agg_node {
                    self.add_agg_node(agg_node);
                }
            }
            Err(e) => {
                error!("[DKG] Failed to add DKG node: {:?}", e);
            }
        }
    }

    pub fn add_agg_node(&self, agg_node: DKGAggNode) {
        match self.dkg_store.lock().add_agg_node(agg_node, &self.epoch_state.verifier) {
            Ok(agg_node) => {
                if let Some(agg_node) = agg_node {
                    self.broadcast_agg_node(agg_node);
                }
            }
            Err(e) => {
                error!("[DKG] Failed to add DKG aggregated node: {:?}", e);
            }
        }
    }

    pub fn take_agg_node(&self) -> Option<DKGAggNode> {
        self.dkg_store.lock().take_agg_node()
    }
}
