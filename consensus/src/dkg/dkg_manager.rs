// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    dkg_store::DKGStore,
    types::{DKGAggNode, DKGAggNodeAckState, DKGMessage, DKGNodeAckState}, dkg_handler::DKGRpcHandleError,
};
use crate::{dkg::{types::DKGNode, tracing::{observe_dkg, DKGStage}}, util::time_service::TimeService};
use aptos_config::config::SecureBackend;
use aptos_consensus_types::common::Author;
use aptos_dkg::{
    pvss::{traits::Transcript, Player},
    utils::random::random_scalar,
};
use aptos_global_constants::CONSENSUS_KEY;
use aptos_infallible::Mutex;
use aptos_logger::{error, debug};
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_secure_storage::{Storage, KVStorage};
use aptos_types::{
    contract_event::ContractEvent,
    dkg::{DKGPvssConfig, DKGTranscriptWrapper, StartDKGEvent, WTrx},
    epoch_state::EpochState,
};
use futures::future::{AbortHandle, Abortable};
use rand::{rngs::StdRng, thread_rng, SeedableRng};
use std::sync::Arc;
use tokio_retry::strategy::ExponentialBackoff;
use aptos_crypto::{Uniform, bls12381};
use crate::dkg::build_dkg_pvss_config;

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
                error!("[DKG] No DKG manager when calling start_dkg!");
            },
            DKGManagerWrapper::WithDKG(dkg_manager) => {
                dkg_manager.start_dkg(dkg_events);
            },
        }
    }

    pub fn finish_dkg(&self) {
        match self {
            DKGManagerWrapper::NoDKG => {
                error!("[DKG] No DKG manager!");
            },
            DKGManagerWrapper::WithDKG(dkg_manager) => {
                dkg_manager.finish_dkg();
            },
        }
    }

    pub fn ready(&self) -> bool {
        match self {
            DKGManagerWrapper::NoDKG => false,
            DKGManagerWrapper::WithDKG(dkg_manager) => dkg_manager.ready(),
        }
    }

    pub fn take_agg_node(&self) -> Option<DKGAggNode> {
        match self {
            DKGManagerWrapper::NoDKG => None,
            DKGManagerWrapper::WithDKG(dkg_manager) => dkg_manager.take_agg_node(),
        }
    }

    pub fn get_pvss_config(&self) -> Option<DKGPvssConfig> {
        match self {
            DKGManagerWrapper::NoDKG => None,
            DKGManagerWrapper::WithDKG(dkg_manager) => dkg_manager.get_pvss_config(),
        }
    }
}

#[derive(Clone)]
pub struct DKGManager {
    author: Author,
    epoch_state: EpochState,
    reliable_broadcast: Arc<ReliableBroadcast<DKGMessage, ExponentialBackoff>>,
    dkg_store: Arc<Mutex<Option<DKGStore>>>,   // dkg store is shared across threads
    backend: SecureBackend, // for private signing keys
    time_service: Arc<dyn TimeService>, // for metrics
}

impl DKGManager {
    pub fn new(
        author: Author,
        epoch_state: EpochState,
        reliable_broadcast: Arc<ReliableBroadcast<DKGMessage, ExponentialBackoff>>,
        backend: SecureBackend,
        time_service: Arc<dyn TimeService>, // for metrics
    ) -> Self {
        Self {
            author,
            epoch_state,
            reliable_broadcast,
            dkg_store: Arc::new(Mutex::new(None)),
            backend,
            time_service,
        }
    }

