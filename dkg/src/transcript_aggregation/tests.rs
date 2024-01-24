// Copyright © Aptos Foundation

use crate::transcript_aggregation::TranscriptAggregationState;
use aptos_crypto::{bls12381::bls12381_keys, Uniform};
use aptos_reliable_broadcast::BroadcastStatus;
use aptos_types::{
    dkg::{
        dummy_dkg::{DummyDKG, DummyDKGTranscript},
        DKGSessionMetadata, DKGTrait, DKGTranscript, DKGTranscriptMetadata,
    },
    epoch_state::EpochState,
    validator_verifier::{
        ValidatorConsensusInfo, ValidatorConsensusInfoMoveStruct, ValidatorVerifier,
    },
};
use move_core_types::account_address::AccountAddress;
use std::sync::Arc;

#[test]
fn test_transcript_aggregation_state() {
    let num_validators = 5;
    let epoch = 999;
    let addrs: Vec<AccountAddress> = (0..num_validators)
        .map(|_| AccountAddress::random())
        .collect();
    let private_keys: Vec<bls12381_keys::PrivateKey> = (0..num_validators)
        .map(|_| bls12381_keys::PrivateKey::generate_for_testing())
        .collect();
    let public_keys: Vec<bls12381_keys::PublicKey> = (0..num_validators)
        .map(|i| bls12381_keys::PublicKey::from(&private_keys[i]))
        .collect();
    let voting_powers = [1, 1, 1, 6, 6]; // total voting power: 15, default threshold: 11
    let validator_infos: Vec<ValidatorConsensusInfo> = (0..num_validators)
        .map(|i| ValidatorConsensusInfo::new(addrs[i], public_keys[i].clone(), voting_powers[i]))
        .collect();
    let validator_consensus_info_move_structs = validator_infos
        .clone()
        .into_iter()
        .map(ValidatorConsensusInfoMoveStruct::from)
        .collect::<Vec<_>>();
    let verifier = ValidatorVerifier::new(validator_infos.clone());
    let pub_params = DummyDKG::new_public_params(&DKGSessionMetadata {
        dealer_epoch: 999,
        dealer_validator_set: validator_consensus_info_move_structs.clone(),
        target_validator_set: validator_consensus_info_move_structs.clone(),
    });
    let epoch_state = Arc::new(EpochState { epoch, verifier });
    let trx_agg_state = Arc::new(TranscriptAggregationState::<DummyDKG>::new(
        pub_params,
        epoch_state,
    ));

    let good_transcript = DummyDKGTranscript::default();
    let good_trx_bytes = bcs::to_bytes(&good_transcript).unwrap();

    // Node with incorrect epoch should be rejected.
    let result = trx_agg_state.add(addrs[0], DKGTranscript {
        metadata: DKGTranscriptMetadata {
            epoch: 998,
            author: addrs[0],
        },
        transcript_bytes: good_trx_bytes.clone(),
    });
    assert!(result.is_err());

    // Node authored by X but sent by Y should be rejected.
    let result = trx_agg_state.add(addrs[1], DKGTranscript {
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
    let result = trx_agg_state.add(addrs[2], DKGTranscript {
        metadata: DKGTranscriptMetadata {
            epoch: 999,
            author: addrs[2],
        },
        transcript_bytes: vec![],
    });
    assert!(result.is_err());

    // Good node should be accepted.
    let result = trx_agg_state.add(addrs[3], DKGTranscript {
        metadata: DKGTranscriptMetadata {
            epoch: 999,
            author: addrs[3],
        },
        transcript_bytes: good_trx_bytes.clone(),
    });
    assert!(matches!(result, Ok(None)));

    // Node from contributed author should be ignored.
    let result = trx_agg_state.add(addrs[3], DKGTranscript {
        metadata: DKGTranscriptMetadata {
            epoch: 999,
            author: addrs[3],
        },
        transcript_bytes: good_trx_bytes.clone(),
    });
    assert!(matches!(result, Ok(None)));

    // Aggregated trx should be returned if after adding a node, the threshold is exceeded.
    let result = trx_agg_state.add(addrs[4], DKGTranscript {
        metadata: DKGTranscriptMetadata {
            epoch: 999,
            author: addrs[4],
        },
        transcript_bytes: good_trx_bytes.clone(),
    });
    assert!(matches!(result, Ok(Some(_))));
}
