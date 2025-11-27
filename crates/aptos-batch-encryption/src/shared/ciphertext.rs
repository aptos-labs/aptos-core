use super::{
    digest::{Digest, EvalProofs}, ids::FreeRootId, key_derivation::BIBEDecryptionKey, symmetric::{self, OneTimePad, OneTimePaddedKey, SymmetricCiphertext, SymmetricKey}
};
use crate::{
    errors::{BatchEncryptionError, CTVerifyError},
    group::{Fr, G1Affine, G2Affine, G2Prepared, PairingOutput, PairingSetting},
    shared::{ark_serialize::*, ids::Id},
    traits::{AssociatedData, Plaintext},
};
use anyhow::Result;
use ark_ec::{pairing::Pairing, AffineRepr};
use ark_serialize::CanonicalSerialize;
use ark_std::{UniformRand, Zero};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand_core::{CryptoRng, RngCore};
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
    pairing_output: PairingOutput,
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    ct_g2: G2Prepared,
    padded_key: OneTimePaddedKey,
    symmetric_ciphertext: SymmetricCiphertext,
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

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Ciphertext<I: Id> {
    vk: VerifyingKey,
    bibe_ct: BIBECiphertext<I>,
    associated_data_bytes: Vec<u8>,
    signature: Signature,
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct PreparedCiphertext {
    vk: VerifyingKey,
    bibe_ct: PreparedBIBECiphertext,
    signature: Signature,
}

impl<I: Id> Hash for Ciphertext<I> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.vk.hash(state);
        self.associated_data_bytes.hash(state);
        self.bibe_ct.hash(state);
        self.signature.to_bytes().hash(state);
    }
}

pub trait CTEncrypt<I: Id> {
    fn encrypt<R: RngCore + CryptoRng>(
        &self,
        rng: &mut R,
        msg: &impl Plaintext,
        associated_data: &impl AssociatedData,
    ) -> Result<Ciphertext<I>>;
}

pub trait CTDecrypt<P: Plaintext> {
    /// convenience method; will look up ct's id in EvalProofs, and
    /// will use it to decrypt the ct's underlying bibe_ct
    /// TODO is this still true? Doesn't seem to look up ID anymore
    fn decrypt(&self, ct: &PreparedCiphertext) -> Result<P>;
}

impl<I: Id, EK: BIBEEncryptionKey> CTEncrypt<I> for EK {
    fn encrypt<R: RngCore + CryptoRng>(
        &self,
        rng: &mut R,
        plaintext: &impl Plaintext,
        associated_data: &impl AssociatedData,
    ) -> Result<Ciphertext<I>> {
        let signing_key: SigningKey = SigningKey::generate(rng);
        let vk = signing_key.verifying_key();
        let hashed_id = I::from_verifying_key(&vk);
        let bibe_ct = self.bibe_encrypt(rng, plaintext, hashed_id)?;

        // So that Ciphertext doesn't have to be generic over some AD: AssociatedData
        let associated_data_bytes = bcs::to_bytes(&associated_data)?;

        let to_sign = (&bibe_ct, &associated_data_bytes);
        let signature = signing_key.sign(&bcs::to_bytes(&to_sign)?);

        Ok(Ciphertext {
            vk,
            bibe_ct,
            associated_data_bytes,
            signature,
        })
    }
}

#[cfg(test)]
impl BIBECiphertext<FreeRootId> {
    pub(crate) fn blank_for_testing() -> Self {
        BIBECiphertext {
            id: FreeRootId::new(Fr::zero()),
            ct_g2: [
                G2Affine::generator(),
                (G2Affine::generator() * Fr::from(2)).into(),
                (G2Affine::generator() * Fr::from(3)).into(),
            ],
            padded_key: OneTimePaddedKey::blank_for_testing(),
            symmetric_ciphertext: SymmetricCiphertext::blank_for_testing(),
        }
    }
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

impl<I: Id> Ciphertext<I> {
    pub fn verify(&self, associated_data: &impl AssociatedData) -> Result<()> {
        let hashed_id = I::from_verifying_key(&self.vk);

        (self.bibe_ct.id == hashed_id)
            .then_some(())
            .ok_or(BatchEncryptionError::CTVerifyError(
                CTVerifyError::IdDoesNotMatchHashedVK,
            ))?;
        (self.associated_data_bytes == bcs::to_bytes(associated_data)?)
            .then_some(())
            .ok_or(BatchEncryptionError::CTVerifyError(
                CTVerifyError::AssociatedDataDoesNotMatch,
            ))?;

        let to_verify = (&self.bibe_ct, &self.associated_data_bytes);

        self.vk
            .verify(&bcs::to_bytes(&to_verify)?, &self.signature)
            .map_err(|e| {
                BatchEncryptionError::CTVerifyError(CTVerifyError::SigVerificationFailed(e))
            })?;

        Ok(())
    }

    pub fn id(&self) -> I {
        self.bibe_ct.id
    }

