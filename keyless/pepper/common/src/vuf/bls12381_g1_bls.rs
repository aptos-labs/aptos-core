// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::vuf::VUF;
use anyhow::{anyhow, ensure};
use aptos_crypto::hash::CryptoHash;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_dkg::utils::multi_pairing;
use aptos_types::keyless::Pepper;
use ark_bls12_381::{Bls12_381, Fq12, Fr, G1Affine, G2Affine, G2Projective};
use ark_ec::{
    hashing::HashToCurve, pairing::Pairing, short_weierstrass::Projective, AffineRepr, CurveGroup,
    Group,
};
use ark_ff::Field;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::{
    rand::{CryptoRng, RngCore},
    UniformRand,
};
use blstrs::{self, Compress};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::ops::Mul;

pub struct Bls12381G1Bls {}

pub static DST: &[u8] = b"APTOS_PEPPER_BLS12381_VUF_DST";

pub static PINKAS_DST: &[u8] = b"APTOS_PEPPER_PINKAS_VUF_DST";

pub static PINKAS_SECRET_KEY_BASE: &[u8] = b"APTOS_PEPPER_PINKAS_VUF_SECRET_KEY_BASE";

pub static PINKAS_SECRET_KEY_BASE_AFFINE: Lazy<G2Affine> =
    Lazy::new(|| Bls12381G1Bls::hash_to_g2(PINKAS_SECRET_KEY_BASE, PINKAS_DST));

pub static PINKAS_SECRET_KEY_BASE_G2: Lazy<blstrs::G2Projective> =
    Lazy::new(|| {
        let mut g2_bytes = vec![];
        PINKAS_SECRET_KEY_BASE_AFFINE
            .serialize_compressed(&mut g2_bytes)
            .map_err(|e| {
                anyhow!("Bls12381G1Bls::eval failed with output serialization error: {e}")
            }).unwrap();
        let b2: [u8; 96] = g2_bytes.clone()[0..96]
            .try_into().unwrap();
        blstrs::G2Projective::from_compressed(&b2).unwrap()
    });

#[derive(Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct PinkasPepper {
    #[serde(with = "BigArray")]
    pub bytes: [u8; 288],
}

impl PinkasPepper {

    pub fn from_affine_bytes(input: &[u8]) -> anyhow::Result<PinkasPepper> {
        let g1 = G1Affine::deserialize_compressed(input)?;
        let mut g1_bytes = vec![];
        g1.serialize_compressed(&mut g1_bytes).map_err(|e| {
            anyhow!("Bls12381G1Bls::eval failed with output serialization error: {e}")
        })?;
        let b1: [u8; 48] = g1_bytes.clone()[0..48]
            .try_into()
            .expect("Expected 48 bytes");
        let r1 = [blstrs::G1Projective::from_compressed(&b1).unwrap()];
        let r2 = [*PINKAS_SECRET_KEY_BASE_G2];
        let pairing = multi_pairing(r1.iter(), r2.iter());
        let mut output: Vec<u8> = vec![];
        pairing.write_compressed(&mut output).unwrap();
        Ok(PinkasPepper {
            bytes: output.try_into().expect("Expected 288 bytes"),
        })
    }

    // This doesn't work
    // pub fn to_pinkas_pepper2(input: &[u8]) -> anyhow::Result<PinkasPepper> {
    //     let g1 = G1Affine::deserialize_compressed(input)?;
    //     let mut g1_bytes = vec![];
    //     g1.serialize_compressed(&mut g1_bytes).map_err(|e| {
    //         anyhow!("Bls12381G1Bls::eval failed with output serialization error: {e}")
    //     })?;
    //     let mut g2_bytes = vec![];
    //     PINKAS_SECRET_KEY_BASE_AFFINE
    //         .serialize_compressed(&mut g2_bytes)
    //         .map_err(|e| {
    //             anyhow!("Bls12381G1Bls::eval failed with output serialization error: {e}")
    //         })?;

    //     // println!("g1");
    //     // println!("{}", hex::encode(g1_bytes));
    //     // println!("g2");
    //     // println!("{}", hex::encode(g2_bytes));

    //     let pinkas_pairing = Bls12_381::pairing(g1, *PINKAS_SECRET_KEY_BASE_AFFINE).0;
    //     println!("{}", hex::encode(pinkas_pairing.c0.c0.c0.0.to_bytes_le()));
    //     let mut output_bytes = vec![];
    //     pinkas_pairing
    //         .serialize_uncompressed(&mut output_bytes)
    //         .map_err(|e| {
    //             anyhow!("Bls12381G1Bls::eval failed with output serialization error: {e}")
    //         })?;

