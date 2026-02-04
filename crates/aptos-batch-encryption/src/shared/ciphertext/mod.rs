// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use super::{
    digest::{Digest, EvalProofs},
    key_derivation::BIBEDecryptionKey,
};
use crate::{
    errors::{BatchEncryptionError, CTVerifyError},
    shared::{ciphertext::bibe_succinct::BIBESuccinctCiphertext, digest::EvalProof, ids::Id},
    traits::{AssociatedData, Plaintext},
};
use anyhow::Result;
use ark_std::rand::{CryptoRng, RngCore};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey, SECRET_KEY_LENGTH};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::hash::Hash;

pub(crate) mod bibe;
pub(crate) mod bibe_succinct;

use bibe::*;

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(bound(deserialize = "PCT: DeserializeOwned"))]
pub struct Ciphertext<PCT: InnerCiphertext> {
    vk: VerifyingKey,
    bibe_ct: PCT,
    #[serde(with = "serde_bytes")]
    associated_data_bytes: Vec<u8>,
    signature: Signature,
}

pub type StandardCiphertext = Ciphertext<BIBECiphertext>;
pub type SuccinctCiphertext = Ciphertext<BIBESuccinctCiphertext>;

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct PreparedCiphertext {
    vk: VerifyingKey,
    bibe_ct: PreparedBIBECiphertext,
    signature: Signature,
}

impl<PCT: InnerCiphertext> Hash for Ciphertext<PCT> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.vk.hash(state);
        self.associated_data_bytes.hash(state);
        self.bibe_ct.hash(state);
        self.signature.to_bytes().hash(state);
    }
}

pub trait CTEncrypt<PCT: InnerCiphertext> {
    fn encrypt<R: RngCore + CryptoRng>(
        &self,
        rng: &mut R,
        msg: &impl Plaintext,
        associated_data: &impl AssociatedData,
    ) -> Result<Ciphertext<PCT>>;
}

pub trait CTDecrypt<P: Plaintext> {
    /// convenience method; will look up ct's id in EvalProofs, and
    /// will use it to decrypt the ct's underlying bibe_ct
    /// TODO is this still true? Doesn't seem to look up ID anymore
    fn decrypt(&self, ct: &PreparedCiphertext) -> Result<P>;
}

