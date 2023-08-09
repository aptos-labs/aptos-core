// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;
use aptos_consensus_types::common::Author;
use aptos_dkg::{utils::random::random_scalar, pvss::{test_utils, das, WeightedTranscript, traits::Transcript}, constants::DST_PVSS_TESTING_APP};
use aptos_infallible::Mutex;
use aptos_types::{epoch_state::EpochState, contract_event::ContractEvent, dkg::{DKGTranscriptWrapper, DKGPvssConfig}};
use rand::{rngs::StdRng, SeedableRng, thread_rng};
use crate::dkg::types::DKGNode;
use futures::future::{AbortHandle, Abortable};
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_logger::error;
use tokio_retry::strategy::ExponentialBackoff;
use aptos_types::dkg::StartDKGEvent;

use super::{types::{DKGAggNode, DKGNodeAckState, DKGAggNodeAckState, DKGMessage, TDKGMessage}, dkg_store::DKGStore, dkg_rounding::DKGRounding};

// the transcript size is 3.25MB
// const TRANSCRIPT_SIZE: usize = 3_250_000;
// const TRANSCRIPT_COMPUTE_TIME_MS: u64 = 4760;
// const TRANSCRIPT_VERIFY_TIME_MS: u64 = 555;
// const TRANSCRIPT_AGGREGATE_TIME_MS: u64 = 21;

type WT = WeightedTranscript<das::Transcript>;

pub enum DKGManagerWrapper {
    #[allow(dead_code)]
    NoDKG,
    WithDKG(DKGManager),
}

impl DKGManagerWrapper {
    #[allow(dead_code)]
    pub fn default() -> Self {
        DKGManagerWrapper::NoDKG
    }

    pub async fn start_dkg(&self, dkg_events: Vec<ContractEvent>) {
        match self {
            DKGManagerWrapper::NoDKG => {
                error!("[DKG] No DKG manager!");
            }
            DKGManagerWrapper::WithDKG(dkg_manager) => {
                dkg_manager.start_dkg(dkg_events).await;
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
    dkg_rounding: Arc<Mutex<Option<DKGRounding>>>,
    dkg_pvss_config: Arc<Mutex<Option<DKGPvssConfig>>>,
}

impl DKGManager {
    pub fn new(author: Author, epoch_state: Arc<EpochState>, reliable_broadcast: Arc<ReliableBroadcast<DKGMessage, ExponentialBackoff>>) -> Self {
        Self {
            author,
            epoch_state,
            reliable_broadcast,
            rb_abort_handle: Arc::new(Mutex::new(None)),
            dkg_store: Arc::new(Mutex::new(DKGStore::new(author))),
            dkg_rounding: Arc::new(Mutex::new(None)),
            dkg_pvss_config: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn start_dkg(&self, dkg_events: Vec<ContractEvent>) {
        // thread::sleep(Duration::from_millis(TRANSCRIPT_COMPUTE_TIME_MS));

        let dkg_rounding: DKGRounding = dkg_events.first().map(|e|StartDKGEvent::try_from(e).expect("[DKG]: Empty DKG events!").into()).expect("[DKG]: Convertion from DKG events to DKG Rounding failed!");

        // let consensus_keys = dkg_rounding.validator_consensus_keys().clone();
        let wc_1 = dkg_rounding.weighted_config_1().clone();
        let wc_2 = dkg_rounding.weighted_config_2().clone();
        self.dkg_rounding.lock().replace(dkg_rounding);

        let mut rng = thread_rng();
        let seed = random_scalar(&mut rng);
        let mut rng = StdRng::from_seed(seed.to_bytes_le());

        // dkg todo: generate these parameters
        // dkg todo: use real encryption keys of the new validators
        let (pp, _dks, eks, s, _sk) = test_utils::setup_dealing::<WT, StdRng>(&wc_1, &mut rng);

        let trx_1 = WT::deal(&wc_1, &pp, &eks, s.clone(), &DST_PVSS_TESTING_APP[..], &mut rng);
        trx_1.verify(&wc_1, &pp, &eks, &DST_PVSS_TESTING_APP[..])
            .expect("PVSS transcript failed verification");

        // // Test transcript (de)serialization
        // let serialized = trx_1.to_bytes();
        // let deserialized = WT::try_from(serialized.as_slice())
        //     .expect("serialized transcript should deserialize correctly");
        // assert_eq!(trx_1, deserialized);

        let trx_2 = WT::deal(&wc_2, &pp, &eks, s, &DST_PVSS_TESTING_APP[..], &mut rng);
        trx_2.verify(&wc_2, &pp, &eks, &DST_PVSS_TESTING_APP[..])
            .expect("PVSS transcript failed verification");

        // // Test transcript (de)serialization
        // let serialized = trx_2.to_bytes();
        // let deserialized = WT::try_from(serialized.as_slice())
        //     .expect("serialized transcript should deserialize correctly");
        // assert_eq!(trx_2, deserialized);

        let dkg_pvss_config = DKGPvssConfig::new(wc_1, wc_2, pp, eks, &DST_PVSS_TESTING_APP[..]);
        self.dkg_pvss_config.lock().replace(dkg_pvss_config);

        let dkg_trx_wrapper = DKGTranscriptWrapper {trx_one_third: trx_1, trx_two_third: trx_2};
        let dkg_node = DKGNode::new(self.epoch_state.epoch, self.author, dkg_trx_wrapper);
        self.broadcast_node(dkg_node);
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

    pub fn add_node(&self, node: DKGNode) -> anyhow::Result<()> {
        if self.dkg_pvss_config.lock().is_none() {
            // dkg todo: think more about what to do in this case
            anyhow::bail!("[DKG] DKG PVSS config is not ready!");
        }
        if node.verify(self.dkg_pvss_config.lock().as_ref().unwrap()).is_ok() {
            match self.dkg_store.lock().add_node(node, &self.epoch_state.verifier, self.dkg_pvss_config.lock().as_ref().unwrap()) {
                Ok(agg_node) => {
                    if let Some(agg_node) = agg_node {
                        self.add_agg_node(agg_node)?;
                    }
                    return Ok(());
                }
                Err(e) => {
                    anyhow::bail!("[DKG] Failed to add DKG node: {:?}", e);
                }
            }
        } else {
            anyhow::bail!("[DKG] Failed to verify DKG node: {:?}", node);
        }
    }

    pub fn add_agg_node(&self, agg_node: DKGAggNode) -> anyhow::Result<()> {
        if self.dkg_pvss_config.lock().is_none() {
            anyhow::bail!("[DKG] DKG PVSS config is not ready!");
        }
        if agg_node.verify(self.dkg_pvss_config.lock().as_ref().unwrap()).is_ok() {
            match self.dkg_store.lock().add_agg_node(agg_node) {
                Ok(agg_node) => {
                    if let Some(agg_node) = agg_node {
                        // Broadcast only the first aggregated dkg node
                        self.broadcast_agg_node(agg_node);
                    }
                    return Ok(());
                }
                Err(e) => {
                    anyhow::bail!("[DKG] Failed to add DKG aggregated node: {:?}", e);
                }
            }
        } else {
            anyhow::bail!("[DKG] Failed to verify DKG aggregated node: {:?}", agg_node);
        }
    }

    // Will be called by the proposal generator
    pub fn take_agg_node(&self) -> Option<DKGAggNode> {
        self.dkg_store.lock().take_agg_node()
    }
}
