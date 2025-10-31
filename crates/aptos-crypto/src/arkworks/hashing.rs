// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Provides an "unsafe" hash-to-curve implementation for elliptic curve affine points.

use ark_ec::AffineRepr;
use digest::Digest;

/// Iteration behavior and failure probability:
///
/// By Hasse's theorem, the order of the elliptic curve is approximately equal
/// to the order of the underlying field Fq. Each x-coordinate either corresponds
/// to exactly two curve points, (x, y) and (x, -y), or to zero points.
///
/// As a result, each iteration of this algorithm has roughly a 50% chance of producing
/// a valid point when given a uniformly random input (assuming the hash function behaves
/// as a random oracle). Consequently, the probability that this function fails on a random
/// input is approximately 1/2^256.
///
/// Note: This algorithm is probabilistic and may be vulnerable to
/// side-channel attacks. For more details, see `MapToGroup` in:
/// Boneh, D., Lynn, B., & Shacham, H. (2004). "Short Signatures from the Weil Pairing."
/// Journal of Cryptology, 17, 297–319. DOI: 10.1007/s00145-004-0314-9.
/// <https://doi.org/10.1007/s00145-004-0314-9>
/// 
/// For RFC9380 see: https://www.rfc-editor.org/rfc/rfc9380.html
pub fn unsafe_hash_to_affine<P: AffineRepr>(msg: &[u8], dst: &[u8]) -> P {
    let dst_len =
        u8::try_from(dst.len()).expect("DST is too long; its length must be <= 255, as in RFC 9380 (Section 5.3.1)");

    let mut buf = Vec::with_capacity(msg.len() + dst.len() + 1);
    buf.extend_from_slice(msg);
    buf.extend_from_slice(dst);
    buf.push(dst_len);
    buf.push(0); // placeholder for counter

    for ctr in 0..=u8::MAX {
        *buf.last_mut()
            .expect("Could not access last byte of buffer") = ctr;

        let hashed = sha3::Sha3_512::digest(&buf);

        // `from_random_bytes()` first tries to construct an x-coordinate, and then a y-coordinate from that, see e.g.:
        // https://github.com/arkworks-rs/algebra/blob/c1f4f5665504154a9de2345f464b0b3da72c28ec/ec/src/models/short_weierstrass/affine.rs#L264
        if let Some(p) = P::from_random_bytes(&hashed) {
            return p.mul_by_cofactor(); // is needed to ensure that `p` lies in the prime order subgroup
        }
    }

    panic!("Failed to hash to affine group element");
}

#[cfg(test)]
mod test_hash_to_affine {
    use super::*;
    use ark_ec::short_weierstrass;

    // Restricting this test to short Weierstrass curves because that's needed
    // for `is_on_curve()` and `is_in_correct_subgroup_assuming_on_curve()`
    fn test_point_validity<C>()
    where
        C: short_weierstrass::SWCurveConfig,
    {
        let msg = b"point validity test";
        let dst = b"domain";

        let p: short_weierstrass::Affine<C> = unsafe_hash_to_affine(msg, dst);

        assert!(p.is_on_curve(), "Point is not on the curve");
        assert!(
            p.is_in_correct_subgroup_assuming_on_curve(),
            "Point is not in the correct subgroup"
        );
    }

    fn test_determinism<P: AffineRepr>() {
        let msg = b"hello world";
        let dst = b"my-domain-separator";

        let p1: P = unsafe_hash_to_affine(msg, dst);
        let p2: P = unsafe_hash_to_affine(msg, dst);

        assert_eq!(p1, p2);
    }

    fn test_domain_separation<P: AffineRepr>() {
        let msg = b"hello world";
        let p1: P = unsafe_hash_to_affine(msg, b"dst-1");
        let p2: P = unsafe_hash_to_affine(msg, b"dst-2");

        assert_ne!(p1, p2);
    }

    fn test_message_sensitivity<P: AffineRepr>() {
        let dst = b"test-domain";
        let p1: P = unsafe_hash_to_affine(b"abc", dst);
        let p2: P = unsafe_hash_to_affine(b"abd", dst);

        assert_ne!(p1, p2);
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
