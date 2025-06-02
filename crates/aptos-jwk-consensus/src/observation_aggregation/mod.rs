// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    mode::TConsensusMode,
    types::{JWKConsensusMsg, ObservedUpdate, ObservedUpdateResponse},
};
use anyhow::{anyhow, ensure};
use aptos_consensus_types::common::Author;
use aptos_infallible::Mutex;
use aptos_logger::info;
use aptos_reliable_broadcast::BroadcastStatus;
use aptos_types::{
    aggregate_signature::PartialSignatures,
    epoch_state::EpochState,
    jwks::{ProviderJWKs, QuorumCertifiedUpdate},
    validator_verifier::VerifyError,
};
use move_core_types::account_address::AccountAddress;
use std::{collections::BTreeSet, marker::PhantomData, sync::Arc};

/// The aggregation state of reliable broadcast where a validator broadcast JWK observation requests
/// and produce quorum-certified JWK updates.
pub struct ObservationAggregationState<ConsensusMode> {
    epoch_state: Arc<EpochState>,
    local_view: ProviderJWKs,
    inner_state: Mutex<PartialSignatures>,
    _phantom: PhantomData<ConsensusMode>,
}

impl<ConsensusMode> ObservationAggregationState<ConsensusMode> {
    pub fn new(epoch_state: Arc<EpochState>, local_view: ProviderJWKs) -> Self {
        Self {
            epoch_state,
            local_view,
            inner_state: Mutex::new(PartialSignatures::empty()),
            _phantom: Default::default(),
        }
    }
}

impl<ConsensusMode: TConsensusMode> BroadcastStatus<JWKConsensusMsg>
    for Arc<ObservationAggregationState<ConsensusMode>>
{
    type Aggregated = QuorumCertifiedUpdate;
    type Message = ConsensusMode::ReliableBroadcastRequest;
    type Response = ObservedUpdateResponse;

    fn add(
        &self,
        sender: Author,
        response: Self::Response,
    ) -> anyhow::Result<Option<Self::Aggregated>> {
        let ObservedUpdateResponse { epoch, update } = response;
        let ObservedUpdate {
            author,
            observed: peer_view,
            signature,
        } = update;
        ensure!(
            epoch == self.epoch_state.epoch,
            "adding peer observation failed with invalid epoch",
        );
        ensure!(
            author == sender,
            "adding peer observation failed with mismatched author",
        );

        let peer_power = self.epoch_state.verifier.get_voting_power(&author);
        ensure!(
            peer_power.is_some(),
            "adding peer observation failed with illegal signer"
        );
        let peer_power = peer_power.unwrap();

        let mut partial_sigs = self.inner_state.lock();
        if partial_sigs.contains_voter(&sender) {
            return Ok(None);
        }

        ensure!(
            self.local_view == peer_view,
            "adding peer observation failed with mismatched view"
        );

        // Verify peer signature.
        self.epoch_state
            .verifier
            .verify(sender, &peer_view, &signature)?;

        // All checks passed. Aggregating.
        partial_sigs.add_signature(sender, signature);
        let voters: BTreeSet<AccountAddress> = partial_sigs.signatures().keys().copied().collect();
        let power_check_result = self
            .epoch_state
            .verifier
            .check_voting_power(voters.iter(), true);
        let new_total_power = match &power_check_result {
            Ok(x) => Some(*x),
            Err(VerifyError::TooLittleVotingPower { voting_power, .. }) => Some(*voting_power),
            _ => None,
        };

        info!(
            epoch = self.epoch_state.epoch,
            peer = sender,
            issuer = String::from_utf8(self.local_view.issuer.clone()).ok(),
            peer_power = peer_power,
            new_total_power = new_total_power,
            threshold = self.epoch_state.verifier.quorum_voting_power(),
            threshold_exceeded = power_check_result.is_ok(),
            "Peer vote aggregated."
        );

        if power_check_result.is_err() {
            return Ok(None);
        }
        let multi_sig = self.epoch_state.verifier.aggregate_signatures(partial_sigs.signatures_iter()).map_err(|e|anyhow!("adding peer observation failed with partial-to-aggregated conversion error: {e}"))?;

        Ok(Some(QuorumCertifiedUpdate {
            update: peer_view,
            multi_sig,
        }))
    }
}

#[cfg(test)]
mod tests;
