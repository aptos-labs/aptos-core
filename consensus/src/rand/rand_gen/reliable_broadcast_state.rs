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
use aptos_consensus_types::common::Author;
use aptos_reliable_broadcast::BroadcastStatus;
use aptos_types::{aggregate_signature::PartialSignatures, epoch_state::EpochState};
use std::{collections::HashSet, sync::Arc};
use tokio::sync::mpsc::UnboundedSender;

pub struct AugDataCertBuilder<D> {
    epoch_state: Arc<EpochState>,
    aug_data: AugData<D>,
    partial_signatures: PartialSignatures,
}

impl<D> AugDataCertBuilder<D> {
    pub fn new(aug_data: AugData<D>, epoch_state: Arc<EpochState>) -> Self {
        Self {
            epoch_state,
            aug_data,
            partial_signatures: PartialSignatures::empty(),
        }
    }
}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData>
    BroadcastStatus<RandMessage<S, P, D>, RandMessage<S, P, D>> for AugDataCertBuilder<D>
{
    type Ack = AugDataSignature;
    type Aggregated = CertifiedAugData<D>;
    type Message = AugData<D>;

    fn add(&mut self, peer: Author, ack: Self::Ack) -> anyhow::Result<Option<Self::Aggregated>> {
        ack.verify(peer, &self.epoch_state.verifier, &self.aug_data)?;
        self.partial_signatures
            .add_signature(peer, ack.into_signature());
        Ok(self
            .epoch_state
            .verifier
            .check_voting_power(self.partial_signatures.signatures().keys(), true)
            .ok()
            .map(|_| {
                let aggregated_signature = self
                    .epoch_state
                    .verifier
                    .aggregate_signatures(&self.partial_signatures)
                    .expect("Signature aggregation should succeed");
                CertifiedAugData::new(self.aug_data.clone(), aggregated_signature)
            }))
    }
}

pub struct CertifiedAugDataAckState {
    validators: HashSet<Author>,
}

impl CertifiedAugDataAckState {
    pub fn new(validators: impl Iterator<Item = Author>) -> Self {
        Self {
            validators: validators.collect(),
        }
    }
}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData>
    BroadcastStatus<RandMessage<S, P, D>, RandMessage<S, P, D>> for CertifiedAugDataAckState
{
    type Ack = CertifiedAugDataAck;
    type Aggregated = ();
    type Message = CertifiedAugData<D>;

    fn add(&mut self, peer: Author, _ack: Self::Ack) -> anyhow::Result<Option<Self::Aggregated>> {
        ensure!(
            self.validators.remove(&peer),
            "[RandMessage] Unknown author: {}",
            peer
        );
        // If receive from all validators, stop the reliable broadcast
        if self.validators.is_empty() {
            Ok(Some(()))
        } else {
            Ok(None)
        }
    }
}

pub struct ShareAckState<P> {
    validators: HashSet<Author>,
    rand_config: RandConfig,
    decision_tx: UnboundedSender<RandDecision<P>>,
}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData>
    BroadcastStatus<RandMessage<S, P, D>, RandMessage<S, P, D>> for ShareAckState<P>
{
    type Ack = ShareAck<P>;
    type Aggregated = ();
    type Message = RandShare<S>;

    fn add(&mut self, peer: Author, ack: Self::Ack) -> anyhow::Result<Option<Self::Aggregated>> {
        ensure!(
            self.validators.remove(&peer),
            "[RandMessage] Unknown author: {}",
            peer
        );
        // If receive a decision, verify it and send it to the randomness manager and stop the reliable broadcast
        if let Some(decision) = ack.into_maybe_decision() {
            if decision.verify(&self.rand_config).is_ok() {
                let _ = self.decision_tx.send(decision);
                return Ok(Some(()));
            }
        }
        // If receive from all validators, stop the reliable broadcast
        if self.validators.is_empty() {
            Ok(Some(()))
        } else {
            Ok(None)
        }
    }
}
