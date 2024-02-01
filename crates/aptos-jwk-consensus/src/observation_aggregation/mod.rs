// Copyright © Aptos Foundation

use crate::types::{
    JWKConsensusMsg, ObservedUpdate, ObservedUpdateRequest, ObservedUpdateResponse,
};
use anyhow::ensure;
use aptos_consensus_types::common::Author;
use aptos_crypto::bls12381;
use aptos_infallible::Mutex;
use aptos_reliable_broadcast::BroadcastStatus;
use aptos_types::{
    epoch_state::EpochState,
    jwks::{ProviderJWKs, QuorumCertifiedUpdate},
};
use move_core_types::account_address::AccountAddress;
use std::{collections::HashSet, sync::Arc};

/// The aggregation state of reliable broadcast where a validator broadcast JWK observation requests
/// and produce quorum-certified JWK updates.
pub struct ObservationAggregationState {
    epoch_state: Arc<EpochState>,
    local_view: ProviderJWKs,
    inner_state: Mutex<InnerState>,
}

#[derive(Default)]
struct InnerState {
    pub contributors: HashSet<AccountAddress>,
    pub multi_sig: Option<bls12381::Signature>,
}

impl ObservationAggregationState {
    pub fn new(epoch_state: Arc<EpochState>, local_view: ProviderJWKs) -> Self {
        Self {
            epoch_state,
            local_view,
            inner_state: Mutex::new(InnerState::default()),
        }
    }
}

impl BroadcastStatus<JWKConsensusMsg> for Arc<ObservationAggregationState> {
    type Aggregated = QuorumCertifiedUpdate;
    type Message = ObservedUpdateRequest;
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

        let mut aggregator = self.inner_state.lock();
        if aggregator.contributors.contains(&sender) {
            return Ok(None);
        }

        ensure!(
            self.local_view == peer_view,
            "adding peer observation failed with mismatched view"
        );

        // Verify the quorum-cert.
        self.epoch_state
            .verifier
            .verify(sender, &peer_view, &signature)?;

        // All checks passed. Aggregating.
        aggregator.contributors.insert(sender);
        let new_multi_sig = if let Some(existing) = aggregator.multi_sig.take() {
            bls12381::Signature::aggregate(vec![existing, signature])?
        } else {
            signature
        };

        let maybe_qc_update = self
            .epoch_state
            .verifier
            .check_voting_power(aggregator.contributors.iter(), true)
            .ok()
            .map(|_| QuorumCertifiedUpdate {
                authors: aggregator.contributors.clone().into_iter().collect(),
                update: peer_view,
                multi_sig: new_multi_sig.clone(),
            });

        aggregator.multi_sig = Some(new_multi_sig);

        Ok(maybe_qc_update)
    }
}

#[cfg(test)]
mod tests;
