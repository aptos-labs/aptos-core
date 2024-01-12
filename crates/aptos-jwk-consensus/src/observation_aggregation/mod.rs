// Copyright © Aptos Foundation

use crate::types::{
    JWKConsensusMsg, ObservedUpdate, ObservedUpdateRequest, ObservedUpdateResponse,
};
use anyhow::ensure;
use aptos_consensus_types::common::Author;
use aptos_crypto::{bls12381, bls12381::Signature};
use aptos_infallible::Mutex;
use aptos_logger::debug;
use aptos_reliable_broadcast::BroadcastStatus;
use aptos_types::{
    epoch_state::EpochState,
    jwks::{ProviderJWKs, QuorumCertifiedUpdate},
};
use move_core_types::account_address::AccountAddress;
use std::{collections::HashSet, sync::Arc};

#[derive(Default)]
pub struct ObservationAggregator {
    pub contributors: HashSet<AccountAddress>,
    pub multi_sig: Option<bls12381::Signature>,
}

pub struct ObservationAggregationState {
    my_addr: AccountAddress,
    epoch_state: Arc<EpochState>,
    local_view: ProviderJWKs,
    observation_aggregator: Mutex<ObservationAggregator>,
}

impl ObservationAggregationState {
    pub fn new(
        my_addr: AccountAddress,
        epoch_state: Arc<EpochState>,
        local_view: ProviderJWKs,
    ) -> Self {
        Self {
            my_addr,
            epoch_state,
            local_view,
            observation_aggregator: Mutex::new(ObservationAggregator::default()),
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
        debug!(
            "[JWK] trying aggregating update={:?} from sender={}, is_self={}",
            update,
            sender,
            sender == self.my_addr
        );
        let ObservedUpdate {
            author,
            observed: peer_view,
            signature,
        } = update;
        ensure!(
            epoch == self.epoch_state.epoch,
            "adding peer observation failed with invalid epoch",
        );
        debug!("[JWK] epoch check passed");
        ensure!(
            author == sender,
            "adding peer observation failed with mismatched author",
        );
        debug!("[JWK] sender check passed");
        let mut aggregator = self.observation_aggregator.lock();
        if aggregator.contributors.contains(&sender) {
            debug!("[JWK] already contributed, ignoring");
            return Ok(None);
        }

        ensure!(
            self.local_view == peer_view,
            "adding peer observation failed with mismatched view"
        );
        debug!("[JWK] view check passed");
        self.epoch_state
            .verifier
            .verify(sender, &peer_view, &signature)?;

        debug!("[JWK] sig verified, all check passed");

        // All checks passed. Aggregating.
        aggregator.contributors.insert(sender);
        let new_multi_sig = if let Some(existing) = aggregator.multi_sig.take() {
            Signature::aggregate(vec![existing, signature])?
        } else {
            signature
        };
        aggregator.multi_sig = Some(new_multi_sig);

        let maybe_qc_update = self
            .epoch_state
            .verifier
            .check_voting_power(aggregator.contributors.iter(), true)
            .ok()
            .map(|_| QuorumCertifiedUpdate {
                authors: aggregator.contributors.clone().into_iter().collect(),
                observed: peer_view,
                multi_sig: aggregator.multi_sig.clone().unwrap(),
            });
        Ok(maybe_qc_update)
    }
}

#[cfg(test)]
mod tests;
