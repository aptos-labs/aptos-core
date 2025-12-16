// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Copied from https://github.com/arkworks-rs/algebra/issues/178#issuecomment-1413219278

use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, Compress, Validate};

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

/// This module contains unit tests for serializing and deserializing
/// elliptic curve points on the BN254 curve using Serde with custom
/// serialization and deserialization functions (`ark_se` and `ark_de`).
#[cfg(test)]
pub mod tests {
    use super::*;
    use ark_bn254::{G1Projective, G1Affine, G2Projective, G2Affine};
    use ark_ec::AffineRepr as _;
    use ark_ff::{BigInteger, PrimeField};
    use serde::{Deserialize, Serialize};

    #[test]
    fn test_g1_serialization() {
        #[derive(Serialize, Deserialize)]
        struct A(#[serde(serialize_with = "ark_se", deserialize_with = "ark_de")] G1Affine);

        let g1 = G1Affine::zero();
        println!("{:?}", bcs::to_bytes(&A(g1)));
        let mut g1 = G1Affine::generator();
        println!("{:?}", bcs::to_bytes(&A(g1)));
        g1 = (g1 + G1Projective::from(g1)).into();
        println!("{:?}", bcs::to_bytes(&A(g1)));
        g1 = (g1 + G1Projective::from(g1)).into();
        println!("{:?}", bcs::to_bytes(&A(g1)));
        g1 = (g1 + G1Projective::from(g1)).into();
        println!("{:?}", bcs::to_bytes(&A(g1)));
    }

    #[test]
    fn test_g2_serialization() {
        #[derive(Serialize, Deserialize)]
        struct A(#[serde(serialize_with = "ark_se", deserialize_with = "ark_de")] G2Affine);

        let g2 = G2Affine::zero();
        println!("{:?}", bcs::to_bytes(&A(g2)));
        let mut g2 = G2Affine::generator();
        println!("{:?}", bcs::to_bytes(&A(g2)));
        println!("{:?}", g2.x.c1.into_bigint().to_bytes_le());
        g2 = (g2 + G2Projective::from(g2)).into();
        println!("{:?}", bcs::to_bytes(&A(g2)));
        println!("{:?}", g2.x.c1.into_bigint().to_bytes_le());
        g2 = (g2 + G2Projective::from(g2)).into();
        println!("{:?}", bcs::to_bytes(&A(g2)));
        println!("{:?}", g2.x.c1.into_bigint().to_bytes_le());
        g2 = (g2 + G2Projective::from(g2)).into();
        println!("{:?}", bcs::to_bytes(&A(g2)));
        println!("{:?}", g2.x.c1.into_bigint().to_bytes_le());
    }
}
