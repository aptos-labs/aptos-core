// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use super::runner::run_ts;
use crate::group::{Fr, G1Affine, G2Affine};
use ark_ec::{bn::Bn, pairing::Pairing, AffineRepr};
use ark_ff::{BigInteger, PrimeField, UniformRand};
use ark_serialize::{CanonicalDeserializeWithFlags, CanonicalSerialize, Compress, EmptyFlags};
use ark_std::rand::thread_rng;

#[test]
#[ignore]
fn test_g1_serialization() {
    let mut rng = thread_rng();
    let rand_exponent: Fr = Fr::rand(&mut rng);
    let g1: G1Affine = (G1Affine::generator() * rand_exponent).into();
    let mut rust_result = vec![];
    g1.serialize_with_mode(&mut rust_result, Compress::Yes)
        .unwrap();

    let input: Vec<u8> = rand_exponent.into_bigint().to_bytes_le();
    let ts_result = run_ts("g1_serialization", &input).unwrap();

    assert_eq!(rust_result, ts_result);
}

#[test]
#[ignore]
fn test_g2_serialization() {
    let mut rng = thread_rng();
    let rand_exponent: Fr = Fr::rand(&mut rng);
    let g2: G2Affine = (G2Affine::generator() * rand_exponent).into();
    let mut rust_result = vec![];
    g2.serialize_with_mode(&mut rust_result, Compress::Yes)
        .unwrap();

    let input: Vec<u8> = rand_exponent.into_bigint().to_bytes_le();
    let ts_result = run_ts("g2_serialization", &input).unwrap();

    assert_eq!(rust_result, ts_result);
}

type TargetField = <Bn<ark_bn254::Config> as Pairing>::TargetField;

#[test]
#[ignore]
fn test_fp12_serialization() {
    let mut rng = thread_rng();
    let x = TargetField::rand(&mut rng);
    let rust_result = x + x;

    let mut input = vec![];
    x.serialize_with_mode(&mut input, Compress::Yes).unwrap();

    let ts_result_bytes = run_ts("leBytesToFp12", &input).unwrap();
    let (ts_result, _) =
        TargetField::deserialize_with_flags::<_, EmptyFlags>(ts_result_bytes.as_slice()).unwrap();

    assert_eq!(rust_result, ts_result);
}
