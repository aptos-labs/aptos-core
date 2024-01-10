// Copyright © Aptos Foundation

#[cfg(test)]
use crate::dummy_dkg::DummyDKG;
#[cfg(test)]
use crate::dummy_dkg::DummyDKGTranscript;
use crate::{types::DKGNodeRequest, DKGMessage};
use anyhow::ensure;
use aptos_consensus_types::common::Author;
#[cfg(test)]
use aptos_crypto::{bls12381, Uniform};
use aptos_infallible::Mutex;
use aptos_reliable_broadcast::BroadcastStatus;
#[cfg(test)]
use aptos_types::dkg::DKGTranscriptMetadata;
#[cfg(test)]
use aptos_types::validator_verifier::ValidatorConsensusInfo;
#[cfg(test)]
use aptos_types::validator_verifier::ValidatorVerifier;
use aptos_types::{
    dkg::{DKGNode, DKGTrait},
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

#[test]
fn test_transcript_aggregation_state() {
    let num_validators = 5;
    let epoch = 999;
    let addrs: Vec<AccountAddress> = (0..num_validators)
        .map(|_| AccountAddress::random())
        .collect();
    let private_keys: Vec<bls12381::PrivateKey> = (0..num_validators)
        .map(|_| bls12381::PrivateKey::generate_for_testing())
        .collect();
    let public_keys: Vec<bls12381::PublicKey> = (0..num_validators)
        .map(|i| bls12381::PublicKey::from(&private_keys[i]))
        .collect();
    let voting_powers = [1, 1, 1, 6, 6]; // total voting power: 15, default threshold: 11
    let validator_infos: Vec<ValidatorConsensusInfo> = (0..num_validators)
        .map(|i| ValidatorConsensusInfo::new(addrs[i], public_keys[i].clone(), voting_powers[i]))
        .collect();
    let verifier = ValidatorVerifier::new(validator_infos);
    let epoch_state = Arc::new(EpochState { epoch, verifier });
    let trx_agg_state = Arc::new(TranscriptAggregationState::<DummyDKG>::new((), epoch_state));

    let good_transcript = DummyDKGTranscript::default();
    let good_trx_bytes = bcs::to_bytes(&good_transcript).unwrap();

    // Node with incorrect epoch should be rejected.
    let result = trx_agg_state.add(addrs[0], DKGNode {
        metadata: DKGTranscriptMetadata {
            epoch: 998,
            author: addrs[0],
        },
        transcript_bytes: good_trx_bytes.clone(),
    });
    assert!(result.is_err());

    // Node authored by X but sent by Y should be rejected.
    let result = trx_agg_state.add(addrs[1], DKGNode {
        metadata: DKGTranscriptMetadata {
            epoch: 999,
            author: addrs[0],
        },
        transcript_bytes: good_trx_bytes.clone(),
    });
    assert!(result.is_err());

    // Node with invalid transcript should be rejected.
    let mut bad_trx_bytes = good_trx_bytes.clone();
    bad_trx_bytes[0] = 0xAB;
    let result = trx_agg_state.add(addrs[2], DKGNode {
        metadata: DKGTranscriptMetadata {
            epoch: 999,
            author: addrs[2],
        },
        transcript_bytes: vec![],
    });
    assert!(result.is_err());

    // Good node should be accepted.
    let result = trx_agg_state.add(addrs[3], DKGNode {
        metadata: DKGTranscriptMetadata {
            epoch: 999,
            author: addrs[3],
        },
        transcript_bytes: good_trx_bytes.clone(),
    });
    assert!(matches!(result, Ok(None)));

    // Node from contributed author should be ignored.
    let result = trx_agg_state.add(addrs[3], DKGNode {
        metadata: DKGTranscriptMetadata {
            epoch: 999,
            author: addrs[3],
        },
        transcript_bytes: good_trx_bytes.clone(),
    });
    assert!(matches!(result, Ok(None)));

    // Aggregated trx should be returned if after adding a node, the threshold is exceeded.
    let result = trx_agg_state.add(addrs[4], DKGNode {
        metadata: DKGTranscriptMetadata {
            epoch: 999,
            author: addrs[4],
        },
        transcript_bytes: good_trx_bytes.clone(),
    });
    assert!(matches!(result, Ok(Some(_))));
}

impl<S: DKGTrait> BroadcastStatus<DKGMessage> for Arc<TranscriptAggregationState<S>> {
    type Aggregated = S::Transcript;
    type Message = DKGNodeRequest;
    type Response = DKGNode;

    fn add(&self, sender: Author, dkg_node: DKGNode) -> anyhow::Result<Option<Self::Aggregated>> {
        let DKGNode {
            metadata,
            transcript_bytes,
        } = dkg_node;
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
            S::aggregate_transcripts(&self.dkg_pub_params, agg_trx, &transcript);
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
