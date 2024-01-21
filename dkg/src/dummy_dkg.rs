// Copyright © Aptos Foundation

use aptos_config::config::IdentityBlob;
use aptos_crypto::bls12381;
use aptos_types::dkg::{DKGPrivateParamsProvider, DKGTrait};
use rand::CryptoRng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct DummyDKG {}

impl DKGTrait for DummyDKG {
    type PrivateParams = bls12381::PrivateKey;
    type PublicParams = ();
    type Transcript = DummyDKGTranscript;

    fn generate_transcript<R: CryptoRng>(
        _rng: &mut R,
        _sk: &Self::PrivateParams,
        _params: &Self::PublicParams,
    ) -> Self::Transcript {
        DummyDKGTranscript::default()
    }

    fn verify_transcript(
        _params: &Self::PublicParams,
        _trx: &Self::Transcript,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn aggregate_transcripts(
        _params: &Self::PublicParams,
        _base: &mut Self::Transcript,
        _extra: &Self::Transcript,
    ) {
    }
}

impl DKGPrivateParamsProvider<DummyDKG> for Arc<IdentityBlob> {
    fn dkg_private_params(&self) -> &<DummyDKG as DKGTrait>::PrivateParams {
        self.consensus_private_key.as_ref().unwrap()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DummyDKGTranscript {
    data: Vec<u8>,
}

impl Default for DummyDKGTranscript {
    fn default() -> Self {
        Self {
            data: b"data".to_vec(),
        }
    }
}
