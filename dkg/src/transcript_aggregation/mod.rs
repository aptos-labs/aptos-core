// Copyright © Aptos Foundation

use crate::{types::DKGTranscriptRequest, DKGMessage};
use anyhow::ensure;
use aptos_consensus_types::common::Author;
use aptos_infallible::Mutex;
use aptos_reliable_broadcast::BroadcastStatus;
use aptos_types::{
    dkg::{DKGTrait, DKGTranscript},
    epoch_state::EpochState,
};
use move_core_types::account_address::AccountAddress;
use std::{collections::HashSet, sync::Arc};

pub struct TranscriptAggregator<S: DKGTrait> {
    pub contributors: HashSet<AccountAddress>,
    pub trx: Option<S::Transcript>,
}

impl<S: DKGTrait> Default for TranscriptAggregator<S> {
    fn default() -> Self {
        Self {
            contributors: HashSet::new(),
            trx: None,
        }
    }
}

pub struct TranscriptAggregationState<DKG: DKGTrait> {
    trx_aggregator: Mutex<TranscriptAggregator<DKG>>,
    dkg_pub_params: DKG::PublicParams,
    epoch_state: Arc<EpochState>,
}

impl<DKG: DKGTrait> TranscriptAggregationState<DKG> {
    pub fn new(dkg_pub_params: DKG::PublicParams, epoch_state: Arc<EpochState>) -> Self {
        //TODO(zjma): take DKG threshold as a parameter.
        Self {
            trx_aggregator: Mutex::new(TranscriptAggregator::default()),
            dkg_pub_params,
            epoch_state,
        }
    }
}

impl<S: DKGTrait> BroadcastStatus<DKGMessage> for Arc<TranscriptAggregationState<S>> {
    type Aggregated = S::Transcript;
    type Message = DKGTranscriptRequest;
    type Response = DKGTranscript;

    fn add(
        &self,
        sender: Author,
        dkg_transcript: DKGTranscript,
    ) -> anyhow::Result<Option<Self::Aggregated>> {
        let DKGTranscript {
            metadata,
            transcript_bytes,
        } = dkg_transcript;
        ensure!(
            metadata.epoch == self.epoch_state.epoch,
            "adding dkg node failed with invalid node epoch",
        );
        ensure!(
            metadata.author == sender,
            "adding dkg node failed with node author mismatch"
        );
        let transcript = bcs::from_bytes(transcript_bytes.as_slice())?;
        let mut trx_aggregator = self.trx_aggregator.lock();
        if trx_aggregator.contributors.contains(&metadata.author) {
            return Ok(None);
        }

        S::verify_transcript(&self.dkg_pub_params, &transcript)?;

        // All checks passed. Aggregating.
        trx_aggregator.contributors.insert(metadata.author);
        if let Some(agg_trx) = trx_aggregator.trx.as_mut() {
            let acc = std::mem::take(agg_trx);
            *agg_trx = S::aggregate_transcripts(&self.dkg_pub_params, vec![acc, transcript]);
        } else {
            trx_aggregator.trx = Some(transcript);
        }
        let maybe_aggregated = self
            .epoch_state
            .verifier
            .check_voting_power(trx_aggregator.contributors.iter(), true)
            .ok()
            .map(|_aggregated_voting_power| trx_aggregator.trx.clone().unwrap());
        Ok(maybe_aggregated)
    }
}

#[cfg(test)]
mod tests;
