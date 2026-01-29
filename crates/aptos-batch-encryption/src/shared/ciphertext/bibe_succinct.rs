// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use super::{
    super::{
        digest::EvalProofs,
        symmetric::{self, OneTimePad, OneTimePaddedKey, SymmetricCiphertext, SymmetricKey},
    },
    PreparedBIBECiphertext,
};
use crate::{
    errors::BatchEncryptionError,
    group::{Fr, G1Affine, G2Affine, PairingOutput, PairingSetting},
    shared::{
        ciphertext::bibe::{BIBECTEncrypt, InnerCiphertext},
        digest::{Digest, EvalProof},
        encryption_key::AugmentedEncryptionKey,
        ids::Id,
    },
    traits::Plaintext,
};
use anyhow::Result;
use aptos_crypto::arkworks::serialization::{ark_de, ark_se};
use ark_ec::{pairing::Pairing, AffineRepr};
use ark_serialize::CanonicalSerialize;
use ark_std::{
    rand::{CryptoRng, RngCore},
    UniformRand,
};
use serde::{Deserialize, Serialize};
use std::hash::Hash;

#[derive(Clone, Serialize, Deserialize, Debug, Hash, Eq, PartialEq)]
pub struct BIBESuccinctCiphertext {
    pub id: Id,
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    ct_g2: [G2Affine; 2],
    padded_key: OneTimePaddedKey,
    symmetric_ciphertext: SymmetricCiphertext,
}

impl InnerCiphertext for BIBESuccinctCiphertext {
    type EncryptionKey = AugmentedEncryptionKey;

    fn prepare(&self, digest: &Digest, eval_proofs: &EvalProofs) -> Result<PreparedBIBECiphertext> {
        let pf = eval_proofs
            .get(&self.id)
            .ok_or(BatchEncryptionError::UncomputedEvalProofError)?;

        self.prepare_individual(digest, &pf)
    }

    fn prepare_individual(
        &self,
        _digest: &Digest,
        eval_proof: &EvalProof,
    ) -> Result<PreparedBIBECiphertext> {
        let pairing_output = PairingSetting::pairing(**eval_proof, self.ct_g2[1]);

        Ok(PreparedBIBECiphertext {
            pairing_output,
            ct_g2: self.ct_g2[0].into(),
            padded_key: self.padded_key.clone(),
            symmetric_ciphertext: self.symmetric_ciphertext.clone(),
        })
    }

    fn id(&self) -> Id {
        self.id
    }
}

impl BIBECTEncrypt for AugmentedEncryptionKey {
    type CT = BIBESuccinctCiphertext;

    fn for_testing() -> Self {
        Self {
            sig_mpk_g2: G2Affine::generator(),
            tau_g2: G2Affine::generator(),
            tau_mpk_g2: G2Affine::generator(),
        }
    }

    fn bibe_encrypt<R: RngCore + CryptoRng>(
        &self,
        rng: &mut R,
        plaintext: &impl Plaintext,
        id: Id,
    ) -> Result<BIBESuccinctCiphertext> {
        let r = Fr::rand(rng);
        let hashed_encryption_key: G1Affine = symmetric::hash_g2_element(self.sig_mpk_g2)?;

        let ct_g2 = [
            (G2Affine::generator() * r).into(),
            ((self.sig_mpk_g2 * id.x() - self.tau_mpk_g2) * r).into(),
        ];

        let otp_source_gt: PairingOutput =
            PairingSetting::pairing(hashed_encryption_key, self.sig_mpk_g2) * r;

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

#[cfg(test)]
pub mod tests {
    use crate::{
        group::*,
        shared::{
            ciphertext::bibe::{BIBECTDecrypt as _, BIBECTEncrypt as _, InnerCiphertext as _},
            digest::DigestKey,
            encryption_key::AugmentedEncryptionKey,
            ids::{Id, IdSet},
            key_derivation::{self, BIBEDecryptionKey},
        },
    };
    use aptos_crypto::arkworks::shamir::ShamirThresholdConfig;
    use aptos_dkg::pvss::traits::Reconstructable as _;
    use ark_ff::UniformRand as _;
    use ark_std::{rand::thread_rng, One, Zero};

    #[test]
    fn test_bibe_ct_encrypt_decrypt() {
        let mut rng = thread_rng();
        let tc = ShamirThresholdConfig::new(1, 1);

        let dk = DigestKey::new(&mut rng, 8, 1).unwrap();
        let msk = Fr::rand(&mut rng);
        let (mpk, _, msk_shares) = key_derivation::gen_msk_shares(msk, &mut rng, &tc);

        let ek = AugmentedEncryptionKey::new(mpk, dk.tau_g2, (dk.tau_g2 * msk).into());

        let mut ids = IdSet::with_capacity(dk.capacity()).unwrap();
        let mut counter = Fr::zero();

        for _ in 0..dk.capacity() {
            ids.add(&Id::new(counter));
            counter += Fr::one();
        }

        ids.compute_poly_coeffs();
        let (digest, pfs) = dk.digest(&mut ids, 0).unwrap();
        let pfs = pfs.compute_all(&dk);

        let plaintext = String::from("hi");

        let id = Id::new(Fr::zero());

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