    // dkg todo: spawn thread to make this function non-blocking if necessary
    pub fn start_dkg(&self, dkg_events: Vec<ContractEvent>) {
        let event = StartDKGEvent::try_from(dkg_events
            .first().unwrap()).unwrap();

        if event.target_epoch <= self.epoch_state.epoch {
            // do DKG only for future epochs
            return;
        }
        debug!("[DKG] start_dkg with current_epoch={} target_epoch={} at node {}", self.epoch_state.epoch, event.target_epoch, self.author);

        let dkg_pvss_config = build_dkg_pvss_config(self.epoch_state.epoch, &event.target_validator_set);

        // Initialize the DKGStore when the DKG starts
        self.dkg_store.lock().replace(DKGStore::new(
            self.author,
            self.epoch_state.verifier.clone(),
            dkg_pvss_config.clone(),
            self.time_service.get_current_timestamp().as_micros() as u64,
        ));

        let my_index = *self.epoch_state.verifier.address_to_validator_index().get(&self.author).unwrap();

        // get private key as signing key for PVSS
        let backend = &self.backend;
        let storage: Storage = backend.try_into().expect("Unable to initialize storage");
        if let Err(error) = storage.available() {
            panic!("Storage is not available: {:?}", error);
        }
        let private_key: bls12381::PrivateKey = storage
            .get(CONSENSUS_KEY)
            .map(|v| v.value)
            .expect("Unable to get private key");

        let seed = if cfg!(feature = "dkg-test") {
            // In DKG test, the test cases need to get the same input secret, so it can verify the reconstructed dealt secret.
            // See function `verify_dkg_transcript()` in `testsuite/smoke-test/src/dkg/mod.rs`.
            private_key.to_bytes()
        } else {
            let mut rng = thread_rng();
            random_scalar(&mut rng).to_bytes_le()
        };

        let mut rng = StdRng::from_seed(seed);

        // The secret generated by the dealer
        let s = <WTrx as Transcript>::InputSecret::generate(&mut rng);
        // The auxiliary information used for PVSS
        let aux = (self.epoch_state.epoch, self.author);


        // compute one transcript for generating the keys for the randomness generation
        let trx = WTrx::deal(
            &dkg_pvss_config.wconfig,
            &dkg_pvss_config.pp,
            &private_key,
            &dkg_pvss_config.eks,
            &s,
            &aux,
            &Player{ id: my_index },
            &mut rng,
        );

        let dkg_trx_wrapper = DKGTranscriptWrapper { trx };
        let dkg_node = DKGNode::new(self.epoch_state.epoch, self.author, dkg_trx_wrapper);

        debug!("[DKG] Finish computing DKG Node of epoch {:?} at node {}", dkg_node.epoch(), self.author);
        observe_dkg(self.get_start_time(), DKGStage::DKG_NODE_READY);

        // reliable broadcast the dkg node
        self.broadcast_node(dkg_node);
    }

    fn broadcast_node(&self, node: DKGNode) {
        if self.get_rb_abort_handle().is_some() {
            // do not rebroadcast if there is an ongoing broadcast
            return;
        }
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let ack_set = DKGNodeAckState::new(self.epoch_state.verifier.len());
        let task = self.reliable_broadcast.broadcast(node.clone(), ack_set);
        tokio::spawn(Abortable::new(task, abort_registration));
        self.set_rb_abort_handle(Some(abort_handle));
        debug!("[DKG] Node {:?} broadcast DKGNode of epoch {:?}", self.author, node.epoch());
    }

    pub(crate) fn broadcast_agg_node(&self, agg_node: DKGAggNode) {
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let ack_set = DKGAggNodeAckState::new(self.epoch_state.verifier.len());
        let task = self.reliable_broadcast.broadcast(agg_node.clone(), ack_set);

        tokio::spawn(Abortable::new(task, abort_registration));
        // abort the current node broadcast
        if let Some(prev_handle) = self.set_rb_abort_handle(Some(abort_handle)) {
            prev_handle.abort();
        }
        debug!("[DKG] Node {:?} broadcast DKGAggNode of epoch {:?}", self.author, agg_node.epoch());
    }

