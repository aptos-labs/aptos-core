// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
// If we want to use BN254
pub use ark_bn254::{
    g1::Config as G1Config, Bn254 as PairingSetting, Config, Fq, Fr, G1Affine, G1Projective,
    G2Affine, G2Projective,
};
use ark_ec::pairing::Pairing;

pub type G1Prepared = <ark_bn254::Bn254 as Pairing>::G1Prepared;
pub type G2Prepared = <ark_bn254::Bn254 as Pairing>::G2Prepared;
pub type PairingOutput = ark_ec::pairing::PairingOutput<ark_bn254::Bn254>;

