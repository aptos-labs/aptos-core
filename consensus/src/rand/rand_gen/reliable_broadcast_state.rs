// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::rand::rand_gen::{
    network_messages::RandMessage,
    types::{AugData, AugDataSignature, AugmentedData, CertifiedAugData, Proof, Share},
};
use aptos_consensus_types::common::Author;
use aptos_reliable_broadcast::BroadcastStatus;
use aptos_types::{aggregate_signature::PartialSignatures, epoch_state::EpochState};
use std::sync::Arc;

pub struct SignatureBuilder<D> {
    epoch_state: Arc<EpochState>,
    aug_data: AugData<D>,
    partial_signatures: PartialSignatures,
}

impl<D> SignatureBuilder<D> {
    pub fn new(aug_data: AugData<D>, epoch_state: Arc<EpochState>) -> Self {
        Self {
            epoch_state,
            aug_data,
            partial_signatures: PartialSignatures::empty(),
        }
    }
}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData>
    BroadcastStatus<RandMessage<S, P, D>, RandMessage<S, P, D>> for SignatureBuilder<D>
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
