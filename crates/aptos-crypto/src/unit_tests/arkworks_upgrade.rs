// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use ark_bls12_381::{Bls12_381 as Bls12_381New, G1Projective as G1New, G2Projective as G2New};
use ark_bls12_381_old::{Bls12_381 as Bls12_381Old, G1Projective as G1Old, G2Projective as G2Old};
use ark_ec::{pairing::Pairing as PairingNew, CurveGroup, PrimeGroup};
use ark_ec_old::{pairing::Pairing as PairingOld, CurveGroup as CurveGroupOld, Group};
use ark_ff::{BigInt as BigIntNew, BigInteger, Field, PrimeField, UniformRand};
use ark_ff_old::{
    BigInt as BigIntOld, BigInteger as BigIntegerOld, Field as OldField,
    PrimeField as PrimeFieldOld, Zero,
};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_serialize_old::{
    CanonicalDeserialize as CanonicalDeserializeOld, CanonicalSerialize as CanonicalSerializeOld,
};
use ark_std::{rand::RngCore, test_rng};

#[test]
fn test_bigint_layout_compatibility() {
    let mut rng = test_rng();
    let mut cases = vec![0u64, 1, u64::MAX, 1234567890123456789];

    for _ in 0..100 {
        cases.push(rng.next_u64());
    }

    for (i, &n) in cases.iter().enumerate() {
        let old = BigIntOld::<4>::from(n);
        let new = BigIntNew::<4>::from(n);

        let bits_old = old.to_bits_le();
        let bits_new = new.to_bits_le();
        assert_eq!(bits_old, bits_new, "Bit mismatch case {} (value={})", i, n);

        let new_from_old = BigIntNew::<4>::from_bits_le(&bits_old);
        assert_eq!(new_from_old.to_bits_le(), bits_old);

        let old_from_new = BigIntOld::<4>::from_bits_le(&bits_new);
        assert_eq!(old_from_new.to_bits_le(), bits_new);
    }
}

#[cfg(test)]
fn roundtrip_old_to_new<TOld, TNew>(old: &TOld) -> TNew
where
    TOld: CanonicalSerializeOld,
    TNew: CanonicalDeserialize,
{
    let mut buf = Vec::new();
    old.serialize_compressed(&mut buf).unwrap();
    TNew::deserialize_compressed(&*buf).unwrap()
}

#[cfg(test)]
fn roundtrip_new_to_old<TOld, TNew>(old: &TOld) -> TNew
where
    TOld: CanonicalSerialize,
    TNew: CanonicalDeserializeOld,
{
    let mut buf = Vec::new();
    old.serialize_compressed(&mut buf).unwrap();
    TNew::deserialize_compressed(&*buf).unwrap()
}

#[test]
fn test_roundtrip_and_serialization() {
    let mut rng = test_rng();

    // Prepare test cases: generator + random points
    let mut test_cases = vec![G1Old::generator()];
    test_cases.extend((0..20).map(|_| G1Old::rand(&mut rng)));

    for p_old in test_cases {
        // Convert old → new
        let p_new = roundtrip_old_to_new::<_, G1New>(&p_old);

        // Generator check
        if p_old == G1Old::generator() {
            assert_eq!(p_new, G1New::generator());
        }

        // Roundtrip old → new → old
        let p_old_back = roundtrip_new_to_old::<_, G1Old>(&p_new);
        assert_eq!(p_old_back, p_old, "Roundtrip old → new → old failed");

        // Serialization compatibility
        let mut buf_old = Vec::new();
        p_old.serialize_compressed(&mut buf_old).unwrap();

        let mut buf_new = Vec::new();
        p_new.serialize_compressed(&mut buf_new).unwrap();

        assert_eq!(buf_old.len(), buf_new.len());
        assert_eq!(buf_old, buf_new, "Compressed serialization mismatch");
    }
}

#[test]
fn test_addition_consistency() {
    let mut rng = test_rng();
    let mut points_old = vec![G1Old::generator(), G1Old::generator() + G1Old::generator()];

    for _ in 0..10 {
        points_old.push(G1Old::rand(&mut rng));
    }

    for (i, p_old) in points_old.iter().enumerate() {
        let sum_old = *p_old + *p_old;
        let p_new = roundtrip_old_to_new::<_, G1New>(p_old);
        let sum_new = roundtrip_old_to_new::<_, G1New>(&sum_old);

        assert_eq!(p_new + p_new, sum_new, "Addition mismatch in case {}", i);
    }

    // Pairwise random sums
    for _ in 0..10 {
        let p_old = G1Old::rand(&mut rng);
        let q_old = G1Old::rand(&mut rng);
        let sum_old = p_old + q_old;

        let p_new = roundtrip_old_to_new::<_, G1New>(&p_old);
        let q_new = roundtrip_old_to_new::<_, G1New>(&q_old);
        let sum_new = roundtrip_old_to_new::<_, G1New>(&sum_old);

        assert_eq!(p_new + q_new, sum_new);
    }
}

#[test]
fn test_scalar_multiplication_consistency() {
    let mut rng = test_rng();

    for _ in 0..10 {
        let scalar_old = ark_bls12_381_old::Fr::rand(&mut rng);
        let scalar_new = roundtrip_old_to_new::<_, ark_bls12_381::Fr>(&scalar_old);

        let g_old = G1Old::generator().mul_bigint(scalar_old.into_bigint());
        let g_new_expected = G1New::generator().mul_bigint(scalar_new.into_bigint());

        let g_new = roundtrip_old_to_new::<_, G1New>(&g_old);
        assert_eq!(g_new, g_new_expected);
    }
}

#[test]
fn test_fr_operations_consistency() {
    let mut rng = test_rng();
    for _ in 0..50 {
        let a_old = ark_bls12_381_old::Fr::rand(&mut rng);
        let b_old = ark_bls12_381_old::Fr::rand(&mut rng);

        let a_new = roundtrip_old_to_new::<_, ark_bls12_381::Fr>(&a_old);
        let b_new = roundtrip_old_to_new::<_, ark_bls12_381::Fr>(&b_old);

        assert_eq!(a_old + b_old, roundtrip_new_to_old(&(a_new + b_new)));
        assert_eq!(a_old - b_old, roundtrip_new_to_old(&(a_new - b_new)));
        assert_eq!(a_old * b_old, roundtrip_new_to_old(&(a_new * b_new)));

        if !a_old.is_zero() {
            assert_eq!(
                a_old.inverse().unwrap(),
                roundtrip_new_to_old(&a_new.inverse().unwrap())
            );
        }
    }
}

#[test]
fn test_pairing_consistency() {
    let mut rng = test_rng();

    // Random pairing checks
    for _ in 0..10 {
        let a_old = G1Old::rand(&mut rng);
        let b_old = G2Old::rand(&mut rng);

        let a_new = roundtrip_old_to_new::<_, G1New>(&a_old);
        let b_new = roundtrip_old_to_new::<_, G2New>(&b_old);

        let e_old = <Bls12_381Old as PairingOld>::pairing(a_old.into_affine(), b_old.into_affine());
        let e_new = <Bls12_381New as PairingNew>::pairing(a_new.into_affine(), b_new.into_affine());

        let mut buf_old = Vec::new();
        e_old.serialize_compressed(&mut buf_old).unwrap();

        let mut buf_new = Vec::new();
        e_new.serialize_compressed(&mut buf_new).unwrap();

        assert_eq!(buf_old, buf_new, "Pairing mismatch on random inputs");
    }
}
