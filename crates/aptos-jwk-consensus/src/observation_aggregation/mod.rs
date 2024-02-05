// Copyright © Aptos Foundation

use crate::types::{
    JWKConsensusMsg, ObservedUpdate, ObservedUpdateRequest, ObservedUpdateResponse,
};
use anyhow::{anyhow, ensure};
use aptos_consensus_types::common::Author;
use aptos_crypto::bls12381::Signature;
use aptos_infallible::Mutex;
use aptos_reliable_broadcast::BroadcastStatus;
use aptos_types::{
    aggregate_signature::PartialSignatures,
    epoch_state::EpochState,
    jwks::{ProviderJWKs, QuorumCertifiedUpdate},
};
use move_core_types::account_address::AccountAddress;
use std::{collections::BTreeSet, sync::Arc};
use aptos_bitvec::BitVec;
use aptos_types::aggregate_signature::AggregateSignature;

/// The aggregation state of reliable broadcast where a validator broadcast JWK observation requests
/// and produce quorum-certified JWK updates.
pub struct ObservationAggregationState {
    epoch_state: Arc<EpochState>,
    local_view: ProviderJWKs,
    inner_state: Mutex<PartialSignatures>,
}

impl ObservationAggregationState {
    pub fn new(epoch_state: Arc<EpochState>, local_view: ProviderJWKs) -> Self {
        Self {
            epoch_state,
            local_view,
            inner_state: Mutex::new(PartialSignatures::empty()),
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
        if self
            .epoch_state
            .verifier
            .check_voting_power(voters.iter(), true)
            .is_err()
        {
            return Ok(None);
        }
        let multi_sig = Signature::aggregate(
            partial_sigs
                .signatures()
                .values()
                .cloned()
                .collect::<Vec<_>>(),
        )
        .map_err(|e| anyhow!("jwk update certification failed with sig agg error: {e}"))?;
        let signer_bit_vec = BitVec::from(self.epoch_state.verifier.get_ordered_account_addresses().into_iter().map(|addr|voters.contains(&addr)).collect());
        let multi_sig = AggregateSignature::new(signer_bit_vec, Some(multi_sig));

        Ok(Some(QuorumCertifiedUpdate {
            update: peer_view,
            multi_sig,
        }))
    }
}

#[cfg(test)]
mod tests;
