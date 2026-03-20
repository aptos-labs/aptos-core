// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::vuf::VUF;
use anyhow::ensure;
use aptos_crypto::{
    blstrs::{g1_proj_from_bytes, multi_pairing, random_scalar},
    hash::CryptoHash,
};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::keyless::Pepper;
use blstrs::{self, Compress, G1Affine, G1Projective, G2Affine, G2Projective, Gt, Scalar};
use group::{prime::PrimeCurveAffine, Group};
use once_cell::sync::Lazy;
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::ops::Mul;

pub struct Bls12381G1Bls {}

pub static DST: &[u8] = b"APTOS_PEPPER_BLS12381_VUF_DST";

pub static PINKAS_DST: &[u8] = b"APTOS_PINKAS_PEPPER_DST";

pub static PINKAS_SECRET_KEY_BASE_SEED: &[u8] = b"APTOS_PINKAS_PEPPER_SECRET_KEY_BASE_SEED";

pub static PINKAS_SECRET_KEY_BASE_G2: Lazy<G2Projective> =
    Lazy::new(|| G2Projective::hash_to_curve(PINKAS_SECRET_KEY_BASE_SEED, PINKAS_DST, b""));

#[derive(Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct PinkasPepper {
    #[serde(with = "BigArray")]
    pub bytes: [u8; 288],
}

impl PinkasPepper {
    pub fn from_affine_bytes(input: &[u8]) -> anyhow::Result<PinkasPepper> {
        let g1 = G1Projective::from_compressed(&input[0..48].try_into()?).unwrap();
        let g2 = *PINKAS_SECRET_KEY_BASE_G2;
        let pairing = multi_pairing([g1].iter(), [g2].iter());
        let mut output: Vec<u8> = vec![];
        pairing.write_compressed(&mut output)?;
        Ok(PinkasPepper {
            bytes: output[0..288].try_into()?,
        })
    }

    pub fn to_master_pepper(&self) -> Pepper {
        let bytes = CryptoHash::hash(self).to_vec();
        Pepper::new(bytes[0..31].try_into().unwrap())
    }
}

impl Bls12381G1Bls {
    fn hash_to_g1(input: &[u8]) -> G1Affine {
        G1Projective::hash_to_curve(input, DST, b"").into()
    }
}

pub const SCHEME_NAME: &str = "BLS12381_G1_BLS";

impl VUF for Bls12381G1Bls {
    type PrivateKey = Scalar;
    type PublicKey = G2Projective;

    fn scheme_name() -> String {
        SCHEME_NAME.to_string()
    }

    fn setup<R: CryptoRng + RngCore>(rng: &mut R) -> (Self::PrivateKey, Self::PublicKey) {
        let sk = random_scalar(rng);
        let pk = G2Affine::generator() * sk;
        (sk, pk)
    }

    fn pk_from_sk(sk: &Scalar) -> anyhow::Result<G2Projective> {
        Ok(G2Projective::generator() * sk)
    }

    /// WARNING: This function must remain constant-time w.r.t. to `sk` and `input`.
    fn eval(sk: &Scalar, input: &[u8]) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
        let input_g1 = Self::hash_to_g1(input);
        let output_g1 = input_g1.mul(sk);
        let output_bytes = output_g1.to_compressed().to_vec();
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
        let input_g1 = Self::hash_to_g1(input);
        let output_g1 = g1_proj_from_bytes(output)?;

        ensure!(
            multi_pairing(
                [-output_g1, input_g1.into()].iter(),
                [G2Projective::generator(), *pk_g2].iter()
            ) == Gt::identity(),
            "Bls12381G1Bls::verify failed with final check failure"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::vuf::{bls12381_g1_bls::Bls12381G1Bls, VUF};

    #[test]
    fn test_eval_verify() {
        let mut rng = rand::thread_rng();
        let (sk, pk) = Bls12381G1Bls::setup(&mut rng);
        let pk_another = Bls12381G1Bls::pk_from_sk(&sk).unwrap();
        assert_eq!(pk_another, pk);
        let input: &[u8] = b"hello world again and again and again and again and again and again";
        let (output, proof) = Bls12381G1Bls::eval(&sk, input).unwrap();
        Bls12381G1Bls::verify(&pk, input, &output, &proof).unwrap();
        println!("output={:?}", output);
    }
}
