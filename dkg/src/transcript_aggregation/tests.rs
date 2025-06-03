// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transcript_aggregation::TranscriptAggregationState;
use aptos_crypto::{bls12381::bls12381_keys, Uniform};
use aptos_infallible::duration_since_epoch;
use aptos_reliable_broadcast::BroadcastStatus;
use aptos_types::{
    dkg::{real_dkg::RealDKG, DKGSessionMetadata, DKGTrait, DKGTranscript, DKGTranscriptMetadata},
    epoch_state::EpochState,
    on_chain_config::OnChainRandomnessConfig,
    validator_verifier::{
        ValidatorConsensusInfo, ValidatorConsensusInfoMoveStruct, ValidatorVerifier,
    },
};
use move_core_types::account_address::AccountAddress;
use rand::thread_rng;
use std::sync::Arc;

#[test]
fn test_transcript_aggregation_state() {
    let mut rng = thread_rng();
    let num_validators = 5;
    let epoch = 999;
    let addrs: Vec<AccountAddress> = (0..num_validators)
        .map(|_| AccountAddress::random())
        .collect();
    let vfn_addr = AccountAddress::random();
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
    let pub_params = RealDKG::new_public_params(&DKGSessionMetadata {
        dealer_epoch: 999,
        randomness_config: OnChainRandomnessConfig::default_enabled().into(),
        dealer_validator_set: validator_consensus_info_move_structs.clone(),
        target_validator_set: validator_consensus_info_move_structs.clone(),
    });
    let epoch_state = Arc::new(EpochState::new(epoch, verifier));
    let trx_agg_state = Arc::new(TranscriptAggregationState::<RealDKG>::new(
        duration_since_epoch(),
        addrs[0],
        pub_params.clone(),
        epoch_state,
    ));

    let good_trx_0 =
        RealDKG::sample_secret_and_generate_transcript(&mut rng, &pub_params, 0, &private_keys[0]);
    let good_trx_0_bytes = bcs::to_bytes(&good_trx_0).unwrap();

    // Node with incorrect epoch should be rejected.
    let result = trx_agg_state.add(addrs[0], DKGTranscript {
        metadata: DKGTranscriptMetadata {
            epoch: 998,
            author: addrs[0],
        },
        transcript_bytes: good_trx_0_bytes.clone(),
    });
    assert!(result.is_err());

    // Node authored by X but sent by Y should be rejected: case 0
    let result = trx_agg_state.add(addrs[1], DKGTranscript {
        metadata: DKGTranscriptMetadata {
            epoch: 999,
            author: addrs[0],
        },
        transcript_bytes: good_trx_0_bytes.clone(),
    });
    assert!(result.is_err());

    // Node authored by X but sent by Y should be rejected: case 1
    let result = trx_agg_state.add(addrs[1], DKGTranscript {
        metadata: DKGTranscriptMetadata {
            epoch: 999,
            author: addrs[1],
        },
        transcript_bytes: good_trx_0_bytes.clone(),
    });
    assert!(result.is_err());

    // Node authored by non-active-validator should be rejected.
    let result = trx_agg_state.add(vfn_addr, DKGTranscript {
        metadata: DKGTranscriptMetadata {
            epoch: 999,
            author: vfn_addr,
        },
        transcript_bytes: good_trx_0_bytes.clone(),
    });
    assert!(result.is_err());

    // Node with invalid transcript should be rejected.
    let mut bad_trx_0_bytes = good_trx_0_bytes.clone();
    *bad_trx_0_bytes.last_mut().unwrap() = 0xAB;
    let result = trx_agg_state.add(addrs[0], DKGTranscript {
        metadata: DKGTranscriptMetadata {
            epoch: 999,
            author: addrs[0],
        },
        transcript_bytes: bad_trx_0_bytes,
    });
    assert!(result.is_err());

    // Transcript where fast-path secret and main-path secret do not match should be rejected.
    let bad_trx_2 = RealDKG::generate_transcript_for_inconsistent_secrets(
        &mut rng,
        &pub_params,
        2,
        &private_keys[2],
    );
    let result = trx_agg_state.add(addrs[2], DKGTranscript {
        metadata: DKGTranscriptMetadata {
            epoch: 999,
            author: addrs[2],
        },
        transcript_bytes: bcs::to_bytes(&bad_trx_2).unwrap(),
    });
    assert!(result.is_err());

    // Good node should be accepted.
    let good_trx_3 =
        RealDKG::sample_secret_and_generate_transcript(&mut rng, &pub_params, 3, &private_keys[3]);
    let result = trx_agg_state.add(addrs[3], DKGTranscript {
        metadata: DKGTranscriptMetadata {
            epoch: 999,
            author: addrs[3],
        },
        transcript_bytes: bcs::to_bytes(&good_trx_3).unwrap(),
    });
    println!("{:?}", result);
    assert!(matches!(result, Ok(None)));

    // Repeated contribution should be ignored.
    let good_trx_3_another =
        RealDKG::sample_secret_and_generate_transcript(&mut rng, &pub_params, 3, &private_keys[3]);
    let result = trx_agg_state.add(addrs[3], DKGTranscript {
        metadata: DKGTranscriptMetadata {
            epoch: 999,
            author: addrs[3],
        },
        transcript_bytes: bcs::to_bytes(&good_trx_3_another).unwrap(),
    });
    assert!(matches!(result, Ok(None)));

    // Aggregated trx should be returned if after adding a node, the threshold is exceeded.
    let good_trx_4 =
        RealDKG::sample_secret_and_generate_transcript(&mut rng, &pub_params, 4, &private_keys[4]);
    let result = trx_agg_state.add(addrs[4], DKGTranscript {
        metadata: DKGTranscriptMetadata {
            epoch: 999,
            author: addrs[4],
        },
        transcript_bytes: bcs::to_bytes(&good_trx_4).unwrap(),
    });
    assert!(matches!(result, Ok(Some(_))));
}
