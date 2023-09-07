// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    dkg_rounding::DKGRounding,
    dkg_store::DKGStore,
    types::{DKGAggNode, DKGAggNodeAckState, DKGMessage, DKGNodeAckState},
};
use crate::dkg::types::DKGNode;
use aptos_consensus_types::common::Author;
use aptos_dkg::{
    pvss::{das, traits::Transcript, WeightedTranscript},
    utils::random::random_scalar,
};
use aptos_infallible::Mutex;
use aptos_logger::{error, debug};
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_types::{
    contract_event::ContractEvent,
    dkg::{DKGPvssConfig, DKGTranscriptWrapper, StartDKGEvent},
    epoch_state::EpochState,
};
use futures::future::{AbortHandle, Abortable};
use rand::{rngs::StdRng, thread_rng, SeedableRng};
use std::sync::Arc;
use tokio_retry::strategy::ExponentialBackoff;
use aptos_crypto::Uniform;

// the transcript size is 3.25MB
// const TRANSCRIPT_SIZE: usize = 3_250_000;
// const TRANSCRIPT_COMPUTE_TIME_MS: u64 = 4760;
// const TRANSCRIPT_VERIFY_TIME_MS: u64 = 555;
// const TRANSCRIPT_AGGREGATE_TIME_MS: u64 = 21;

type WT = WeightedTranscript<das::Transcript>;

pub enum DKGManagerWrapper {
    #[allow(dead_code)]
    NoDKG,
    WithDKG(Arc<Mutex<DKGManager>>),
}

impl DKGManagerWrapper {
    #[allow(dead_code)]
    pub fn default() -> Self {
        DKGManagerWrapper::NoDKG
    }

    pub async fn start_dkg(&self, dkg_events: Vec<ContractEvent>) {
        match self {
            DKGManagerWrapper::NoDKG => {
                debug!("[DKG] start_dkg: DKGManagerWrapper::NoDKG!");
                error!("[DKG] No DKG manager!");
            },
            DKGManagerWrapper::WithDKG(dkg_manager) => {
                debug!("[DKG] start_dkg: DKGManagerWrapper::WithDKG!");
                let dkg_manager_clone = dkg_manager.clone();
                let mut guard = dkg_manager_clone.lock();
                debug!("[DKG] start_dkg: dkg_manager lock acquired");
                guard.start_dkg(dkg_events);
            },
        }
    }

    pub fn finish_dkg(&self) {
        match self {
            DKGManagerWrapper::NoDKG => {
                error!("[DKG] No DKG manager!");
            },
            DKGManagerWrapper::WithDKG(dkg_manager) => {
                let dkg_manager_clone = dkg_manager.clone();
                let mut guard = dkg_manager_clone.lock();
                guard.finish_dkg();
            },
        }
    }

    pub fn ready(&self) -> bool {
        match self {
            DKGManagerWrapper::NoDKG => false,
            DKGManagerWrapper::WithDKG(dkg_manager) => dkg_manager.lock().ready(),
        }
    }

    pub fn take_agg_node(&self) -> Option<DKGAggNode> {
        match self {
            DKGManagerWrapper::NoDKG => None,
            DKGManagerWrapper::WithDKG(dkg_manager) => dkg_manager.lock().take_agg_node(),
        }
    }

    pub fn get_pvss_config(&self) -> Option<DKGPvssConfig> {
        match self {
            DKGManagerWrapper::NoDKG => None,
            DKGManagerWrapper::WithDKG(dkg_manager) => dkg_manager.lock().dkg_store.get_pvss_config(),
        }
    }
}

pub struct DKGManager {
    author: Author,
    epoch_state: EpochState,
    reliable_broadcast: Arc<ReliableBroadcast<DKGMessage, ExponentialBackoff>>,
    rb_abort_handle: Option<AbortHandle>,
    dkg_store: DKGStore,
    dkg_rounding: Option<DKGRounding>,
}

impl DKGManager {
    pub fn new(
        author: Author,
        epoch_state: EpochState,
        reliable_broadcast: Arc<ReliableBroadcast<DKGMessage, ExponentialBackoff>>,
    ) -> Self {
        let verifier = epoch_state.verifier.clone();
        Self {
            author,
            epoch_state,
            reliable_broadcast,
            rb_abort_handle: None,
            dkg_store: DKGStore::new(author, verifier),
            dkg_rounding: None,
        }
    }

