// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{counters::DKG_STAGE_SECONDS, types::DKGTranscriptRequest, DKGMessage};
use anyhow::{anyhow, ensure, Context};
use aptos_batch_encryption::group::Pairing;
use aptos_channels::aptos_channel::Sender;
use aptos_consensus_types::common::Author;
use aptos_dkg::pvss::chunky::PublicParameters;
use aptos_infallible::{duration_since_epoch, Mutex};
use aptos_logger::info;
use aptos_reliable_broadcast::{BroadcastStatus, ReliableBroadcast};
use aptos_types::{
    dkg::{
        chunky_dkg::{
            ChunkyDKG, ChunkyDKGSessionMetadata, ChunkyTranscript, DealerPublicKey, EncryptPubKey,
            SecretSharingConfig,
        },
        DKGTranscript,
    },
    epoch_state::EpochState,
    validator_verifier::VerifyError,
};
use futures::future::AbortHandle;
use futures_util::future::Abortable;
use move_core_types::account_address::AccountAddress;
use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio_retry::strategy::ExponentialBackoff;

pub struct ChunkyAggTrxProducer {
    reliable_broadcast: Arc<ReliableBroadcast<DKGMessage, ExponentialBackoff>>,
}

impl ChunkyAggTrxProducer {
    pub fn new(reliable_broadcast: ReliableBroadcast<DKGMessage, ExponentialBackoff>) -> Self {
        Self {
            reliable_broadcast: Arc::new(reliable_broadcast),
        }
    }

    pub fn start_produce(
        &self,
        start_time: Duration,
        my_addr: AccountAddress,
        epoch_state: Arc<EpochState>,
        secret_sharing_config: SecretSharingConfig,
        public_parameters: PublicParameters<Pairing>,
        eks: Vec<EncryptPubKey>,
        spks: Vec<DealerPublicKey>,
        session_metadata: ChunkyDKGSessionMetadata,
        agg_trx_tx: Option<Sender<(), ChunkyTranscript>>,
    ) -> AbortHandle {
        let epoch = epoch_state.epoch;
        let rb = self.reliable_broadcast.clone();
        let req = DKGTranscriptRequest::new(epoch_state.epoch);
        let agg_state = Arc::new(ChunkyTranscriptAggregationState::new(
            start_time,
            my_addr,
            secret_sharing_config,
            public_parameters,
            eks,
            spks,
            session_metadata,
            epoch_state,
        ));
        let task = async move {
            let agg_trx = rb
                .broadcast(req, agg_state)
                .await
                .expect("broadcast cannot fail");
            info!(
                epoch = epoch,
                my_addr = my_addr,
                "[DKG] aggregated chunky transcript locally"
            );
            if let Err(e) = agg_trx_tx
                .expect("[DKG] agg_trx_tx should be available")
                .push((), agg_trx)
            {
                // If the `ChunkyDKGManager` was dropped, this send will fail by design.
                info!(
                    epoch = epoch,
                    my_addr = my_addr,
                    "[DKG] Failed to send aggregated chunky transcript to ChunkyDKGManager, maybe ChunkyDKGManager stopped and channel dropped: {:?}", e
                );
            }
        };
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(task, abort_registration));
        abort_handle
    }
}

struct ChunkyTranscriptAggregator {
    contributors: HashSet<AccountAddress>,
    trx: Option<ChunkyTranscript>,
}

impl Default for ChunkyTranscriptAggregator {
    fn default() -> Self {
        Self {
            contributors: HashSet::new(),
            trx: None,
        }
    }
}

pub struct ChunkyTranscriptAggregationState {
    start_time: Duration,
    my_addr: AccountAddress,
    valid_peer_transcript_seen: bool,
    trx_aggregator: Mutex<ChunkyTranscriptAggregator>,
    secret_sharing_config: SecretSharingConfig,
    public_parameters: PublicParameters<Pairing>,
    eks: Vec<EncryptPubKey>,
    spks: Vec<DealerPublicKey>,
    session_metadata: ChunkyDKGSessionMetadata,
    epoch_state: Arc<EpochState>,
}

impl ChunkyTranscriptAggregationState {
    pub fn new(
        start_time: Duration,
        my_addr: AccountAddress,
        secret_sharing_config: SecretSharingConfig,
        public_parameters: PublicParameters<Pairing>,
        eks: Vec<EncryptPubKey>,
        spks: Vec<DealerPublicKey>,
        session_metadata: ChunkyDKGSessionMetadata,
        epoch_state: Arc<EpochState>,
    ) -> Self {
        Self {
            start_time,
            my_addr,
            valid_peer_transcript_seen: false,
            trx_aggregator: Mutex::new(ChunkyTranscriptAggregator::default()),
            secret_sharing_config,
            public_parameters,
            eks,
            spks,
            session_metadata,
            epoch_state,
        }
    }
}

impl BroadcastStatus<DKGMessage> for Arc<ChunkyTranscriptAggregationState> {
    type Aggregated = ChunkyTranscript;
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
            "[DKG] adding peer chunky transcript failed with invalid node epoch",
        );

        let peer_power = self.epoch_state.verifier.get_voting_power(&sender);
        ensure!(
            peer_power.is_some(),
            "[DKG] adding peer chunky transcript failed with illegal dealer"
        );
        ensure!(
            metadata.author == sender,
            "[DKG] adding peer chunky transcript failed with node author mismatch"
        );
        let transcript: ChunkyTranscript = bcs::from_bytes(transcript_bytes.as_slice()).map_err(|e| {
            anyhow!("[DKG] adding peer chunky transcript failed with trx deserialization error: {e}")
        })?;
        let mut trx_aggregator = self.trx_aggregator.lock();
        if trx_aggregator.contributors.contains(&metadata.author) {
            return Ok(None);
        }

        // Verify the transcript directly
        transcript
            .verify(
                &self.secret_sharing_config,
                &self.public_parameters,
                &self.spks,
                &self.eks,
                &self.session_metadata,
            )
            .context("chunky transcript verification failed")?;

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
            agg_trx
                .aggregate_with(&self.secret_sharing_config, &transcript)
                .context("chunky transcript aggregation failed")?;
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
            "[DKG] added chunky transcript from validator {}, {} out of {} aggregated.",
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
