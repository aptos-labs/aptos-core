// Copyright © Aptos Foundation

use crate::{
    dkg::{DKGSessionMetadata, DKGTrait},
    validator_verifier::ValidatorConsensusInfo,
    ValidatorConsensusInfoMoveStruct,
};
use anyhow::{anyhow, ensure};
use aptos_crypto::{bls12381, Uniform};
use move_core_types::account_address::AccountAddress;
use rand::{thread_rng, CryptoRng, Rng, RngCore};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// TODO: either make a separate RealDKG and make this test-only,
/// or rename it and replace its implementation with the real one.
#[derive(Debug)]
pub struct DummyDKG {}

impl DKGTrait for DummyDKG {
    type DealerPrivateKey = bls12381::PrivateKey;
    type DealtSecret = DummySecret;
    type DealtSecretShare = DummySecret;
    type InputSecret = DummySecret;
    type NewValidatorDecryptKey = bls12381::PrivateKey;
    type PublicParams = DKGSessionMetadata;
    type Transcript = DummyDKGTranscript;

    fn new_public_params(dkg_session_metadata: &DKGSessionMetadata) -> Self::PublicParams {
        dkg_session_metadata.clone()
    }

    fn aggregate_input_secret(secrets: Vec<DummySecret>) -> DummySecret {
        DummySecret::aggregate(secrets)
    }

    fn dealt_secret_from_input(input: &Self::InputSecret) -> Self::DealtSecret {
        *input
    }

    fn generate_transcript<R: CryptoRng>(
        _rng: &mut R,
        _params: &Self::PublicParams,
        input_secret: &Self::InputSecret,
        my_index: u64,
        _sk: &Self::DealerPrivateKey,
    ) -> Self::Transcript {
        DummyDKGTranscript {
            secret: *input_secret,
            contributions_by_dealer: BTreeMap::from([(my_index, *input_secret)]),
        }
    }

    fn verify_transcript(
        _params: &Self::PublicParams,
        transcript: &Self::Transcript,
    ) -> anyhow::Result<()> {
        let secret_another = DummyDKG::aggregate_input_secret(
            transcript
                .contributions_by_dealer
                .values()
                .copied()
                .collect::<Vec<_>>(),
        );
        ensure!(transcript.secret == secret_another);
        Ok(())
    }

    fn aggregate_transcripts(
        _params: &Self::PublicParams,
        transcripts: Vec<DummyDKGTranscript>,
    ) -> DummyDKGTranscript {
        let mut all_secrets = vec![];
        let mut agg_contributions_by_dealer = BTreeMap::new();
        for transcript in transcripts {
            let DummyDKGTranscript {
                secret,
                contributions_by_dealer,
            } = transcript;
            all_secrets.push(secret);
            agg_contributions_by_dealer.extend(contributions_by_dealer);
        }
        DummyDKGTranscript {
            secret: DummySecret::aggregate(all_secrets),
            contributions_by_dealer: agg_contributions_by_dealer,
        }
    }

    fn decrypt_secret_share_from_transcript(
        _pub_params: &Self::PublicParams,
        transcript: &DummyDKGTranscript,
        _player_idx: u64,
        _dk: &Self::NewValidatorDecryptKey,
    ) -> anyhow::Result<DummySecret> {
        Ok(transcript.secret)
    }

    fn reconstruct_secret_from_shares(
        _pub_params: &Self::PublicParams,
        player_share_pairs: Vec<(u64, Self::DealtSecretShare)>,
    ) -> anyhow::Result<Self::DealtSecret> {
        let mut secret = None;
        for (_, secret_share) in player_share_pairs {
            if let Some(s) = secret.as_ref() {
                ensure!(*s == secret_share);
            } else {
                secret = Some(secret_share);
            }
        }
        secret.ok_or_else(|| anyhow!("zero shares"))
    }

    fn get_dealers(transcript: &DummyDKGTranscript) -> BTreeSet<u64> {
        transcript.contributions_by_dealer.keys().copied().collect()
    }

    fn generate_predictable_input_secret_for_testing(dealer_sk: &bls12381::PrivateKey) -> DummySecret {
        let bytes_8: [u8; 8] = dealer_sk.to_bytes()[0..8].try_into().unwrap();
        DummySecret {
            val: u64::from_be_bytes(bytes_8),
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DummySecret {
    val: u64,
}

impl DummySecret {
    pub fn aggregate(secrets: Vec<Self>) -> Self {
        let mut ret = 0;
        for secret in secrets {
            ret ^= secret.val;
        }
        Self { val: ret }
    }
}

impl Uniform for DummySecret {
    fn generate<R>(rng: &mut R) -> Self
    where
        R: RngCore + CryptoRng,
    {
        Self { val: rng.gen() }
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct DummyDKGTranscript {
    secret: DummySecret,
    contributions_by_dealer: BTreeMap<u64, DummySecret>,
}

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
fn test_dummy_dkg() {
    let mut rng = thread_rng();
    let mut dealer_states: Vec<DealerState> = (0..3)
        .map(|_| {
            let sk = bls12381::PrivateKey::generate_for_testing();
            let pk = bls12381::PublicKey::from(&sk);
            let input_secret = derive_input_secret_for_testing(&sk);
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
    let dkg_session_metadata = DKGSessionMetadata {
        dealer_epoch: 999,
        dealer_validator_set: dealer_infos.clone(),
        target_validator_set: new_validator_infos.clone(),
    };
    let pub_params = DummyDKG::new_public_params(&dkg_session_metadata);
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

    let all_transcripts: Vec<DummyDKGTranscript> = dealer_states
        .iter()
        .map(|state| state.transcript.clone().unwrap())
        .collect();
    let agg_transcript = DummyDKG::aggregate_transcripts(&pub_params, all_transcripts);
    assert!(DummyDKG::verify_transcript(&pub_params, &agg_transcript).is_ok());
    let mut mauled_agg_transcript = agg_transcript.clone();
    mauled_agg_transcript.secret.val = !mauled_agg_transcript.secret.val;
    assert!(DummyDKG::verify_transcript(&pub_params, &mauled_agg_transcript).is_err());

    for (idx, nvi) in new_validator_states.iter_mut().enumerate() {
        nvi.secret_share = DummyDKG::decrypt_secret_share_from_transcript(
            &pub_params,
            &agg_transcript,
            idx as u64,
            &nvi.sk,
        )
        .ok()
    }

    let player_share_pairs = new_validator_states
        .iter()
        .enumerate()
        .map(|(idx, nvi)| (idx as u64, nvi.secret_share.clone().unwrap()))
        .collect();
    let dealt_secret_from_reconstruct =
        DummyDKG::reconstruct_secret_from_shares(&pub_params, player_share_pairs).unwrap();

    let all_input_secrets = dealer_states
        .iter()
        .map(|ds| ds.input_secret.clone())
        .collect();
    let dealt_secret_from_input =
        DummyDKG::dealt_secret_from_input(&DummyDKG::aggregate_input_secret(all_input_secrets));
    assert_eq!(dealt_secret_from_reconstruct, dealt_secret_from_input);
}
