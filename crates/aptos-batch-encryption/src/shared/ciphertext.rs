use std::marker::PhantomData;

use ark_ff::field_hashers::{DefaultFieldHasher, HashToField};
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{digest::FixedOutput, Digest, Sha256};
use crate::{errors::{BatchEncryptionError, CTVerifyError}, group::{Fr, G1Affine, G1Config, G1Projective, G2Affine, PairingSetting}, shared::ids::{FreeRootId, Id}, traits::Plaintext};
use crate::shared::ark_serialize::*;
use ark_std::UniformRand;
use ark_ec::{hashing::{curve_maps::wb::WBMap, map_to_curve_hasher::MapToCurveBasedHasher, HashToCurve}, pairing::{Pairing, PairingOutput}, AffineRepr};
use rayon::prelude::*;
use hmac::{Hmac, Mac};
use anyhow::Result;
use ark_serialize::CanonicalSerialize;
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use ed25519_dalek::Signature;
use ed25519_dalek::Verifier;

use super::{digest::EvalProofs, key_derivation::BIBEDecryptionKey, symmetric::{self, OneTimePad, OneTimePaddedKey, SymmetricCiphertext, SymmetricKey}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BIBECiphertext<I: Id> {
    pub id: I,
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    ct_1: [G2Affine; 3],
    padded_key: OneTimePaddedKey,
    symmetric_ciphertext: SymmetricCiphertext,
}

impl<I: Id> BIBECiphertext<I> {
    pub fn as_bytes(&self) -> Vec<u8> {
        // TODO is this the best way to do this?
        let ct1_parts = self.ct_1.map(|ct| format!("{:?}", ct));
        let padded_key_bytes = format!("{:?}", self.padded_key);
        let symmetric_ct_bytes = format!("{:?}", self.symmetric_ciphertext);
        let mut bytes = Vec::from(ct1_parts.clone().concat().as_bytes());
        bytes.extend_from_slice(padded_key_bytes.as_bytes());
        bytes.extend_from_slice(symmetric_ct_bytes.as_bytes());
        bytes
    }
}

pub trait BIBEEncryptionKey {
    fn sig_mpk_g2(&self) -> G2Affine;
    fn tau_g2(&self) -> G2Affine;
    fn id_set_capacity(&self) -> usize;
}


pub trait BIBECTEncrypt<I: Id> {
    fn bibe_encrypt<R: RngCore + CryptoRng>(&self, rng: &mut R, msg: &impl Plaintext, id: I) -> Result<BIBECiphertext<I>>;

}

pub trait BIBECTDecrypt<I: Id, P: Plaintext> {
    fn bibe_decrypt(
        &self, 
        ct: &BIBECiphertext<I>, 
        eval_proof: G1Affine
        ) -> Result<P>;
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Ciphertext<I: Id> {
    vk: VerifyingKey,
    bibe_ct: BIBECiphertext<I>,
    signature: Signature,
}


pub trait CTEncrypt<I: Id> {
    fn encrypt<R: RngCore + CryptoRng>(&self, rng: &mut R, msg: &impl Plaintext) -> Result<Ciphertext<I>>;
}

pub trait CTDecrypt<I: Id, P: Plaintext> {
    /// convenience method; will look up ct's id in EvalProofs, and 
    /// will use it to decrypt the ct's underlying bibe_ct
    fn decrypt(
        &self,
        ct: &Ciphertext<I>,
        eval_proofs: &EvalProofs<I::Set>,
        ) -> Result<P>;
}


impl<I: Id, EK: BIBEEncryptionKey> CTEncrypt<I> for EK {
    fn encrypt<R: RngCore + CryptoRng>(&self, rng: &mut R, msg: &impl Plaintext) -> Result<Ciphertext<I>> {
        let signing_key: SigningKey = SigningKey::generate(rng);
        let vk = signing_key.verifying_key();
        let hashed_id = I::from_verifying_key(&vk);
        let bibe_ct = self.bibe_encrypt(rng, msg, hashed_id)?;
        let signature = signing_key.sign(&bibe_ct.as_bytes());

        Ok(Ciphertext {
            vk,
            bibe_ct,
            signature,
        })
    }
}

impl<I: Id> Ciphertext<I> {
    pub fn verify(&self) -> Result<()> {
        let hashed_id = I::from_verifying_key(&self.vk);
        (self.bibe_ct.id == hashed_id).then_some(()).ok_or(
            BatchEncryptionError::CTVerifyError(CTVerifyError::IdDoesNotMatchHashedVK)
        )?;

        self.vk.verify(&self.bibe_ct.as_bytes(), &self.signature).map_err(
            |e| BatchEncryptionError::CTVerifyError(CTVerifyError::SigVerificationFailed(e))
        )?;

        Ok(())
    }

    pub fn id(&self) -> I {
        self.bibe_ct.id
    }
}


impl<I: Id, T: BIBEEncryptionKey> BIBECTEncrypt<I> for T {
    fn bibe_encrypt<R: RngCore + CryptoRng>(&self, rng: &mut R, plaintext: &impl Plaintext, id: I) -> Result<BIBECiphertext<I>>
    {
        let r = vec![Fr::rand(rng), Fr::rand(rng)];
        let hashed_encryption_key : G1Affine = symmetric::hash_g2_element(self.sig_mpk_g2())?;

        let ct_1 = [
            (G2Affine::generator() * r[0] + self.sig_mpk_g2() * r[1]).into(),
            ((G2Affine::generator() * id.x() - self.tau_g2()) * r[0]).into(),
            (- ( G2Affine::generator() * r[1] )).into()
        ];

        let otp_source_gt : PairingOutput<PairingSetting> =  PairingSetting::pairing(
            G1Affine::generator() * id.y(), 
            G2Affine::generator()) * r[0] 
            - PairingSetting::pairing(hashed_encryption_key, self.sig_mpk_g2()) * r[1];

        let mut otp_source_bytes = Vec::new();
        otp_source_gt.serialize_compressed(&mut otp_source_bytes)?;
        let otp = OneTimePad::from_source_bytes(otp_source_bytes);
        
        let symmetric_key = SymmetricKey::new(rng);
        let padded_key = otp.pad_key(&symmetric_key);

        let symmetric_ciphertext = symmetric_key.encrypt(rng, plaintext)?;

        Ok(BIBECiphertext {
            id,
            ct_1,
            padded_key,
            symmetric_ciphertext,
        })
    }
}


impl<I: Id, P: Plaintext> BIBECTDecrypt<I, P> for BIBEDecryptionKey {
    fn bibe_decrypt(
        &self, 
        ct: &BIBECiphertext<I>, 
        eval_proof: G1Affine
        ) -> Result<P> {
        let otp_source_ml = PairingSetting::multi_miller_loop(
            &[self.digest_g1, eval_proof, self.signature_g1],
            ct.ct_1);
        let otp_source_gt = PairingSetting::final_exponentiation(otp_source_ml).unwrap();

        let mut otp_source_bytes = Vec::new();
        otp_source_gt.serialize_compressed(&mut otp_source_bytes)?;
        let otp = OneTimePad::from_source_bytes(otp_source_bytes);

        let symmetric_key = otp.unpad_key(&ct.padded_key);

        symmetric_key.decrypt(&ct.symmetric_ciphertext)
    }

}

impl<I: Id, P: Plaintext> CTDecrypt<I, P> for BIBEDecryptionKey {
    fn decrypt(
        &self,
        ct: &Ciphertext<I>,
        eval_proofs: &EvalProofs<<I as Id>::Set>,
        ) -> Result<P> {

        let pf = eval_proofs
            .get(&ct.id())
            .ok_or(BatchEncryptionError::UncomputedEvalProofError)?;

        self.bibe_decrypt(&ct.bibe_ct, pf)
    }
}

#[cfg(test)]
pub mod tests {
    use crate::{schemes::fptx::{self, EncryptionKey, FPTX}, shared::{algebra::shamir::ThresholdConfig, ciphertext::{CTDecrypt, CTEncrypt, Ciphertext}, digest::DigestKey, ids::{FreeRootId, FreeRootIdSet, IdSet as _}, key_derivation::BIBEDecryptionKey}, traits::{BatchThresholdEncryption as _, Plaintext}};
    use ark_std::{rand::thread_rng, One, Zero};
    use crate::group::*;

    use super::{BIBECTDecrypt, BIBECTEncrypt};

    #[test]
    fn test_bibe_ct_encrypt_decrypt() {
        let mut rng = thread_rng();
        let tc = ThresholdConfig::new(1, 1);
        let (ek, dk, _, msk_shares) = FPTX::setup(&mut rng, 8, 1, &tc).unwrap();

        let mut ids = FreeRootIdSet::with_capacity(dk.capacity()).unwrap();
        let mut counter = Fr::zero();

        for _ in 0..dk.capacity() {
            ids.add(&FreeRootId::new(counter));
            counter += Fr::one();
        }

        ids.compute_poly_coeffs();
        let (digest, mut pfs) = dk.digest(&mut ids, 0).unwrap();
        pfs.compute_all();

        let plaintext = String::from("hi");

        let id = FreeRootId::new(Fr::zero());

        let ct = ek.bibe_encrypt(&mut rng, &plaintext, id).unwrap();

        let dk = BIBEDecryptionKey::reconstruct(&vec![msk_shares[0].derive_decryption_key_share(&digest).unwrap()], &tc).unwrap();

        let decrypted_plaintext : String = dk.bibe_decrypt(&ct, pfs.get(&FreeRootId::new(Fr::zero())).unwrap()).unwrap();

        assert_eq!(decrypted_plaintext, plaintext);
    }

    #[test]
    fn test_ct_encrypt_decrypt() {
        let mut rng = thread_rng();
        let tc = ThresholdConfig::new(1, 1);
        let (ek, dk, _, msk_shares) = FPTX::setup(&mut rng, 8, 1, &tc).unwrap();

        let plaintext = String::from("hi");
        let ct : Ciphertext<FreeRootId> = ek.encrypt(&mut rng, &plaintext).unwrap();

        let mut ids = FreeRootIdSet::with_capacity(dk.capacity()).unwrap();
        ids.add(&ct.id());

        ids.compute_poly_coeffs();
        let (digest, mut pfs) = dk.digest(&mut ids, 0).unwrap();
        pfs.compute_all();

        let dk = BIBEDecryptionKey::reconstruct(&vec![msk_shares[0].derive_decryption_key_share(&digest).unwrap()], &tc).unwrap();

        let decrypted_plaintext : String = dk.decrypt(&ct, &pfs).unwrap();

        assert_eq!(decrypted_plaintext, plaintext);
    }
}
