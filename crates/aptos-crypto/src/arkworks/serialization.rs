// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Copied from https://github.com/arkworks-rs/algebra/issues/178#issuecomment-1413219278

use ark_ec::pairing::Pairing;
use ark_serialize::{
    CanonicalDeserialize, CanonicalSerialize, Compress, SerializationError, Validate, Write,
};

/// Serializes a type implementing `CanonicalSerialize` into bytes (with compression) using the
/// [`ark_serialize`](https://docs.rs/ark-serialize) format and writes it to a Serde serializer.
///
/// This is useful for integrating Arkworks types (e.g., elliptic curve elements, field elements)
/// with Serde-compatible formats such as JSON, CBOR, or MessagePack.
pub fn ark_se<S, A: CanonicalSerialize>(a: &A, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut bytes = vec![];
    a.serialize_with_mode(&mut bytes, Compress::Yes)
        .map_err(serde::ser::Error::custom)?;
    s.serialize_bytes(&bytes)
}

/// Deserializes a type implementing `CanonicalDeserialize` from bytes produced by [`ark_se`].
///
/// This function allows Arkworks types to be deserialized from Serde-compatible data sources.
/// It assumes the data was serialized with compression, and attempts to check its correctness.
pub fn ark_de<'de, D, A: CanonicalDeserialize>(data: D) -> Result<A, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let s: Vec<u8> = serde::de::Deserialize::deserialize(data)?;
    let a = A::deserialize_with_mode(s.as_slice(), Compress::Yes, Validate::Yes);
    a.map_err(serde::de::Error::custom)
}

/// TODO: Not sure this is a good idea, will probably remove it in the next PR?
pub trait BatchSerializable<E: Pairing> {
    /// Collect *all* curve elements in canonical order
    fn collect_points(&self, g1: &mut Vec<E::G1>, g2: &mut Vec<E::G2>);

    /// Serialize using already-normalized affine points
    fn serialize_from_affine<W: Write>(
        &self,
        writer: &mut W,
        compress: Compress,
        g1_iter: &mut impl Iterator<Item = E::G1Affine>,
        g2_iter: &mut impl Iterator<Item = E::G2Affine>,
    ) -> Result<(), SerializationError>;
}

/// This module contains unit tests for serializing and deserializing
/// elliptic curve points on the BN254 curve using Serde with custom
/// serialization and deserialization functions (`ark_se` and `ark_de`).
#[cfg(test)]
pub mod tests {
    use super::*;
    use ark_bn254::{G1Affine, G1Projective, G2Affine, G2Projective};
    use ark_ec::{AffineRepr as _, PrimeGroup};
    use serde::{Deserialize, Serialize};

    const MAX_DOUBLINGS: usize = 5; // Test 1G, 2G, 4G, 8G, 16G

    #[test]
    fn test_g1_serialization_multiple_points() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct A(#[serde(serialize_with = "ark_se", deserialize_with = "ark_de")] G1Affine);

        let mut points = vec![G1Affine::zero()]; // Include zero
        let mut g = G1Projective::generator();

        for _ in 0..MAX_DOUBLINGS {
            points.push(g.into());
            g += g; // double for next
        }

        for p in points {
            let serialized = bcs::to_bytes(&A(p)).expect("Serialization failed");
            let deserialized: A = bcs::from_bytes(&serialized).expect("Deserialization failed");

            assert_eq!(deserialized.0, p, "G1 point round-trip failed for {:?}", p);
        }
    }

    #[test]
    fn test_g2_serialization_multiple_points() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct A(#[serde(serialize_with = "ark_se", deserialize_with = "ark_de")] G2Affine);

        let mut points = vec![G2Affine::zero()]; // Include zero
        let mut g = G2Projective::generator();

        for _ in 0..MAX_DOUBLINGS {
            points.push(g.into());
            g += g; // double for next
        }

        for p in points {
            let serialized = bcs::to_bytes(&A(p)).expect("Serialization failed");
            let deserialized: A = bcs::from_bytes(&serialized).expect("Deserialization failed");

            assert_eq!(deserialized.0, p, "G2 point round-trip failed for {:?}", p);
        }
    }
}
