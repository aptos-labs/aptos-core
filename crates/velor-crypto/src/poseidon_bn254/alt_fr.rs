// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use ark_ff::{BigInteger, PrimeField as ArkPrimeField};
use ff::PrimeField;

/// Unfortunately, `neptune` is not compatible with the arkworks ff traits.
#[derive(PrimeField)]
#[PrimeFieldModulus = "21888242871839275222246405745257275088548364400416034343698204186575808495617"]
#[PrimeFieldGenerator = "5"]
#[PrimeFieldReprEndianness = "little"]
pub struct AltFr([u64; 4]);

impl From<ark_bn254::Fr> for AltFr {
    fn from(fr: ark_bn254::Fr) -> Self {
        AltFr::from_repr_vartime(AltFrRepr(
            fr.into_bigint()
                .to_bytes_le()
                .try_into()
                .expect("Expected ark_bn254::Fr to have 32 byte length"),
        ))
        .expect("The ark_bn254::Fr bytes were expected to be valid")
    }
}

impl From<AltFr> for ark_bn254::Fr {
    fn from(fr: AltFr) -> Self {
        ark_bn254::Fr::from_le_bytes_mod_order(fr.to_repr().as_ref())
    }
}

impl From<&str> for AltFr {
    fn from(hex: &str) -> Self {
        AltFr::from_str_vartime(hex).unwrap()
    }
}