    //     println!("pinkas");
    //     println!("{}", hex::encode(output_bytes.clone()));
    //     Ok(PinkasPepper {
    //         bytes: output_bytes.try_into().expect("Expected 288 bytes"),
    //     })
    // }

    pub fn to_master_pepper(&self) -> Pepper {
        let bytes = CryptoHash::hash(self).to_vec();

        println!("hashed");
        println!("{:?}", bytes);

        let slice_bytes: &[u8] = &bytes[0..31];
        // If you're sure that `vec_bytes` has exactly 31 elements
        let mut array_bytes: [u8; 31] = [0; 31];

        // Copy the elements from the slice to the array
        array_bytes.copy_from_slice(slice_bytes);
        Pepper::new(array_bytes)
    }
}

impl Bls12381G1Bls {
    fn hash_to_g1(input: &[u8], dst: &[u8]) -> G1Affine {
        let mapper = ark_ec::hashing::map_to_curve_hasher::MapToCurveBasedHasher::<
            Projective<ark_bls12_381::g1::Config>,
            ark_ff::fields::field_hashers::DefaultFieldHasher<sha2_0_10_6::Sha256, 128>,
            ark_ec::hashing::curve_maps::wb::WBMap<ark_bls12_381::g1::Config>,
        >::new(dst)
        .unwrap();
        mapper.hash(input).unwrap()
    }

    fn hash_to_g2(input: &[u8], dst: &[u8]) -> G2Affine {
        let mapper = ark_ec::hashing::map_to_curve_hasher::MapToCurveBasedHasher::<
            Projective<ark_bls12_381::g2::Config>,
            ark_ff::fields::field_hashers::DefaultFieldHasher<sha2_0_10_6::Sha256, 128>,
            ark_ec::hashing::curve_maps::wb::WBMap<ark_bls12_381::g2::Config>,
        >::new(dst)
        .unwrap();
        mapper.hash(input).unwrap()
    }
}

pub const SCHEME_NAME: &str = "BLS12381_G1_BLS";

impl VUF for Bls12381G1Bls {
    type PrivateKey = Fr;
    type PublicKey = G2Projective;

    fn scheme_name() -> String {
        SCHEME_NAME.to_string()
    }

    fn setup<R: CryptoRng + RngCore>(rng: &mut R) -> (Self::PrivateKey, Self::PublicKey) {
        let sk = Fr::rand(rng);
        let pk = G2Affine::generator() * sk;
        (sk, pk)
    }

    fn pk_from_sk(sk: &Fr) -> anyhow::Result<G2Projective> {
        Ok(G2Projective::generator() * sk)
    }

    fn eval(sk: &Fr, input: &[u8]) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
        let input_g1 = Self::hash_to_g1(input, DST);
        let output_g1 = input_g1.mul(sk).into_affine();
        let mut output_bytes = vec![];
        output_g1
            .serialize_compressed(&mut output_bytes)
            .map_err(|e| {
                anyhow!("Bls12381G1Bls::eval failed with output serialization error: {e}")
            })?;
        Ok((output_bytes, vec![]))
    }

    fn verify(
        pk_g2: &G2Projective,
        input: &[u8],
        output: &[u8],
        proof: &[u8],
    ) -> anyhow::Result<()> {
        ensure!(
            proof.is_empty(),
            "Bls12381G1Bls::verify failed with proof deserialization error"
        );
        let input_g1 = Self::hash_to_g1(input, DST);
        let output_g1 = G1Affine::deserialize_compressed(output).map_err(|e| {
            anyhow!("Bls12381G1Bls::verify failed with output deserialization error: {e}")
        })?;
        ensure!(
            Fq12::ONE
                == Bls12_381::multi_pairing([-output_g1, input_g1], [
                    G2Affine::generator(),
                    (*pk_g2).into_affine()
                ])
                .0,
            "Bls12381G1Bls::verify failed with final check failure"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::vuf::{bls12381_g1_bls::Bls12381G1Bls, VUF};

    #[test]
    fn gen_eval_verify() {
        let mut rng = ark_std::rand::thread_rng();
        let (sk, pk) = Bls12381G1Bls::setup(&mut rng);
        let pk_another = Bls12381G1Bls::pk_from_sk(&sk).unwrap();
        assert_eq!(pk_another, pk);
        let input: &[u8] = b"hello world again and again and again and again and again and again";
        let (output, proof) = Bls12381G1Bls::eval(&sk, input).unwrap();
        Bls12381G1Bls::verify(&pk, input, &output, &proof).unwrap();
        println!("output={:?}", output);
    }
}
