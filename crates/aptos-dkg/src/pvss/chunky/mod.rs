// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::pvss::signed::GenericSigning;
use ark_ec::pairing::Pairing;

mod chunked_elgamal;
mod chunks;
mod hkzg_chunked_elgamal;
mod input_secret;
mod keys;
mod public_parameters;
mod transcript;
mod weighted_transcript;

pub use public_parameters::DEFAULT_ELL_FOR_TESTING;
pub use transcript::{
    SubTranscript as UnweightedSubtranscript, Transcript as UnsignedUnweightedTranscript,
};
pub use weighted_transcript::{
    SubTranscript as WeightedSubtranscript, Transcript as UnsignedWeightedTranscript,
};

#[allow(type_alias_bounds)]
pub type SignedWeightedTranscript<E: Pairing> = GenericSigning<UnsignedWeightedTranscript<E>>;
#[allow(type_alias_bounds)]
pub type SignedUnweightedTranscript<E: Pairing> = GenericSigning<UnsignedUnweightedTranscript<E>>;
