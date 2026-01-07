// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use super::super::{
    digest::{Digest, EvalProofs},
    key_derivation::BIBEDecryptionKey,
    symmetric::{self, OneTimePad, OneTimePaddedKey, SymmetricCiphertext, SymmetricKey},
};
use crate::{
    errors::BatchEncryptionError,
    group::{Fr, G1Affine, G2Affine, G2Prepared, PairingOutput, PairingSetting},
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
pub struct BIBECiphertext<I: Id> {
    pub id: I,
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    ct_g2: [G2Affine; 3],
    padded_key: OneTimePaddedKey,
    symmetric_ciphertext: SymmetricCiphertext,
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct PreparedBIBECiphertext {
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub(crate) pairing_output: PairingOutput,
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub(crate) ct_g2: G2Prepared,
    pub(crate) padded_key: OneTimePaddedKey,
    pub(crate) symmetric_ciphertext: SymmetricCiphertext,
}

pub trait BIBEEncryptionKey {
    fn sig_mpk_g2(&self) -> G2Affine;
    fn tau_g2(&self) -> G2Affine;
}

pub trait BIBECTEncrypt<I: Id> {
    fn bibe_encrypt<R: RngCore + CryptoRng>(
        &self,
        rng: &mut R,
        msg: &impl Plaintext,
        id: I,
    ) -> Result<BIBECiphertext<I>>;
}

pub trait BIBECTDecrypt<P: Plaintext> {
    fn bibe_decrypt(&self, ct: &PreparedBIBECiphertext) -> Result<P>;
}

impl<I: Id> BIBECiphertext<I> {
    pub fn prepare(
        &self,
        digest: &Digest,
        eval_proofs: &EvalProofs<<I as Id>::OssifiedSet>,
    ) -> Result<PreparedBIBECiphertext> {
        let pf = eval_proofs
            .get(&self.id)
            .ok_or(BatchEncryptionError::UncomputedEvalProofError)?;

        self.prepare_individual(digest, &pf)
    }

    pub fn prepare_individual(
        &self,
        digest: &Digest,
        eval_proof: &G1Affine,
    ) -> Result<PreparedBIBECiphertext> {
        let pairing_output = PairingSetting::pairing(digest.as_g1(), self.ct_g2[0])
            + PairingSetting::pairing(eval_proof, self.ct_g2[1]);

        Ok(PreparedBIBECiphertext {
            pairing_output,
            ct_g2: self.ct_g2[2].into(),
            padded_key: self.padded_key.clone(),
            symmetric_ciphertext: self.symmetric_ciphertext.clone(),
        })
    }
}


impl<I: Id, T: BIBEEncryptionKey> BIBECTEncrypt<I> for T {
    fn bibe_encrypt<R: RngCore + CryptoRng>(
        &self,
        rng: &mut R,
        plaintext: &impl Plaintext,
        id: I,
    ) -> Result<BIBECiphertext<I>> {
        let r = [Fr::rand(rng), Fr::rand(rng)];
        let hashed_encryption_key: G1Affine = symmetric::hash_g2_element(self.sig_mpk_g2())?;

        let ct_g2 = [
            (G2Affine::generator() * r[0] + self.sig_mpk_g2() * r[1]).into(),
            ((G2Affine::generator() * id.x() - self.tau_g2()) * r[0]).into(),
            (-(G2Affine::generator() * r[1])).into(),
        ];

        let otp_source_gt: PairingOutput =
            PairingSetting::pairing(G1Affine::generator() * id.y(), G2Affine::generator()) * r[0]
                - PairingSetting::pairing(hashed_encryption_key, self.sig_mpk_g2()) * r[1];

        let mut otp_source_bytes = Vec::new();
        otp_source_gt.serialize_compressed(&mut otp_source_bytes)?;
        let otp = OneTimePad::from_source_bytes(otp_source_bytes);

        let symmetric_key = SymmetricKey::new(rng);
        let padded_key = otp.pad_key(&symmetric_key);

        let symmetric_ciphertext = symmetric_key.encrypt(rng, plaintext)?;

        Ok(BIBECiphertext {
            id,
            ct_g2,
            padded_key,
            symmetric_ciphertext,
        })
    }
}

impl<P: Plaintext> BIBECTDecrypt<P> for BIBEDecryptionKey {
    fn bibe_decrypt(&self, ct: &PreparedBIBECiphertext) -> Result<P> {
        let otp_source_1 = PairingSetting::pairing(self.signature_g1, ct.ct_g2.clone());
        let otp_source_gt = otp_source_1 + ct.pairing_output;

        let mut otp_source_bytes = Vec::new();
        otp_source_gt.serialize_compressed(&mut otp_source_bytes)?;
        let otp = OneTimePad::from_source_bytes(otp_source_bytes);

        let symmetric_key = otp.unpad_key(&ct.padded_key);

        symmetric_key.decrypt(&ct.symmetric_ciphertext)
    }
}


#[cfg(test)]
pub mod tests {
    use super::{BIBECTDecrypt, BIBECTEncrypt};
    use crate::{
        group::*,
        schemes::fptx::FPTX,
        shared::{
            ids::{FreeRootId, FreeRootIdSet, IdSet as _},
            key_derivation::BIBEDecryptionKey,
        },
        traits::BatchThresholdEncryption as _,
    };
    use aptos_crypto::arkworks::shamir::ShamirThresholdConfig;
    use aptos_dkg::pvss::traits::Reconstructable as _;
    use ark_std::{
        rand::{thread_rng, Rng},
        One, Zero,
    };

    #[test]
    fn test_bibe_ct_encrypt_decrypt() {
        let mut rng = thread_rng();
        let tc = ShamirThresholdConfig::new(1, 1);
        let (ek, dk, _, msk_shares, _, _) =
            FPTX::setup_for_testing(rng.r#gen(), 8, 1, &tc, &tc).unwrap();

        let mut ids = FreeRootIdSet::with_capacity(dk.capacity()).unwrap();
        let mut counter = Fr::zero();

        for _ in 0..dk.capacity() {
            ids.add(&FreeRootId::new(counter));
            counter += Fr::one();
        }

        ids.compute_poly_coeffs();
        let (digest, pfs) = dk.digest(&mut ids, 0).unwrap();
        let pfs = pfs.compute_all(&dk);

        let plaintext = String::from("hi");

        let id = FreeRootId::new(Fr::zero());

        let ct = ek.bibe_encrypt(&mut rng, &plaintext, id).unwrap();

        let dk = BIBEDecryptionKey::reconstruct(&tc, &[msk_shares[0]
            .derive_decryption_key_share(&digest)
            .unwrap()])
        .unwrap();

        let decrypted_plaintext: String = dk
            .bibe_decrypt(&ct.prepare(&digest, &pfs).unwrap())
            .unwrap();

        assert_eq!(decrypted_plaintext, plaintext);
    }
}
