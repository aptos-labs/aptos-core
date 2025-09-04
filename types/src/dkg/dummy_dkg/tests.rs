// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dkg::{
        dummy_dkg::{DummyDKG, DummyDKGTranscript, DummySecret},
        DKGSessionMetadata, DKGTrait,
    },
    on_chain_config::OnChainRandomnessConfig,
    validator_verifier::{ValidatorConsensusInfo, ValidatorConsensusInfoMoveStruct},
};
use velor_crypto::{bls12381, Uniform};
use move_core_types::account_address::AccountAddress;
use rand::thread_rng;

struct DealerState {
    addr: AccountAddress,
    voting_power: u64,
    sk: bls12381::PrivateKey,
    pk: bls12381::PublicKey,
    input_secret: DummySecret,
    transcript: Option<DummyDKGTranscript>,
}

impl DealerState {
    fn as_validator_consensus_info(&self) -> ValidatorConsensusInfo {
        ValidatorConsensusInfo {
            address: self.addr,
            public_key: self.pk.clone(),
            voting_power: self.voting_power,
        }
    }
}

struct NewValidatorState {
    addr: AccountAddress,
    voting_power: u64,
    sk: bls12381::PrivateKey,
    pk: bls12381::PublicKey,
    secret_share: Option<DummySecret>,
}

impl NewValidatorState {
    fn as_validator_consensus_info(&self) -> ValidatorConsensusInfo {
        ValidatorConsensusInfo {
            address: self.addr,
            public_key: self.pk.clone(),
            voting_power: self.voting_power,
        }
    }
}

#[test]
fn test_dummy_dkg_correctness() {
    let mut rng = thread_rng();

    // Initialize the current validator states. Also prepare their DKG input secrets.
    let mut dealer_states: Vec<DealerState> = (0..3)
        .map(|_| {
            let sk = bls12381::PrivateKey::generate_for_testing();
            let pk = bls12381::PublicKey::from(&sk);
            let input_secret = DummySecret::generate_for_testing();
            DealerState {
                addr: AccountAddress::random(),
                voting_power: 1,
                sk,
                pk,
                input_secret,
                transcript: None,
            }
        })
        .collect();
    let dealer_infos: Vec<ValidatorConsensusInfoMoveStruct> = dealer_states
        .iter()
        .map(|ds| ds.as_validator_consensus_info().into())
        .collect();

    // Initialize the next validator states.
    let mut new_validator_states: Vec<NewValidatorState> = (0..4)
        .map(|_| {
            let sk = bls12381::PrivateKey::generate_for_testing();
            let pk = bls12381::PublicKey::from(&sk);
            NewValidatorState {
                addr: AccountAddress::random(),
                voting_power: 2,
                sk,
                pk,
                secret_share: None,
            }
        })
        .collect();
    let new_validator_infos: Vec<ValidatorConsensusInfoMoveStruct> = new_validator_states
        .iter()
        .map(|nvi| nvi.as_validator_consensus_info().into())
        .collect();

    // Now imagine DKG starts.
    let dkg_session_metadata = DKGSessionMetadata {
        dealer_epoch: 999,
        randomness_config: OnChainRandomnessConfig::default_enabled().into(),
        dealer_validator_set: dealer_infos.clone(),
        target_validator_set: new_validator_infos.clone(),
    };

    let pub_params = DummyDKG::new_public_params(&dkg_session_metadata);
    // Every current validator generates a transcript.
    for (idx, dealer_state) in dealer_states.iter_mut().enumerate() {
        let trx = DummyDKG::generate_transcript(
            &mut rng,
            &pub_params,
            &dealer_state.input_secret,
            idx as u64,
            &dealer_state.sk,
        );
        assert!(DummyDKG::verify_transcript(&pub_params, &trx).is_ok());
        dealer_state.transcript = Some(trx);
    }

    // Aggregate all individual transcripts.
    let all_transcripts: Vec<DummyDKGTranscript> = dealer_states
        .iter()
        .map(|state| state.transcript.clone().unwrap())
        .collect();
    let mut agg_transcript = DummyDKGTranscript::default();
    all_transcripts.into_iter().for_each(|trx| {
        DummyDKG::aggregate_transcripts(&pub_params, &mut agg_transcript, trx);
    });

    assert!(DummyDKG::verify_transcript(&pub_params, &agg_transcript).is_ok());

    // Optional check: bad transcript should be rejected.
    let mut mauled_agg_transcript = agg_transcript.clone();
    mauled_agg_transcript.secret.val = !mauled_agg_transcript.secret.val;
    assert!(DummyDKG::verify_transcript(&pub_params, &mauled_agg_transcript).is_err());

    // Every new validator decrypt their own secret share.
    for (idx, nvi) in new_validator_states.iter_mut().enumerate() {
        let (secret, _pub_key) = DummyDKG::decrypt_secret_share_from_transcript(
            &pub_params,
            &agg_transcript,
            idx as u64,
            &nvi.sk,
        )
        .unwrap();
        nvi.secret_share = Some(secret);
    }

    // The dealt secret should be reconstructable.
    let player_share_pairs = new_validator_states
        .iter()
        .enumerate()
        .map(|(idx, nvi)| (idx as u64, nvi.secret_share.unwrap()))
        .collect();
    let dealt_secret_from_reconstruct =
        DummyDKG::reconstruct_secret_from_shares(&pub_params, player_share_pairs).unwrap();

    let all_input_secrets = dealer_states.iter().map(|ds| ds.input_secret).collect();
    let dealt_secret_from_input = DummyDKG::dealt_secret_from_input(
        &pub_params,
        &DummyDKG::aggregate_input_secret(all_input_secrets),
    );
    assert_eq!(dealt_secret_from_reconstruct, dealt_secret_from_input);
}
