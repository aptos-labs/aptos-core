// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::rand::rand_gen::{
    network_messages::RandMessage,
    types::{
        AugData, AugDataSignature, AugmentedData, CertifiedAugData, CertifiedAugDataAck, Proof,
        RandConfig, RandDecision, RandShare, Share, ShareAck,
    },
};
use anyhow::ensure;
use aptos_consensus_types::{common::Author, randomness::RandMetadata};
use aptos_infallible::Mutex;
use aptos_logger::error;
use aptos_reliable_broadcast::BroadcastStatus;
use aptos_types::{aggregate_signature::PartialSignatures, epoch_state::EpochState};
use std::{collections::HashSet, sync::Arc};
use tokio::sync::mpsc::UnboundedSender;

pub struct AugDataCertBuilder<D> {
    epoch_state: Arc<EpochState>,
    aug_data: AugData<D>,
    partial_signatures: Mutex<PartialSignatures>,
}

impl<D> AugDataCertBuilder<D> {
    pub fn new(aug_data: AugData<D>, epoch_state: Arc<EpochState>) -> Arc<Self> {
        Arc::new(Self {
            epoch_state,
            aug_data,
            partial_signatures: Mutex::new(PartialSignatures::empty()),
        })
    }
}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData>
    BroadcastStatus<RandMessage<S, P, D>, RandMessage<S, P, D>> for Arc<AugDataCertBuilder<D>>
{
    type Ack = AugDataSignature;
    type Aggregated = CertifiedAugData<D>;
    type Message = AugData<D>;

    fn add(&self, peer: Author, ack: Self::Ack) -> anyhow::Result<Option<Self::Aggregated>> {
        ack.verify(peer, &self.epoch_state.verifier, &self.aug_data)?;
        let mut parital_signatures_guard = self.partial_signatures.lock();
        parital_signatures_guard.add_signature(peer, ack.into_signature());
        Ok(self
            .epoch_state
            .verifier
            .check_voting_power(parital_signatures_guard.signatures().keys(), true)
            .ok()
            .map(|_| {
                let aggregated_signature = self
                    .epoch_state
                    .verifier
                    .aggregate_signatures(&parital_signatures_guard)
                    .expect("Signature aggregation should succeed");
                CertifiedAugData::new(self.aug_data.clone(), aggregated_signature)
            }))
    }
}

pub struct CertifiedAugDataAckState {
    validators: Mutex<HashSet<Author>>,
}

impl CertifiedAugDataAckState {
    pub fn new(validators: impl Iterator<Item = Author>) -> Self {
        Self {
            validators: Mutex::new(validators.collect()),
        }
    }
}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData>
    BroadcastStatus<RandMessage<S, P, D>, RandMessage<S, P, D>> for Arc<CertifiedAugDataAckState>
{
    type Ack = CertifiedAugDataAck;
    type Aggregated = ();
    type Message = CertifiedAugData<D>;

    fn add(&self, peer: Author, _ack: Self::Ack) -> anyhow::Result<Option<Self::Aggregated>> {
        let mut validators_guard = self.validators.lock();
        ensure!(
            validators_guard.remove(&peer),
            "[RandMessage] Unknown author: {}",
            peer
        );
        // If receive from all validators, stop the reliable broadcast
        if validators_guard.is_empty() {
            Ok(Some(()))
        } else {
            Ok(None)
        }
    }
}

pub struct ShareAckState<P> {
    rand_metadata: RandMetadata,
    validators: Mutex<HashSet<Author>>,
    rand_config: RandConfig,
    decision_tx: UnboundedSender<RandDecision<P>>,
}

impl<P> ShareAckState<P> {
    pub fn new(
        validators: impl Iterator<Item = Author>,
        metadata: RandMetadata,
        rand_config: RandConfig,
        decision_tx: UnboundedSender<RandDecision<P>>,
    ) -> Self {
        Self {
            validators: Mutex::new(validators.collect()),
            rand_metadata: metadata,
            rand_config,
            decision_tx,
        }
    }
}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData>
    BroadcastStatus<RandMessage<S, P, D>, RandMessage<S, P, D>> for Arc<ShareAckState<P>>
{
    type Ack = ShareAck<P>;
    type Aggregated = ();
    type Message = RandShare<S>;

    fn add(&self, peer: Author, ack: Self::Ack) -> anyhow::Result<Option<Self::Aggregated>> {
        ensure!(
            self.validators.lock().remove(&peer),
            "[RandMessage] Unknown author: {}",
            peer
        );
        // If receive a decision, verify it and send it to the randomness manager and stop the reliable broadcast
        if let Some(decision) = ack.into_maybe_decision() {
            match decision.verify(&self.rand_config, &self.rand_metadata) {
                Ok(_) => {
                    let _ = self.decision_tx.send(decision);
                    return Ok(Some(()));
                },
                Err(e) => error!("[RandManager] Failed to verify decision: {}", e),
            }
        }
        // If receive from all validators, stop the reliable broadcast
        if self.validators.lock().is_empty() {
            Ok(Some(()))
        } else {
            Ok(None)
        }
    }
}
