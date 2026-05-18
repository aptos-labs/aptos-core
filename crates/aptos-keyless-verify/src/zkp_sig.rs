// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
//
// Vendored from aptos-core/types/src/keyless/zkp_sig.rs @ rev 8ec3fb76.

use crate::groth16_sig::Groth16Proof;
use serde::{Deserialize, Serialize};

/// Discriminant for `ZkProof`. Currently only Groth16 is defined; future
/// proof systems would add variants here.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum ZkpVariant {
    Groth16 = 0,
}

/// BCS-tagged enum wrapping the inner proof.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum ZkProof {
    Groth16Zkp(Groth16Proof),
}