    pub fn start_dkg(&mut self, dkg_events: Vec<ContractEvent>) {
        // thread::sleep(Duration::from_millis(TRANSCRIPT_COMPUTE_TIME_MS));
        debug!("[DKG] start_dkg: liveness check 1");
        let event = StartDKGEvent::try_from(dkg_events
            .first().unwrap()).unwrap();
        debug!("[DKG] start_dkg: first_event={:?}", event);

        let validator_info = event.locked_new_validator_info;
        let validator_addresses = validator_info.iter().map(|vi| vi.account_address).collect();
        let validator_stakes: Vec<u64> = validator_info
            .iter()
            .map(|vi| vi.consensus_voting_power())
            .collect();
        let validator_consensus_keys = validator_info
            .iter()
            .map(|vi| vi.consensus_public_key().clone())
            .collect();

        let dkg_rounding = DKGRounding::new(validator_addresses, validator_stakes, validator_consensus_keys);
        debug!(
            "[DKG] Starting DKG with the following parameters: number of validators: {:?}, validator stakes: {:?}, validator weights: {:?}, validator 1/3 weights: {:?}, validator 2/3 weights: {:?}",
            dkg_rounding.validator_stakes().len(),
            dkg_rounding.validator_stakes(),
            dkg_rounding.validator_weights(),
            dkg_rounding.weighted_config_1().get_threshold_weight(),
            dkg_rounding.weighted_config_2().get_threshold_weight(),
        );

        // dkg todo: decide whether to use consensus key as encryption key
        let consensus_keys: Vec<<das::Transcript as Transcript>::EncryptPubKey> = dkg_rounding.validator_consensus_keys().iter().map(|k| k.to_bytes().as_slice().try_into().unwrap()).collect::<Vec<_>>();
        let wc_1 = dkg_rounding.weighted_config_1().clone();
        let wc_2 = dkg_rounding.weighted_config_2().clone();
        self.dkg_rounding.replace(dkg_rounding);

        let mut rng = thread_rng();
        let seed = random_scalar(&mut rng);
        let mut rng = StdRng::from_seed(seed.to_bytes_le());

        let pp = <WT as Transcript>::PvssPublicParameters::default();
        let s = <WT as Transcript>::InputSecret::generate(&mut rng);

        let trx_1 = WT::deal(
            &wc_1,
            &pp,
            &consensus_keys,
            &s,
            &mut rng,
        );
        trx_1
            .verify(&wc_1, &pp, &consensus_keys)
            .expect("PVSS transcript failed verification");

        // // Test transcript (de)serialization
        // let serialized = trx_1.to_bytes();
        // let deserialized = WT::try_from(serialized.as_slice())
        //     .expect("serialized transcript should deserialize correctly");
        // assert_eq!(trx_1, deserialized);

        let trx_2 = WT::deal(&wc_2, &pp, &consensus_keys, &s, &mut rng);
        trx_2
            .verify(&wc_2, &pp, &consensus_keys)
            .expect("PVSS transcript failed verification");

        // // Test transcript (de)serialization
        // let serialized = trx_2.to_bytes();
        // let deserialized = WT::try_from(serialized.as_slice())
        //     .expect("serialized transcript should deserialize correctly");
        // assert_eq!(trx_2, deserialized);

        let dkg_pvss_config = DKGPvssConfig::new(wc_1.clone(), wc_2.clone(), pp, consensus_keys);
        self.dkg_store.add_pvss_config(dkg_pvss_config);

        let dkg_trx_wrapper = DKGTranscriptWrapper {
            trx_one_third: trx_1,
            trx_two_third: trx_2,
        };
        let dkg_node = DKGNode::new(self.epoch_state.epoch, self.author, dkg_trx_wrapper);

        debug!("[DKG] Node {:?} finish computing DKG Node of epoch {:?}", self.author, dkg_node.epoch());

        if let Err(e) = self.add_node(dkg_node.clone()) {
            error!("[DKG] Error when adding DKG node: {:?}", e);
        }
        self.broadcast_node(dkg_node);
    }

    fn broadcast_node(&mut self, node: DKGNode) {
        if self.rb_abort_handle.is_some() {
            // do not rebroadcast if there is an ongoing broadcast
            return;
        }
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let ack_set = DKGNodeAckState::new(self.epoch_state.verifier.len());
        let task = self.reliable_broadcast.broadcast(node.clone(), ack_set);
        tokio::spawn(Abortable::new(task, abort_registration));
        self.rb_abort_handle.replace(abort_handle);
        debug!("[DKG] Node {:?} broadcast DKG Node of epoch {:?}", self.author, node.epoch());
    }

    pub(crate) fn broadcast_agg_node(&mut self, agg_node: DKGAggNode) {
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let ack_set = DKGAggNodeAckState::new(self.epoch_state.verifier.len());
        let task = self.reliable_broadcast.broadcast(agg_node.clone(), ack_set);

        tokio::spawn(Abortable::new(task, abort_registration));
        // abort the current node broadcast
        // no concurrent agg_node broadcast guaranteed by OnceCell
        if let Some(prev_handle) = self.rb_abort_handle.replace(abort_handle) {
            prev_handle.abort();
        }
        debug!("[DKG] Node {:?} broadcast DKG Aggregated Node of epoch {:?}", self.author, agg_node.epoch());
    }

    pub fn add_node(&mut self, node: DKGNode) -> anyhow::Result<()> {
        match self.dkg_store.add_node(node) {
            Ok(agg_node) => {
                if let Some(agg_node) = agg_node {
                    self.add_agg_node(agg_node)?;
                }
                Ok(())
            },
            Err(e) => {
                anyhow::bail!("[DKG] Failed to add DKG node: {:?}", e);
            },
        }
    }

    pub fn add_agg_node(&mut self, agg_node: DKGAggNode) -> anyhow::Result<()> {
        match self.dkg_store.add_agg_node(agg_node) {
            Ok(agg_node) => {
                if let Some(agg_node) = agg_node {
                    // Broadcast only the first aggregated dkg node
                    self.broadcast_agg_node(agg_node);
                }
                Ok(())
            },
            Err(e) => {
                anyhow::bail!("[DKG] Failed to add DKG aggregated node: {:?}", e);
            },
        }
    }

    pub fn ready(&self) -> bool {
        self.dkg_store.ready()
    }

    // Will be called by the proposal generator
    pub fn take_agg_node(&mut self) -> Option<DKGAggNode> {
        self.dkg_store.take_agg_node()
    }

    // Will be called by the state computer
    pub fn finish_dkg(&mut self) {
        // terminate the ongoing broadcast when the DKG aggregated node is committed
        if let Some(handle) = self.rb_abort_handle.take() {
            debug!("[DKG] Node {:?} abort broadcast due to DKG finish", self.author);
            handle.abort();
        }
    }
}
