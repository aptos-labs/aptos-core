// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dkg::{DKGSessionMetadata, DKGTrait},
    validator_verifier::ValidatorVerifier,
};
use anyhow::{anyhow, ensure};
use velor_crypto::{bls12381, Uniform};
use move_core_types::account_address::AccountAddress;
use rand::{CryptoRng, Rng, RngCore};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// TODO: either make a separate RealDKG and make this test-only,
/// or rename it and replace its implementation with the real one.
#[derive(Debug)]
pub struct DummyDKG {}

impl DKGTrait for DummyDKG {
    type DealerPrivateKey = bls12381::PrivateKey;
    type DealtPubKeyShare = ();
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

    fn dealt_secret_from_input(
        _pub_params: &Self::PublicParams,
        input: &Self::InputSecret,
    ) -> Self::DealtSecret {
        *input
    }

    fn generate_transcript<R: CryptoRng + RngCore>(
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

    fn verify_transcript_extra(
        _trx: &Self::Transcript,
        _verifier: &ValidatorVerifier,
        _checks_voting_power: bool,
        _ensures_single_dealer: Option<AccountAddress>,
    ) -> anyhow::Result<()> {
        Ok(())
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
        accumulator: &mut Self::Transcript,
        element: Self::Transcript,
    ) {
        let DummyDKGTranscript {
            secret,
            contributions_by_dealer,
        } = element;
        accumulator
            .contributions_by_dealer
            .extend(contributions_by_dealer);
        accumulator.secret =
            DummySecret::aggregate(vec![std::mem::take(&mut accumulator.secret), secret]);
    }

    fn decrypt_secret_share_from_transcript(
        _pub_params: &Self::PublicParams,
        transcript: &DummyDKGTranscript,
        _player_idx: u64,
        _dk: &Self::NewValidatorDecryptKey,
    ) -> anyhow::Result<(DummySecret, ())> {
        Ok((transcript.secret, ()))
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

#[cfg(test)]
mod tests;