    pub fn prepare(
        &self,
        digest: &Digest,
        eval_proofs: &EvalProofs<<I as Id>::OssifiedSet>,
    ) -> Result<PreparedCiphertext> {
        Ok(PreparedCiphertext {
            vk: self.vk,
            bibe_ct: self.bibe_ct.prepare(digest, eval_proofs)?,
            signature: self.signature,
        })
    }

    pub fn prepare_individual(
        &self,
        digest: &Digest,
        eval_proof: &G1Affine,
    ) -> Result<PreparedCiphertext> {
        Ok(PreparedCiphertext {
            vk: self.vk,
            bibe_ct: self.bibe_ct.prepare_individual(digest, eval_proof)?,
            signature: self.signature,
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
         let r = vec![Fr::rand(rng), Fr::rand(rng)];
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

impl<P: Plaintext> CTDecrypt<P> for BIBEDecryptionKey {
    fn decrypt(&self, ct: &PreparedCiphertext) -> Result<P> {
        self.bibe_decrypt(&ct.bibe_ct)
    }
}

#[cfg(test)]
pub mod tests {

    use super::{BIBECTDecrypt, BIBECTEncrypt};
    use crate::{
        errors::{BatchEncryptionError, CTVerifyError},
        group::*,
        schemes::fptx::FPTX,
        shared::{
            algebra::shamir::ThresholdConfig,
            ciphertext::{CTDecrypt, CTEncrypt, Ciphertext},
            ids::{FreeRootId, FreeRootIdSet, IdSet as _},
            key_derivation::BIBEDecryptionKey,
        },
        traits::BatchThresholdEncryption as _,
    };
    use ark_std::{
        rand::{thread_rng, Rng},
        One, Zero,
    };

    #[test]
    fn test_bibe_ct_encrypt_decrypt() {
        let mut rng = thread_rng();
        let tc = ThresholdConfig::new(1, 1);
        let (ek, dk, _, msk_shares, _, _) =
            FPTX::setup_for_testing(rng.gen(), 8, 1, &tc, &tc).unwrap();

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

        let dk = BIBEDecryptionKey::reconstruct(
            &vec![msk_shares[0].derive_decryption_key_share(&digest).unwrap()],
            &tc,
        )
        .unwrap();

        let decrypted_plaintext: String = dk
            .bibe_decrypt(&ct.prepare(&digest, &pfs).unwrap())
            .unwrap();

        assert_eq!(decrypted_plaintext, plaintext);
    }

    #[test]
    fn test_ct_encrypt_decrypt() {
        let mut rng = thread_rng();
        let tc = ThresholdConfig::new(1, 1);
        let (ek, dk, _, msk_shares, _, _) =
            FPTX::setup_for_testing(rng.gen(), 8, 1, &tc, &tc).unwrap();

        let plaintext = String::from("hi");
        let associated_data = String::from("");
        let ct: Ciphertext<FreeRootId> =
            ek.encrypt(&mut rng, &plaintext, &associated_data).unwrap();

        let mut ids = FreeRootIdSet::with_capacity(dk.capacity()).unwrap();
        ids.add(&ct.id());

        ids.compute_poly_coeffs();
        let (digest, pfs) = dk.digest(&mut ids, 0).unwrap();
        let pfs = pfs.compute_all(&dk);

        let dk = BIBEDecryptionKey::reconstruct(
            &vec![msk_shares[0].derive_decryption_key_share(&digest).unwrap()],
            &tc,
        )
        .unwrap();

        let decrypted_plaintext: String = dk.decrypt(&ct.prepare(&digest, &pfs).unwrap()).unwrap();

        assert_eq!(decrypted_plaintext, plaintext);
    }

    #[test]
    fn test_ct_verify() {
        let mut rng = thread_rng();
        let tc = ThresholdConfig::new(1, 1);
        let (ek, _, _,  _, _, _) =
            FPTX::setup_for_testing(rng.gen(), 8, 1, &tc, &tc).unwrap();

        let plaintext = String::from("hi");
        let associated_data = String::from("associated data");
        let mut ct: Ciphertext<FreeRootId> =
            ek.encrypt(&mut rng, &plaintext, &associated_data).unwrap();

        // Verification with the correct associated data should succeed.
        ct.verify(&associated_data).unwrap();

        // The CT itself contains a byte encoding of the associated data. Verification with
        // incorrect associated data returns an error indicating as such.
        let e: BatchEncryptionError = ct
            .verify(&String::from("fake associated data"))
            .unwrap_err()
            .downcast()
            .unwrap();
        assert!(matches!(
            e,
            BatchEncryptionError::CTVerifyError(CTVerifyError::AssociatedDataDoesNotMatch)
        ));

        // Even if the CT itself is modified to contain a byte encoding of incorrect associated
        // data, verification should fail, this time with an error message indicating that the
        // signature verification failed.
        ct.associated_data_bytes = bcs::to_bytes(&String::from("fake associated data")).unwrap();
        let e: BatchEncryptionError = ct
            .verify(&String::from("fake associated data"))
            .unwrap_err()
            .downcast()
            .unwrap();
        assert!(matches!(
            e,
            BatchEncryptionError::CTVerifyError(CTVerifyError::SigVerificationFailed(_))
        ));
    }
}
