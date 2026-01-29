// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::pvss::signed::GenericSigning;
use ark_ec::pairing::Pairing;

pub mod chunked_elgamal;
pub mod chunked_elgamal_pp;
pub mod chunked_scalar_mul; // needs to be `pub` for tests
pub mod chunks;
mod hkzg_chunked_elgamal;
mod hkzg_chunked_elgamal_commit;
mod input_secret;
mod keys;
pub mod public_parameters;
mod weighted_transcript;
mod weighted_transcriptv2;

pub use input_secret::InputSecret;
pub use keys::EncryptPubKey;
pub use public_parameters::{PublicParameters, DEFAULT_ELL_FOR_TESTING};
pub use weighted_transcript::{
    Subtranscript as WeightedSubtranscript, Transcript as UnsignedWeightedTranscript,
};
pub use weighted_transcriptv2::{
    Subtranscript as WeightedSubtranscriptv2, Transcript as UnsignedWeightedTranscriptv2,
};
#[allow(type_alias_bounds)]
pub type SignedWeightedTranscript<E: Pairing> = GenericSigning<UnsignedWeightedTranscript<E>>;
#[allow(type_alias_bounds)]
pub type SignedWeightedTranscriptv2<E: Pairing> = GenericSigning<UnsignedWeightedTranscriptv2<E>>;
