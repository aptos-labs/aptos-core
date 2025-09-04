// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    mode::per_issuer::PerIssuerMode,
    observation_aggregation::ObservationAggregationState,
    types::{ObservedUpdate, ObservedUpdateResponse},
};
use velor_crypto::{bls12381, SigningKey, Uniform};
use velor_reliable_broadcast::BroadcastStatus;
use velor_types::{
    epoch_state::EpochState,
    jwks::{
        jwk::{JWKMoveStruct, JWK},
        unsupported::UnsupportedJWK,
        ProviderJWKs, QuorumCertifiedUpdate,
    },
    validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
};
use move_core_types::account_address::AccountAddress;
use std::sync::Arc;

#[test]
fn test_observation_aggregation_state() {
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
    let epoch_state = Arc::new(EpochState::new(epoch, verifier));
    let view_0 = ProviderJWKs {
        issuer: b"https::/alice.com".to_vec(),
        version: 123,
        jwks: vec![JWKMoveStruct::from(JWK::Unsupported(
            UnsupportedJWK::new_for_testing("id1", "payload1"),
        ))],
    };
    let view_1 = ProviderJWKs {
        issuer: b"https::/alice.com".to_vec(),
        version: 123,
        jwks: vec![JWKMoveStruct::from(JWK::Unsupported(
            UnsupportedJWK::new_for_testing("id2", "payload2"),
        ))],
    };
    let ob_agg_state = Arc::new(ObservationAggregationState::<PerIssuerMode>::new(
        epoch_state.clone(),
        view_0.clone(),
    ));

    // `ObservedUpdate` with incorrect epoch should be rejected.
    let result = ob_agg_state.add(addrs[0], ObservedUpdateResponse {
        epoch: 998,
        update: ObservedUpdate {
            author: addrs[0],
            observed: view_0.clone(),
            signature: private_keys[0].sign(&view_0).unwrap(),
        },
    });
    assert!(result.is_err());

    // `ObservedUpdate` authored by X but sent by Y should be rejected.
    let result = ob_agg_state.add(addrs[1], ObservedUpdateResponse {
        epoch: 999,
        update: ObservedUpdate {
            author: addrs[0],
            observed: view_0.clone(),
            signature: private_keys[0].sign(&view_0).unwrap(),
        },
    });
    assert!(result.is_err());

    // `ObservedUpdate` that cannot be verified should be rejected.
    let result = ob_agg_state.add(addrs[2], ObservedUpdateResponse {
        epoch: 999,
        update: ObservedUpdate {
            author: addrs[2],
            observed: view_0.clone(),
            signature: private_keys[2].sign(&view_1).unwrap(),
        },
    });
    assert!(result.is_err());

    // Good `ObservedUpdate` should be accepted.
    let result = ob_agg_state.add(addrs[3], ObservedUpdateResponse {
        epoch: 999,
        update: ObservedUpdate {
            author: addrs[3],
            observed: view_0.clone(),
            signature: private_keys[3].sign(&view_0).unwrap(),
        },
    });
    assert!(matches!(result, Ok(None)));

    // `ObservedUpdate` from contributed author should be ignored.
    let result = ob_agg_state.add(addrs[3], ObservedUpdateResponse {
        epoch: 999,
        update: ObservedUpdate {
            author: addrs[3],
            observed: view_0.clone(),
            signature: private_keys[3].sign(&view_0).unwrap(),
        },
    });
    assert!(matches!(result, Ok(None)));

    // Quorum-certified update should be returned if after adding an `ObservedUpdate`, the threshold is exceeded.
    let result = ob_agg_state.add(addrs[4], ObservedUpdateResponse {
        epoch: 999,
        update: ObservedUpdate {
            author: addrs[4],
            observed: view_0.clone(),
            signature: private_keys[4].sign(&view_0).unwrap(),
        },
    });
    let QuorumCertifiedUpdate {
        update: observed,
        multi_sig,
    } = result.unwrap().unwrap();
    assert_eq!(view_0, observed);
    assert!(epoch_state
        .verifier
        .verify_multi_signatures(&observed, &multi_sig)
        .is_ok());
}
