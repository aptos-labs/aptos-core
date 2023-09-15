// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    dkg_rounding::DKGRounding,
    dkg_store::DKGStore,
    types::{DKGAggNode, DKGAggNodeAckState, DKGMessage, DKGNodeAckState},
};
use crate::{dkg::{types::{DKGNode, TDKGMessage}, tracing::{observe_dkg, DKGStage}}, util::time_service::TimeService};
use aptos_config::config::SecureBackend;
use aptos_consensus_types::common::Author;
use aptos_dkg::{
    pvss::{das, traits::Transcript, WeightedTranscript, Player},
    utils::random::random_scalar,
};
use aptos_global_constants::CONSENSUS_KEY;
use aptos_infallible::Mutex;
use aptos_logger::{error, debug};
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_secure_storage::{Storage, KVStorage};
use aptos_types::{
    contract_event::ContractEvent,
    dkg::{DKGPvssConfig, DKGTranscriptWrapper, StartDKGEvent},
    epoch_state::EpochState,
};
use futures::future::{AbortHandle, Abortable};
use rand::{rngs::StdRng, thread_rng, SeedableRng};
use std::sync::Arc;
use tokio_retry::strategy::ExponentialBackoff;
use aptos_crypto::{Uniform, bls12381};
use crate::dkg::build_dkg_pvss_config;

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
    backend: SecureBackend, // for private signing keys
    time_service: Arc<dyn TimeService>, // for metrics
    start_time: u64,
}

impl DKGManager {
    pub fn new(
        author: Author,
        epoch_state: EpochState,
        reliable_broadcast: Arc<ReliableBroadcast<DKGMessage, ExponentialBackoff>>,
        backend: SecureBackend,
        time_service: Arc<dyn TimeService>, // for metrics
    ) -> Self {
        let verifier = epoch_state.verifier.clone();
        Self {
            author,
            epoch_state,
            reliable_broadcast,
            rb_abort_handle: None,
            dkg_store: DKGStore::new(author, verifier),
            dkg_rounding: None,
            backend,
            time_service,
            start_time: 0,
        }
    }

    pub fn start_dkg(&mut self, dkg_events: Vec<ContractEvent>) {
        self.start_time = self.time_service.get_current_timestamp().as_micros() as u64;

        let event = StartDKGEvent::try_from(dkg_events
            .first().unwrap()).unwrap();
        debug!("[DKG] start_dkg, target_epoch={}", event.target_epoch);

        let (dkg_rounding, dkg_pvss_config) = build_dkg_pvss_config(self.epoch_state.epoch, &event.target_validator_set);

        let mut rng = thread_rng();
        let seed = random_scalar(&mut rng);
        let mut rng = StdRng::from_seed(seed.to_bytes_le());


        let s = <WT as Transcript>::InputSecret::generate(&mut rng);
        let aux = (self.epoch_state.epoch, self.author);

        let my_index = *self.epoch_state.verifier.address_to_validator_index().get(&self.author).unwrap();

        // get private key
        let backend = &self.backend;
        let storage: Storage = backend.try_into().expect("Unable to initialize storage");
        if let Err(error) = storage.available() {
            panic!("Storage is not available: {:?}", error);
        }
        let private_key: bls12381::PrivateKey = storage
            .get(CONSENSUS_KEY)
            .map(|v| v.value)
            .expect("Unable to get private key");

        let trx_1 = WT::deal(
            &dkg_pvss_config.wc_1,
            &dkg_pvss_config.pp,
            &private_key,
            &dkg_pvss_config.eks,
            &s,
            &aux,
            &Player{ id: my_index },
            &mut rng,
        );

        let trx_2 = WT::deal(
            &dkg_pvss_config.wc_2,
            &dkg_pvss_config.pp,
            &private_key,
            &dkg_pvss_config.eks,
            &s,
            &aux,
            &Player{ id: my_index },
            &mut rng,
        );

        self.dkg_rounding.replace(dkg_rounding);
        self.dkg_store.add_pvss_config(dkg_pvss_config.clone());

        let dkg_trx_wrapper = DKGTranscriptWrapper {
            trx_one_third: trx_1,
            trx_two_third: trx_2,
        };
        let dkg_node = DKGNode::new(self.epoch_state.epoch, self.author, dkg_trx_wrapper);

        dkg_node.verify(&dkg_pvss_config, &self.epoch_state.verifier).expect("[DKG] Failed to verify own DKG Node");

        debug!("[DKG] Node {:?} finish computing DKG Node of epoch {:?}", self.author, dkg_node.epoch());

        if let Err(e) = self.add_node(dkg_node.clone()) {
            error!("[DKG] Error when adding DKG node: {:?}", e);
        }

        observe_dkg(self.start_time, DKGStage::DKG_NODE_READY);

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
                    observe_dkg(self.start_time, DKGStage::DKG_AGG_NODE_READY);

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
        observe_dkg(self.start_time, DKGStage::DKG_AGG_NODE_PROPOSED);
        self.dkg_store.take_agg_node()
    }

    // Will be called by the state computer
    pub fn finish_dkg(&mut self) {
        observe_dkg(self.start_time, DKGStage::DKG_FINISH);
        // terminate the ongoing broadcast when the DKG aggregated node is committed
        if let Some(handle) = self.rb_abort_handle.take() {
            debug!("[DKG] Node {:?} abort broadcast due to DKG finish", self.author);
            handle.abort();
        }
    }
}
