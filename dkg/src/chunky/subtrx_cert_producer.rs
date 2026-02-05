// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    chunky::types::{AggregatedSubtranscript, CertifiedAggregatedSubtranscript},
    counters,
    types::{
        ChunkyDKGSubtranscriptSignatureRequest, ChunkyDKGSubtranscriptSignatureResponse, DKGMessage,
    },
};
use anyhow::{anyhow, ensure, Context};
use aptos_channels::aptos_channel::Sender;
use aptos_consensus_types::common::Author;
use aptos_crypto::{bls12381::Signature, hash::CryptoHash, Signature as _};
use aptos_infallible::Mutex;
use aptos_logger::info;
use aptos_reliable_broadcast::{BroadcastStatus, ReliableBroadcast};
use aptos_types::{
    dkg::chunky_dkg::ChunkyDKGConfig, epoch_state::EpochState, validator_verifier::VerifyError,
};
use futures::future::AbortHandle;
use futures_util::future::Abortable;
use move_core_types::account_address::AccountAddress;
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tokio_retry::strategy::ExponentialBackoff;

#[allow(dead_code)]
pub fn start_chunky_subtranscript_certification(
    reliable_broadcast: Arc<ReliableBroadcast<DKGMessage, ExponentialBackoff>>,
    start_time: Duration,
    my_addr: AccountAddress,
    epoch_state: Arc<EpochState>,
    _dkg_config: ChunkyDKGConfig,
    aggregated_subtranscript: AggregatedSubtranscript,
    certified_agg_subtx_tx: Option<Sender<(), CertifiedAggregatedSubtranscript>>,
) -> AbortHandle {
    let epoch = epoch_state.epoch;
    let rb = reliable_broadcast.clone();
    let req = ChunkyDKGSubtranscriptSignatureRequest::new(
        epoch,
        aggregated_subtranscript.hash(),
        aggregated_subtranscript.dealers.clone(),
    );
    let validation_state = Arc::new(ChunkySubtranscriptCertificationState::new(
        start_time,
        my_addr,
        epoch_state.clone(),
        aggregated_subtranscript,
    ));
    let task = async move {
        let validated_trx = rb
            .broadcast(req, validation_state)
            .await
            .expect("broadcast cannot fail");
        info!(
            epoch = epoch,
            my_addr = my_addr,
            "[ChunkyDKG] validated aggregate subtranscript locally"
        );
        if let Err(e) = certified_agg_subtx_tx
            .expect("[ChunkyDKG] validated_trx_tx should be available")
            .push((), validated_trx)
        {
            // If the `ChunkyDKGManager` was dropped, this send will fail by design.
            info!(
                epoch = epoch,
                my_addr = my_addr,
                "[ChunkyDKG] Failed to send validated aggregate subtranscript to ChunkyDKGManager, maybe ChunkyDKGManager stopped and channel dropped: {:?}", e
            );
        }
    };
    let (abort_handle, abort_registration) = AbortHandle::new_pair();
    tokio::spawn(Abortable::new(task, abort_registration));
    abort_handle
}

#[derive(Default)]
struct ChunkySubtranscriptSignatureAggregator {
    signatures: BTreeMap<AccountAddress, Signature>,
}

pub struct ChunkySubtranscriptCertificationState {
    start_time: Duration,
    my_addr: AccountAddress,
    valid_peer_signature_seen: bool,
    sig_aggregator: Mutex<ChunkySubtranscriptSignatureAggregator>,
    epoch_state: Arc<EpochState>,
    aggregated_subtranscript: AggregatedSubtranscript,
}

impl ChunkySubtranscriptCertificationState {
    pub fn new(
        start_time: Duration,
        my_addr: AccountAddress,
        epoch_state: Arc<EpochState>,
        aggregated_subtranscript: AggregatedSubtranscript,
    ) -> Self {
        Self {
            start_time,
            my_addr,
            valid_peer_signature_seen: false,
            sig_aggregator: Mutex::new(ChunkySubtranscriptSignatureAggregator::default()),
            epoch_state,
            aggregated_subtranscript,
        }
    }
}

impl BroadcastStatus<DKGMessage> for Arc<ChunkySubtranscriptCertificationState> {
    type Aggregated = CertifiedAggregatedSubtranscript;
    type Message = ChunkyDKGSubtranscriptSignatureRequest;
    type Response = ChunkyDKGSubtranscriptSignatureResponse;

    fn add(
        &self,
        sender: Author,
        validation_response: ChunkyDKGSubtranscriptSignatureResponse,
    ) -> anyhow::Result<Option<Self::Aggregated>> {
        let ChunkyDKGSubtranscriptSignatureResponse {
            dealer_epoch: _,
            subtranscript_hash: _,
            signature,
        } = validation_response;

        let peer_power = self.epoch_state.verifier.get_voting_power(&sender);
        ensure!(
            peer_power.is_some(),
            "[ChunkyDKG] adding peer subtranscript validation signature failed with illegal validator"
        );

        let mut sig_aggregator = self.sig_aggregator.lock();
        if sig_aggregator.signatures.contains_key(&sender) {
            return Ok(None);
        }

        // Get the validator's public key (dealer public key) for signature verification
        let peer_pk = self
            .epoch_state
            .verifier
            .get_public_key(&sender)
            .ok_or_else(|| anyhow!("peer public key not found"))?;

        signature
            .verify(&self.aggregated_subtranscript, &peer_pk)
            .context("subtranscript validation signature verification failed")?;

        // All checks passed. Adding signature.
        let is_self = self.my_addr == sender;
        if !is_self && !self.valid_peer_signature_seen {
            counters::observe_chunky_dkg_stage(
                self.start_time,
                self.my_addr,
                "first_valid_peer_subtranscript_signature",
            );
        }

        sig_aggregator.signatures.insert(sender, signature);

        let threshold = self.epoch_state.verifier.quorum_voting_power();
        let power_check_result = self
            .epoch_state
            .verifier
            .check_voting_power(sig_aggregator.signatures.keys(), true);
        let new_total_power = match &power_check_result {
            Ok(x) => Some(*x),
            Err(VerifyError::TooLittleVotingPower { voting_power, .. }) => Some(*voting_power),
            _ => None,
        };

        let maybe_validated = if power_check_result.is_ok() {
            // Once we have 2f+1 signatures, aggregate them and return the validated subtranscript
            let aggregate_signature = self
                .epoch_state
                .verifier
                .aggregate_signatures(sig_aggregator.signatures.iter())?;

            Some(CertifiedAggregatedSubtranscript {
                aggregated_subtranscript: self.aggregated_subtranscript.clone(),
                aggregate_signature,
            })
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
            "[ChunkyDKG] added subtranscript validation signature from validator {}, {} out of {} aggregated.",
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
