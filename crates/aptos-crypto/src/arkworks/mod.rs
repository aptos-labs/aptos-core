// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module provides some helper functions for arkworks.

pub mod mult_tree;
pub mod serialization;
pub mod shamir;

use ark_ec::{pairing::Pairing, AffineRepr, CurveGroup};
use ark_ff::{BigInteger, PrimeField};
use ark_poly::EvaluationDomain;
use digest::Digest;

/// Returns the first `ell` powers of two as scalar field elements, so
/// [1, 2, 4, 8, 16, ..., 2^{ell - 1}]
pub fn powers_of_two<E: Pairing>(ell: usize) -> Vec<E::ScalarField> {
    (0..ell).map(|j| E::ScalarField::from(1u64 << j)).collect()
}

/// Commit to scalars by multiplying a base group element with each scalar.
///
/// Equivalent to `[base * s for s in scalars]`.
pub fn commit_to_scalars<G, F>(commitment_base: &G, scalars: &[F]) -> Vec<G>
where
    G: CurveGroup<ScalarField = F>,
    F: PrimeField,
{
    scalars.iter().map(|s| *commitment_base * s).collect()
}

// TODO: There's probably a better way to do this?
/// Converts a prime field scalar into a `u32`, if possible.
pub fn scalar_to_u32<F: ark_ff::PrimeField>(scalar: &F) -> Option<u32> {
    let mut bytes = scalar.into_bigint().to_bytes_le();

    while bytes.last() == Some(&0) {
        bytes.pop();
    }

    if bytes.len() > 4 {
        // More than 4 bytes → cannot fit in u32
        return None;
    }

    // Pad bytes to 4 bytes for u32 conversion
    let mut padded = [0u8; 4];
    padded[..bytes.len()].copy_from_slice(&bytes);

    Some(u32::from_le_bytes(padded))
}

/// Computes all `num_omegas`-th roots of unity in the scalar field, where `num_omegas` must be a power of two.
pub fn compute_roots_of_unity<E: Pairing>(num_omegas: usize) -> Vec<E::ScalarField> {
    let eval_dom = ark_poly::Radix2EvaluationDomain::<E::ScalarField>::new(num_omegas)
        .expect("Could not reconstruct evaluation domain");
    eval_dom.elements().collect()
}

/// Iteration behavior and failure probability:
///
/// By Hasse's theorem, the order of the elliptic curve is approximately equal
/// to the order of the underlying field Fq. Each x-coordinate either corresponds
/// to exactly two curve points, (x, y) and (x, -y), or to zero points.
///
/// As a result, each iteration of this algorithm has roughly a 50% chance
/// of producing a valid point when given a uniformly random input
/// (assuming the hash function behaves as a random oracle). Consequently,
/// the probability that this function fails on a random input is
/// approximately 1/2^256.  
///
/// Note: This algorithm is probabilistic and may be vulnerable to
/// side-channel attacks. For more details, see `MapToGroup` in:
/// Boneh, D., Lynn, B., & Shacham, H. (2004). "Short Signatures from the Weil Pairing."
/// Journal of Cryptology, 17, 297–319. DOI: 10.1007/s00145-004-0314-9.
/// <https://doi.org/10.1007/s00145-004-0314-9>
pub fn hash_to_affine<A: AffineRepr>(msg: &[u8], dst: &[u8]) -> A {
    let dst_len =
        u8::try_from(dst.len()).expect("DST is too long; must fit in one byte, as in RFC 9380");

    let mut buf = Vec::with_capacity(msg.len() + dst.len() + 1);
    buf.extend_from_slice(msg);
    buf.extend_from_slice(dst);
    buf.push(dst_len);
    buf.push(0); // placeholder for counter

    for ctr in 0..=u8::MAX {
        *buf.last_mut()
            .expect("Could not access last byte of buffer") = ctr;

        let hashed = sha3::Sha3_512::digest(&buf);

        // from_random_bytes() first tries to construct an x-coordinate, and then a y-coordinate from that, see e.g.:
        // https://github.com/arkworks-rs/algebra/blob/c1f4f5665504154a9de2345f464b0b3da72c28ec/ec/src/models/short_weierstrass/affine.rs#L264
        if let Some(p) = A::from_random_bytes(&hashed) {
            return p.mul_by_cofactor(); // is needed to ensure `p` lies in the prime order subgroup
        }
    }

    panic!("Failed to hash to affine group element");
}

#[cfg(test)]
mod test_scalar_to_u32 {
    use super::scalar_to_u32;

    #[test]
    fn test_round_trip_for_valid_values() {
        for i in [0, 1, 42, 255, 65_535, 1_000_000, u32::MAX] {
            let scalar = ark_bn254::Fr::from(i as u64);
            assert_eq!(scalar_to_u32(&scalar), Some(i));
        }
    }
}

#[cfg(test)]
mod test_hash_to_affine {
    use super::*;

    fn serialize_affine<P: AffineRepr>(point: &P) -> Vec<u8> {
        let mut bytes = Vec::new();
        point.serialize_compressed(&mut bytes).unwrap();
        bytes
    }

    fn test_point_validity<C>()
    where
        C: ark_ec::short_weierstrass::SWCurveConfig,
    {
        let msg = b"point validity test";
        let dst = b"domain";

        let p: ark_ec::short_weierstrass::Affine<C> = hash_to_affine(msg, dst);

        assert!(p.is_on_curve(), "Point is not on the curve");
        assert!(
            p.is_in_correct_subgroup_assuming_on_curve(),
            "Point is not in the correct subgroup"
        );
    }

    fn test_determinism<P: AffineRepr>() {
        let msg = b"hello world";
        let dst = b"my-domain-separator";

        let p1: P = hash_to_affine(msg, dst);
        let p2: P = hash_to_affine(msg, dst);

        assert_eq!(serialize_affine(&p1), serialize_affine(&p2));
    }

    fn test_domain_separation<P: AffineRepr>() {
        let msg = b"hello world";
        let p1: P = hash_to_affine(msg, b"dst-1");
        let p2: P = hash_to_affine(msg, b"dst-2");

        assert_ne!(serialize_affine(&p1), serialize_affine(&p2));
    }

    fn test_message_sensitivity<P: AffineRepr>() {
        let dst = b"test-domain";
        let p1: P = hash_to_affine(b"abc", dst);
        let p2: P = hash_to_affine(b"abd", dst);

        assert_ne!(serialize_affine(&p1), serialize_affine(&p2));
    }

    #[test]
    fn bn254_tests() {
        use ark_bn254::G1Affine;

        test_point_validity::<ark_bn254::g1::Config>();
        test_determinism::<G1Affine>();
        test_domain_separation::<G1Affine>();
        test_message_sensitivity::<G1Affine>();
    }

    #[test]
    fn bls12_381_tests() {
        use ark_bls12_381::G1Affine;

        test_point_validity::<ark_bls12_381::g1::Config>();
        test_determinism::<G1Affine>();
        test_domain_separation::<G1Affine>();
        test_message_sensitivity::<G1Affine>();
    }
}
