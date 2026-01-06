// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    counters::DKG_STAGE_SECONDS,
    types::{DKGMessage, DKGSubtranscriptValidationRequest},
};
use anyhow::{anyhow, ensure, Context};
use aptos_batch_encryption::group::Pairing;
use aptos_channels::aptos_channel::Sender;
use aptos_consensus_types::common::Author;
use aptos_crypto::bls12381::Signature;
use aptos_dkg::pvss::{chunky::PublicParameters, signed::generic_signing::SessionContribution};
use aptos_infallible::{duration_since_epoch, Mutex};
use aptos_logger::info;
use aptos_reliable_broadcast::{BroadcastStatus, ReliableBroadcast};
use aptos_types::{
    dkg::{
        chunky_dkg::{
            ChunkyDKGSessionMetadata, ChunkySubtranscript, ChunkyTranscript, DealerPublicKey,
            EncryptPubKey, SecretSharingConfig,
        },
        DKGTranscriptMetadata,
    },
    epoch_state::EpochState,
    validator_verifier::VerifyError,
};
use futures::future::AbortHandle;
use futures_util::future::Abortable;
use move_core_types::account_address::AccountAddress;
use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio_retry::strategy::ExponentialBackoff;

pub struct ChunkySubtranscriptValidationProducer {
    reliable_broadcast: Arc<ReliableBroadcast<DKGMessage, ExponentialBackoff>>,
}

impl ChunkySubtranscriptValidationProducer {
    pub fn new(reliable_broadcast: ReliableBroadcast<DKGMessage, ExponentialBackoff>) -> Self {
        Self {
            reliable_broadcast: Arc::new(reliable_broadcast),
        }
    }

    pub fn start_validate(
        &self,
        start_time: Duration,
        my_addr: AccountAddress,
        epoch_state: Arc<EpochState>,
        secret_sharing_config: SecretSharingConfig,
        public_parameters: PublicParameters<Pairing>,
        eks: Vec<EncryptPubKey>,
        spks: Vec<DealerPublicKey>,
        session_metadata: ChunkyDKGSessionMetadata,
        aggregated_subtranscript: ChunkySubtranscript,
        aggregated_transcript: ChunkyTranscript,
        validated_trx_tx: Option<Sender<(), ChunkyTranscript>>,
    ) -> AbortHandle {
        let epoch = epoch_state.epoch;
        let rb = self.reliable_broadcast.clone();

        // Compute hash of the subtranscript
        let subtranscript_hash = aggregated_subtranscript.to_bytes();

        // Get dealers from the aggregated transcript
        let dealers = aggregated_transcript.get_dealers();

        let req = DKGSubtranscriptValidationRequest::new(epoch, subtranscript_hash, dealers);
        let validation_state = Arc::new(ChunkySubtranscriptValidationState::new(
            start_time,
            my_addr,
            secret_sharing_config,
            public_parameters,
            eks,
            spks,
            session_metadata,
            epoch_state.clone(),
            aggregated_subtranscript,
            aggregated_transcript,
        ));
        let task = async move {
            let validated_trx = rb
                .broadcast(req, validation_state)
                .await
                .expect("broadcast cannot fail");
            info!(
                epoch = epoch,
                my_addr = my_addr,
                "[DKG] validated aggregate subtranscript locally"
            );
            if let Err(e) = validated_trx_tx
                .expect("[DKG] validated_trx_tx should be available")
                .push((), validated_trx)
            {
                // If the `ChunkyDKGManager` was dropped, this send will fail by design.
                info!(
                    epoch = epoch,
                    my_addr = my_addr,
                    "[DKG] Failed to send validated aggregate subtranscript to ChunkyDKGManager, maybe ChunkyDKGManager stopped and channel dropped: {:?}", e
                );
            }
        };
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(task, abort_registration));
        abort_handle
    }
}

struct ChunkySubtranscriptSignatureAggregator {
    contributors: HashSet<AccountAddress>,
    signatures: Vec<(AccountAddress, Signature)>,
}

impl Default for ChunkySubtranscriptSignatureAggregator {
    fn default() -> Self {
        Self {
            contributors: HashSet::new(),
            signatures: Vec::new(),
        }
    }
}

pub struct ChunkySubtranscriptValidationState {
    start_time: Duration,
    my_addr: AccountAddress,
    valid_peer_signature_seen: bool,
    sig_aggregator: Mutex<ChunkySubtranscriptSignatureAggregator>,
    secret_sharing_config: SecretSharingConfig,
    public_parameters: PublicParameters<Pairing>,
    eks: Vec<EncryptPubKey>,
    spks: Vec<DealerPublicKey>,
    session_metadata: ChunkyDKGSessionMetadata,
    epoch_state: Arc<EpochState>,
    aggregated_subtranscript: ChunkySubtranscript,
    aggregated_transcript: ChunkyTranscript,
}