impl<EK: BIBECTEncrypt> CTEncrypt<EK::CT> for EK {
    fn encrypt<R: RngCore + CryptoRng>(
        &self,
        rng: &mut R,
        plaintext: &impl Plaintext,
        associated_data: &impl AssociatedData,
    ) -> Result<Ciphertext<EK::CT>> {
        // Doing this to avoid rand dependency hell
        let mut signing_key_bytes: [u8; SECRET_KEY_LENGTH] = [0; SECRET_KEY_LENGTH];
        rng.fill_bytes(&mut signing_key_bytes);

        let signing_key: SigningKey = SigningKey::from_bytes(&signing_key_bytes);
        let vk = signing_key.verifying_key();
        let hashed_id = Id::from_verifying_key(&vk);
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

impl<PCT: InnerCiphertext> Ciphertext<PCT> {
    pub fn random() -> Self {
        use ark_std::rand::thread_rng;

        let mut rng = thread_rng();
        let enc_key = PCT::EncryptionKey::for_testing();

        enc_key
            .encrypt(&mut rng, &String::from("random"), &String::from("data"))
            .unwrap()
    }

    pub fn verify(&self, associated_data: &impl AssociatedData) -> Result<()> {
        let hashed_id = Id::from_verifying_key(&self.vk);

        (self.bibe_ct.id() == hashed_id).then_some(()).ok_or(
            BatchEncryptionError::CTVerifyError(CTVerifyError::IdDoesNotMatchHashedVK),
        )?;
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

    pub fn id(&self) -> Id {
        self.bibe_ct.id()
    }

    pub fn prepare(&self, digest: &Digest, eval_proofs: &EvalProofs) -> Result<PreparedCiphertext> {
        Ok(PreparedCiphertext {
            vk: self.vk,
            bibe_ct: self.bibe_ct.prepare(digest, eval_proofs)?,
            signature: self.signature,
        })
    }

    pub fn prepare_individual(
        &self,
        digest: &Digest,
        eval_proof: &EvalProof,
    ) -> Result<PreparedCiphertext> {
        Ok(PreparedCiphertext {
            vk: self.vk,
            bibe_ct: self.bibe_ct.prepare_individual(digest, eval_proof)?,
            signature: self.signature,
        })
    }
}

impl<P: Plaintext> CTDecrypt<P> for BIBEDecryptionKey {
    fn decrypt(&self, ct: &PreparedCiphertext) -> Result<P> {
        self.bibe_decrypt(&ct.bibe_ct)
    }
}

#[cfg(test)]
pub mod tests {

    use crate::{
        errors::{BatchEncryptionError, CTVerifyError},
        group::Fr,
        schemes::fptx::FPTX,
        shared::{
            ciphertext::{CTDecrypt, CTEncrypt, StandardCiphertext, SuccinctCiphertext},
            digest::DigestKey,
            encryption_key::AugmentedEncryptionKey,
            ids::IdSet,
            key_derivation::{self, BIBEDecryptionKey},
        },
        traits::BatchThresholdEncryption as _,
    };
    use aptos_crypto::arkworks::shamir::ShamirThresholdConfig;
    use aptos_dkg::pvss::traits::Reconstructable as _;
    use ark_ff::UniformRand as _;
    use ark_std::rand::{thread_rng, Rng};

    #[test]
    fn test_ct_encrypt_decrypt() {
        let mut rng = thread_rng();
        let tc = ShamirThresholdConfig::new(1, 1);
        let (ek, dk, _, msk_shares) = FPTX::setup_for_testing(rng.r#gen(), 8, 1, &tc).unwrap();

        let plaintext = String::from("hi");
        let associated_data = String::from("");
        let ct: StandardCiphertext = ek.encrypt(&mut rng, &plaintext, &associated_data).unwrap();

        let mut ids = IdSet::with_capacity(dk.capacity()).unwrap();
        ids.add(&ct.id());

        ids.compute_poly_coeffs();
        let (digest, pfs) = dk.digest(&mut ids, 0).unwrap();
        let pfs = pfs.compute_all(&dk);

        let dk = BIBEDecryptionKey::reconstruct(&tc, &[msk_shares[0]
            .derive_decryption_key_share(&digest)
            .unwrap()])
        .unwrap();

        let decrypted_plaintext: String = dk.decrypt(&ct.prepare(&digest, &pfs).unwrap()).unwrap();

        assert_eq!(decrypted_plaintext, plaintext);
    }

    #[test]
    fn test_succinct_ct_encrypt_decrypt() {
        let mut rng = thread_rng();
        let tc = ShamirThresholdConfig::new(1, 1);

        let dk = DigestKey::new(&mut rng, 8, 1).unwrap();
        let msk = Fr::rand(&mut rng);
        let (mpk, _, msk_shares) = key_derivation::gen_msk_shares(msk, &mut rng, &tc);

        let ek = AugmentedEncryptionKey::new(mpk, dk.tau_g2, (dk.tau_g2 * msk).into());

        let plaintext = String::from("hi");
        let associated_data = String::from("");
        let ct: SuccinctCiphertext = ek.encrypt(&mut rng, &plaintext, &associated_data).unwrap();

        let mut ids = IdSet::with_capacity(dk.capacity()).unwrap();
        ids.add(&ct.id());

        ids.compute_poly_coeffs();
        let (digest, pfs) = dk.digest(&mut ids, 0).unwrap();
        let pfs = pfs.compute_all(&dk);

        let dk = BIBEDecryptionKey::reconstruct(&tc, &[msk_shares[0]
            .derive_decryption_key_share(&digest)
            .unwrap()])
        .unwrap();

        let decrypted_plaintext: String = dk.decrypt(&ct.prepare(&digest, &pfs).unwrap()).unwrap();

        assert_eq!(decrypted_plaintext, plaintext);
    }

    #[test]
    fn test_ct_verify() {
        let mut rng = thread_rng();
        let tc = ShamirThresholdConfig::new(1, 1);
        let (ek, _, _, _) = FPTX::setup_for_testing(rng.r#gen(), 8, 1, &tc).unwrap();

        let plaintext = String::from("hi");
        let associated_data = String::from("associated data");
        let mut ct: StandardCiphertext =
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
