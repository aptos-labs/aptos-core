// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::rand::rand_gen::{
    network_messages::RandMessage,
    rand_store::RandStore,
    types::{
        AugData, AugDataSignature, AugmentedData, CertifiedAugData, CertifiedAugDataAck,
        RandConfig, RandShare, RequestShare, Share,
    },
};
use anyhow::ensure;
use aptos_consensus_types::{common::Author, randomness::RandMetadata};
use aptos_infallible::Mutex;
use aptos_reliable_broadcast::BroadcastStatus;
use aptos_types::{aggregate_signature::PartialSignatures, epoch_state::EpochState};
use std::{collections::HashSet, sync::Arc};

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

impl<S: Share, D: AugmentedData> BroadcastStatus<RandMessage<S, D>, RandMessage<S, D>>
    for Arc<AugDataCertBuilder<D>>
{
    type Aggregated = CertifiedAugData<D>;
    type Message = AugData<D>;
    type Response = AugDataSignature;

    fn add(&self, peer: Author, ack: Self::Response) -> anyhow::Result<Option<Self::Aggregated>> {
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

impl<S: Share, D: AugmentedData> BroadcastStatus<RandMessage<S, D>, RandMessage<S, D>>
    for Arc<CertifiedAugDataAckState>
{
    type Aggregated = ();
    type Message = CertifiedAugData<D>;
    type Response = CertifiedAugDataAck;

    fn add(&self, peer: Author, _ack: Self::Response) -> anyhow::Result<Option<Self::Aggregated>> {
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

pub struct ShareAggregateState<S> {
    rand_metadata: RandMetadata,
    rand_store: Arc<Mutex<RandStore<S>>>,
    rand_config: RandConfig,
}

impl<S> ShareAggregateState<S> {
    pub fn new(
        rand_store: Arc<Mutex<RandStore<S>>>,
        metadata: RandMetadata,
        rand_config: RandConfig,
    ) -> Self {
        Self {
            rand_store,
            rand_metadata: metadata,
            rand_config,
        }
    }
}

impl<S: Share, D: AugmentedData> BroadcastStatus<RandMessage<S, D>, RandMessage<S, D>>
    for Arc<ShareAggregateState<S>>
{
    type Aggregated = ();
    type Message = RequestShare;
    type Response = RandShare<S>;

    fn add(&self, peer: Author, share: Self::Response) -> anyhow::Result<Option<()>> {
        ensure!(share.author() == &peer, "Author does not match");
        ensure!(
            share.metadata() == &self.rand_metadata,
            "Metadata does not match: local {:?}, received {:?}",
            self.rand_metadata,
            share.metadata()
        );
        share.verify(&self.rand_config)?;
        let mut store = self.rand_store.lock();
        let aggregated = if store.add_share(share)? {
            Some(())
        } else {
            None
        };
        Ok(aggregated)
    }
}
