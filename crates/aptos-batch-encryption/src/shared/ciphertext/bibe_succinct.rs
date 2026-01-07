// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use super::super::{
    digest::{EvalProofs},
    symmetric::{self, OneTimePad, OneTimePaddedKey, SymmetricCiphertext, SymmetricKey},
};
use super::PreparedBIBECiphertext;
use crate::{
    errors::BatchEncryptionError,
    group::{Fr, G1Affine, G2Affine, PairingOutput, PairingSetting},
    shared::{ark_serialize::*, ids::Id},
    traits::Plaintext,
};
use anyhow::Result;
use ark_ec::{pairing::Pairing, AffineRepr};
use ark_serialize::CanonicalSerialize;
use ark_std::{
    rand::{CryptoRng, RngCore},
    UniformRand,
};
use serde::{Deserialize, Serialize};
use std::hash::Hash;

#[derive(Clone, Serialize, Deserialize, Debug, Hash, Eq, PartialEq)]
pub struct BIBESuccinctCiphertext<I: Id> {
    pub id: I,
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    ct_g2: [G2Affine; 2],
    padded_key: OneTimePaddedKey,
    symmetric_ciphertext: SymmetricCiphertext,
}

pub trait BIBEAugmentedEncryptionKey {
    fn sig_mpk_g2(&self) -> G2Affine;
    fn tau_g2(&self) -> G2Affine;
    fn tau_mpk_g2(&self) -> G2Affine;
}

pub trait BIBESuccintCTEncrypt<I: Id> {
    fn bibe_succinct_encrypt<R: RngCore + CryptoRng>(
        &self,
        rng: &mut R,
        msg: &impl Plaintext,
        id: I,
    ) -> Result<BIBESuccinctCiphertext<I>>;
}

impl<I: Id> BIBESuccinctCiphertext<I> {
    pub fn prepare(
        &self,
        eval_proofs: &EvalProofs<<I as Id>::OssifiedSet>,
    ) -> Result<PreparedBIBECiphertext> {
        let pf = eval_proofs
            .get(&self.id)
            .ok_or(BatchEncryptionError::UncomputedEvalProofError)?;

        self.prepare_individual(&pf)
    }

    pub fn prepare_individual(
        &self,
        eval_proof: &G1Affine,
    ) -> Result<PreparedBIBECiphertext> {
        let pairing_output = PairingSetting::pairing(eval_proof, self.ct_g2[1]);

        Ok(PreparedBIBECiphertext {
            pairing_output,
            ct_g2: self.ct_g2[0].into(),
            padded_key: self.padded_key.clone(),
            symmetric_ciphertext: self.symmetric_ciphertext.clone(),
        })
    }
}


impl<I: Id, T: BIBEAugmentedEncryptionKey> BIBESuccintCTEncrypt<I> for T {
    fn bibe_succinct_encrypt<R: RngCore + CryptoRng>(
        &self,
        rng: &mut R,
        plaintext: &impl Plaintext,
        id: I,
    ) -> Result<BIBESuccinctCiphertext<I>> {
        let r = Fr::rand(rng);
        let hashed_encryption_key: G1Affine = symmetric::hash_g2_element(self.sig_mpk_g2())?;

        let ct_g2 = [
            ((self.sig_mpk_g2() * id.x() - self.tau_mpk_g2()) * r).into(),
            (G2Affine::generator() * r).into(),
        ];

        // Although id has a y coordinate, in the current code this is always 0. Should
        // remove this at some point. Succinct CTs currently only work w/ zero-y ids.
        let otp_source_gt: PairingOutput =
                - PairingSetting::pairing(hashed_encryption_key, self.sig_mpk_g2()) * r;

        let mut otp_source_bytes = Vec::new();
        otp_source_gt.serialize_compressed(&mut otp_source_bytes)?;
        let otp = OneTimePad::from_source_bytes(otp_source_bytes);

        let symmetric_key = SymmetricKey::new(rng);
        let padded_key = otp.pad_key(&symmetric_key);

        let symmetric_ciphertext = symmetric_key.encrypt(rng, plaintext)?;

        Ok(BIBESuccinctCiphertext {
            id,
            ct_g2,
            padded_key,
            symmetric_ciphertext,
        })
    }
}