    pub fn add_node(&self, node: DKGNode) -> anyhow::Result<()> {
        let mut guard = self.dkg_store.lock();
        if guard.is_none() {
            return Err(DKGRpcHandleError::DKGStoreNotInitialized.into());
        }
        if guard.as_mut().unwrap().get_agg_node().is_some() {
            // do not add node if the aggregated node is already available
            return Ok(());
        }
        let maybe_agg_node = guard.as_mut().unwrap().add_node(node);
        drop(guard);

        observe_dkg(self.get_start_time(), DKGStage::DKG_NODES_RECEIVED);

        match maybe_agg_node {
            Ok(agg_node) => {
                observe_dkg(self.get_start_time(), DKGStage::DKG_NODES_VERIFIED_AND_AGGREGATED);

                if let Some(agg_node) = agg_node {
                    self.add_agg_node(agg_node)?;
                }
                Ok(())
            },
            Err(e) => {
                anyhow::bail!("[DKG] Failed to add DKGNode: {:?}", e);
            },
        }
    }

    pub fn add_agg_node(&self, agg_node: DKGAggNode) -> anyhow::Result<()> {
        let mut guard = self.dkg_store.lock();
        if guard.is_none() {
            return Err(DKGRpcHandleError::DKGStoreNotInitialized.into());
        }

        let maybe_agg_node = guard.as_mut().unwrap().add_agg_node(agg_node);
        drop(guard);

        match maybe_agg_node {
            Ok(agg_node) => {
                if let Some(agg_node) = agg_node {
                    observe_dkg(self.get_start_time(), DKGStage::DKG_AGG_NODE_READY);

                    // Broadcast only the first aggregated dkg node
                    self.broadcast_agg_node(agg_node);
                }
                Ok(())
            },
            Err(e) => {
                anyhow::bail!("[DKG] Failed to add DKGAggNode: {:?}", e);
            },
        }
    }

    pub fn ready(&self) -> bool {
        if let Some(dkg_store) = self.dkg_store.lock().as_ref() {
            dkg_store.ready()
        } else {
            false
        }
    }

    // Will be called by the proposal generator
    pub fn take_agg_node(&self) -> Option<DKGAggNode> {
        observe_dkg(self.get_start_time(), DKGStage::DKG_AGG_NODE_PROPOSED);
        if let Some(dkg_store) = self.dkg_store.lock().as_mut() {
            dkg_store.take_agg_node()
        } else {
            unreachable!("[DKG] DKGStore is not initialized!")
        }
    }

    // Will be called by the state computer
    pub fn finish_dkg(&self) {
        if self.dkg_store.lock().is_none() {
            debug!("[RandManager] DKGStore is not initialized when finish_dkg.");
            return;
        }
        observe_dkg(self.get_start_time(), DKGStage::DKG_FINISH);
        // terminate the ongoing broadcast when the DKG aggregated node is committed
        if let Some(handle) = self.set_rb_abort_handle(None) {
            debug!("[DKG] Node {:?} abort broadcast due to DKG finish", self.author);
            handle.abort();
        }
    }

    fn get_start_time(&self) -> Option<u64> {
        if let Some(dkg_store) = self.dkg_store.lock().as_ref() {
            Some(dkg_store.get_start_time())
        } else {
            unreachable!("[DKG] DKGStore is not initialized!")
        }
    }

    fn get_pvss_config(&self) -> Option<DKGPvssConfig> {
        if let Some(dkg_store) = self.dkg_store.lock().as_ref() {
            Some(dkg_store.get_pvss_config().clone())
        } else {
            // It is possible that the DKGStore is not initialized when receiving DKGPayload
            debug!("[DKG] DKGStore is not initialized!");
            None
        }
    }

    fn set_rb_abort_handle(&self, rb_abort_handle: Option<AbortHandle>) -> Option<AbortHandle> {
        if let Some(dkg_store) = self.dkg_store.lock().as_mut() {
            dkg_store.set_rb_abort_handle(rb_abort_handle)
        } else {
            unreachable!("[DKG] DKGStore is not initialized!")
        }
    }

    fn get_rb_abort_handle(&self) -> Option<AbortHandle> {
        if let Some(dkg_store) = self.dkg_store.lock().as_ref() {
            dkg_store.get_rb_abort_handle()
        } else {
            unreachable!("[DKG] DKGStore is not initialized!")
        }
    }
}