impl ChunkySubtranscriptValidationState {
    pub fn new(
        start_time: Duration,
        my_addr: AccountAddress,
        secret_sharing_config: SecretSharingConfig,
        public_parameters: PublicParameters<Pairing>,
        eks: Vec<EncryptPubKey>,
        spks: Vec<DealerPublicKey>,
        session_metadata: ChunkyDKGSessionMetadata,
        epoch_state: Arc<EpochState>,
        aggregated_subtranscript: ChunkySubtranscript,
        aggregated_transcript: ChunkyTranscript,
    ) -> Self {
        Self {
            start_time,
            my_addr,
            valid_peer_signature_seen: false,
            sig_aggregator: Mutex::new(ChunkySubtranscriptSignatureAggregator::default()),
            secret_sharing_config,
            public_parameters,
            eks,
            spks,
            session_metadata,
            epoch_state,
            aggregated_subtranscript,
            aggregated_transcript,
        }
    }
}

impl BroadcastStatus<DKGMessage> for Arc<ChunkySubtranscriptValidationState> {
    type Aggregated = ChunkyTranscript;
    type Message = DKGSubtranscriptValidationRequest;
    type Response = crate::types::DKGSubtranscriptValidationResponse;

    fn add(
        &self,
        sender: Author,
        validation_response: crate::types::DKGSubtranscriptValidationResponse,
    ) -> anyhow::Result<Option<Self::Aggregated>> {
        let crate::types::DKGSubtranscriptValidationResponse {
            metadata,
            signature,
        } = validation_response;

        ensure!(
            metadata.epoch == self.epoch_state.epoch,
            "[DKG] adding peer subtranscript validation signature failed with invalid node epoch",
        );

        let peer_power = self.epoch_state.verifier.get_voting_power(&sender);
        ensure!(
            peer_power.is_some(),
            "[DKG] adding peer subtranscript validation signature failed with illegal validator"
        );
        ensure!(
            metadata.author == sender,
            "[DKG] adding peer subtranscript validation signature failed with node author mismatch"
        );

        let mut sig_aggregator = self.sig_aggregator.lock();
        if sig_aggregator.contributors.contains(&metadata.author) {
            return Ok(None);
        }

        // Verify the signature
        // The signature is on the dealt public key from the aggregated subtranscript
        let dealt_pub_key = self.aggregated_subtranscript.get_dealt_public_key();
        let session_contribution = SessionContribution {
            contrib: dealt_pub_key,
            sid: &self.session_metadata,
        };

        // Get the validator's public key (dealer public key) for signature verification
        let peer_pk = self
            .epoch_state
            .verifier
            .get_public_key(&sender)
            .ok_or_else(|| anyhow!("peer public key not found"))?;

        signature
            .verify(&session_contribution, &peer_pk)
            .context("subtranscript validation signature verification failed")?;

        // All checks passed. Adding signature.
        let is_self = self.my_addr == sender;
        if !is_self && !self.valid_peer_signature_seen {
            let secs_since_dkg_start =
                duration_since_epoch().as_secs_f64() - self.start_time.as_secs_f64();
            DKG_STAGE_SECONDS
                .with_label_values(&[
                    self.my_addr.to_hex().as_str(),
                    "first_valid_peer_subtranscript_signature",
                ])
                .observe(secs_since_dkg_start);
        }

        sig_aggregator.contributors.insert(metadata.author);
        sig_aggregator.signatures.push((metadata.author, signature));

        let threshold = self.epoch_state.verifier.quorum_voting_power();
        let power_check_result = self
            .epoch_state
            .verifier
            .check_voting_power(sig_aggregator.contributors.iter(), true);
        let new_total_power = match &power_check_result {
            Ok(x) => Some(*x),
            Err(VerifyError::TooLittleVotingPower { voting_power, .. }) => Some(*voting_power),
            _ => None,
        };

        let maybe_validated = if power_check_result.is_ok() {
            // Once we have 2f+1 signatures, the aggregated transcript is validated
            // Return the stored aggregated transcript
            Some(self.aggregated_transcript.clone())
        } else {
            None
        };

        info!(
            epoch = self.epoch_state.epoch,
            peer = sender,
            is_self = is_self,
            peer_power = peer_power,
            new_total_power = new_total_power,
            threshold = threshold,
            threshold_exceeded = maybe_validated.is_some(),
            "[DKG] added subtranscript validation signature from validator {}, {} out of {} aggregated.",
            self.epoch_state
                .verifier
                .address_to_validator_index()
                .get(&sender)
                .unwrap(),
            new_total_power.unwrap_or(0),
            threshold
        );

        Ok(maybe_validated)
    }
}
