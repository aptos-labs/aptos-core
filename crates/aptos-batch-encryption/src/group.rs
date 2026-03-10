// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
pub use ark_bls12_381::{
    g1::Config as G1Config, Bls12_381 as PairingSetting, Config, Fq, Fr, G1Affine, G1Projective,
    G2Affine, G2Projective,
};

pub type G1Prepared = <ark_bls12_381::Bls12_381 as ark_ec::pairing::Pairing>::G1Prepared;
pub type G2Prepared = <ark_bls12_381::Bls12_381 as ark_ec::pairing::Pairing>::G2Prepared;
pub type PairingOutput = ark_ec::pairing::PairingOutput<ark_bls12_381::Bls12_381>;
pub type Pairing = ark_bls12_381::Bls12_381;

// Uncommenting these lines, and commenting out the ones above, switches the crate to use the bn254 curve.
//pub use ark_bn254::{
//    g1::Config as G1Config, Bn254 as PairingSetting, Config, Fq, Fr, G1Affine, G1Projective,
//    G2Affine, G2Projective,
//};
//
//pub type G1Prepared = <ark_bn254::Bn254 as ark_ec::pairing::Pairing>::G1Prepared;
//pub type G2Prepared = <ark_bn254::Bn254 as ark_ec::pairing::Pairing>::G2Prepared;
//pub type PairingOutput = ark_ec::pairing::PairingOutput<ark_bn254::Bn254>;
//pub type Pairing = ark_bn254::Bn254;
