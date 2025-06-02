// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{counters::DKG_STAGE_SECONDS, types::DKGTranscriptRequest, DKGMessage};
use anyhow::{anyhow, ensure, Context};
use aptos_consensus_types::common::Author;
use aptos_infallible::{duration_since_epoch, Mutex};
use aptos_logger::info;
use aptos_reliable_broadcast::BroadcastStatus;
use aptos_types::{
    dkg::{DKGTrait, DKGTranscript},
    epoch_state::EpochState,
    validator_verifier::VerifyError,
};
use move_core_types::account_address::AccountAddress;
use std::{collections::HashSet, sync::Arc, time::Duration};

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
    start_time: Duration,
    my_addr: AccountAddress,
    valid_peer_transcript_seen: bool,
    trx_aggregator: Mutex<TranscriptAggregator<DKG>>,
    dkg_pub_params: DKG::PublicParams,
    epoch_state: Arc<EpochState>,
}

impl<DKG: DKGTrait> TranscriptAggregationState<DKG> {
    pub fn new(
        start_time: Duration,
        my_addr: AccountAddress,
        dkg_pub_params: DKG::PublicParams,
        epoch_state: Arc<EpochState>,
    ) -> Self {
        //TODO(zjma): take DKG threshold as a parameter.
        Self {
            start_time,
            my_addr,
            valid_peer_transcript_seen: false,
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
            "[DKG] adding peer transcript failed with invalid node epoch",
        );

        let peer_power = self.epoch_state.verifier.get_voting_power(&sender);
        ensure!(
            peer_power.is_some(),
            "[DKG] adding peer transcript failed with illegal dealer"
        );
        ensure!(
            metadata.author == sender,
            "[DKG] adding peer transcript failed with node author mismatch"
        );
        let transcript = bcs::from_bytes(transcript_bytes.as_slice()).map_err(|e| {
            anyhow!("[DKG] adding peer transcript failed with trx deserialization error: {e}")
        })?;
        let mut trx_aggregator = self.trx_aggregator.lock();
        if trx_aggregator.contributors.contains(&metadata.author) {
            return Ok(None);
        }

        S::verify_transcript_extra(&transcript, &self.epoch_state.verifier, false, Some(sender))
            .context("extra verification failed")?;

        S::verify_transcript(&self.dkg_pub_params, &transcript).map_err(|e| {
            anyhow!("[DKG] adding peer transcript failed with trx verification failure: {e}")
        })?;

        // All checks passed. Aggregating.
        let is_self = self.my_addr == sender;
        if !is_self && !self.valid_peer_transcript_seen {
            let secs_since_dkg_start =
                duration_since_epoch().as_secs_f64() - self.start_time.as_secs_f64();
            DKG_STAGE_SECONDS
                .with_label_values(&[
                    self.my_addr.to_hex().as_str(),
                    "first_valid_peer_transcript",
                ])
                .observe(secs_since_dkg_start);
        }

        trx_aggregator.contributors.insert(metadata.author);
        if let Some(agg_trx) = trx_aggregator.trx.as_mut() {
            S::aggregate_transcripts(&self.dkg_pub_params, agg_trx, transcript);
        } else {
            trx_aggregator.trx = Some(transcript);
        }
        let threshold = self.epoch_state.verifier.quorum_voting_power();
        let power_check_result = self
            .epoch_state
            .verifier
            .check_voting_power(trx_aggregator.contributors.iter(), true);
        let new_total_power = match &power_check_result {
            Ok(x) => Some(*x),
            Err(VerifyError::TooLittleVotingPower { voting_power, .. }) => Some(*voting_power),
            _ => None,
        };
        let maybe_aggregated = power_check_result
            .ok()
            .map(|_| trx_aggregator.trx.clone().unwrap());
        info!(
            epoch = self.epoch_state.epoch,
            peer = sender,
            is_self = is_self,
            peer_power = peer_power,
            new_total_power = new_total_power,
            threshold = threshold,
            threshold_exceeded = maybe_aggregated.is_some(),
            "[DKG] added transcript from validator {}, {} out of {} aggregated.",
            self.epoch_state
                .verifier
                .address_to_validator_index()
                .get(&sender)
                .unwrap(),
            new_total_power.unwrap_or(0),
            threshold
        );
        Ok(maybe_aggregated)
    }
}

#[cfg(test)]
mod tests;
