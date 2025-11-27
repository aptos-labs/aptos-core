// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
//! copied from https://github.com/arkworks-rs/algebra/issues/178#issuecomment-1413219278
//!
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, Compress, Validate};

pub fn ark_se<S, A: CanonicalSerialize>(a: &A, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut bytes = vec![];
    a.serialize_with_mode(&mut bytes, Compress::Yes)
        .map_err(serde::ser::Error::custom)?;
    s.serialize_bytes(&bytes)
}

pub fn ark_de<'de, D, A: CanonicalDeserialize>(data: D) -> Result<A, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let s: Vec<u8> = serde::de::Deserialize::deserialize(data)?;
    let a = A::deserialize_with_mode(s.as_slice(), Compress::Yes, Validate::No);
    a.map_err(serde::de::Error::custom)
}

#[cfg(test)]
pub mod tests {
    use crate::{
        group::{G1Affine, G1Projective, G2Affine, G2Projective},
        shared::ark_serialize::*,
    };
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
