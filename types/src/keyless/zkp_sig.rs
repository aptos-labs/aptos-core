// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::keyless::Groth16Proof;
use velor_crypto_derive::{BCSCryptoHash, CryptoHasher};
use serde::{Deserialize, Serialize};

#[derive(
    Copy, Clone, Debug, Deserialize, PartialEq, Eq, Hash, Serialize, CryptoHasher, BCSCryptoHash,
)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
pub enum ZKP {
    Groth16(Groth16Proof),
}

impl From<Groth16Proof> for ZKP {
    fn from(proof: Groth16Proof) -> Self {
        ZKP::Groth16(proof)
    }
}

impl From<ZKP> for Groth16Proof {
    fn from(zkp: ZKP) -> Self {
        match zkp {
            ZKP::Groth16(proof) => proof,
        }
    }
}
