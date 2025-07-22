use std::marker::PhantomData;
use ark_ec::AffineRepr as _;
use ark_ff::UniformRand as _;
use ed25519_dalek::SIGNATURE_LENGTH;
use rand_core::{CryptoRng, RngCore};
use anyhow::Result;
use anyhow::anyhow;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator as _;

use crate::errors::BatchEncryptionError;
use crate::shared::algebra::shamir::{ShamirGroupShare, ShamirShare, ThresholdConfig};
use crate::shared::ciphertext::BIBECTDecrypt;
use crate::shared::ciphertext::CTDecrypt;
use crate::shared::ciphertext::CTEncrypt;
use crate::shared::ciphertext::{BIBEEncryptionKey, Ciphertext};
use crate::shared::digest::{Digest, EvalProofs};
use crate::shared::ids::{FreeRootId, FreeRootIdSet};
use crate::shared::key_derivation::{self, BIBEDecryptionKey, BIBEDecryptionKeyShare, BIBEMasterSecretKeyShare, BIBEVerificationKey};
use crate::{group::*, shared::{digest::DigestKey, ids::{Id, IdSet}}, traits::{BatchThresholdEncryption, Plaintext}};

pub struct FPTX {
}

pub struct EncryptionKey {
    sig_mpk_g2: G2Affine,
    tau_g2: G2Affine,
    id_set_capacity: usize,
}

impl EncryptionKey {
    pub fn new(
        sig_mpk_g2: G2Affine,
        tau_g2: G2Affine,
        id_set_capacity: usize,
    ) -> Self
    {
        Self {
            sig_mpk_g2,
            tau_g2,
            id_set_capacity,
        }
    }
}


impl BIBEEncryptionKey for EncryptionKey {

    fn sig_mpk_g2(&self) -> G2Affine {
        self.sig_mpk_g2
    }

    fn tau_g2(&self) -> G2Affine {
        self.tau_g2
    }

    fn id_set_capacity(&self) -> usize {
        self.id_set_capacity
    }
}



impl BatchThresholdEncryption for FPTX {
    type EncryptionKey = EncryptionKey;

    type DigestKey = DigestKey;

    type Ciphertext = Ciphertext<FreeRootId>;

    type Round = usize;

    type Digest = Digest;

    type EvalProofs<'a> = EvalProofs<'a, FreeRootIdSet>;

    type MasterSecretKeyShare = BIBEMasterSecretKeyShare;

    type DecryptionKeyShare = BIBEDecryptionKeyShare;

    type DecryptionKey = BIBEDecryptionKey;

    type Id = FreeRootId;

    type VerificationKey = BIBEVerificationKey;

    fn setup<R: RngCore + CryptoRng>(rng: &mut R, max_batch_size: usize, number_of_rounds: usize, tc: &ThresholdConfig)
        -> Result<(Self::EncryptionKey, Self::DigestKey, Vec<Self::VerificationKey>, Vec<Self::MasterSecretKeyShare>)> {

        let digest_key = DigestKey::new(rng, max_batch_size, number_of_rounds)
            .ok_or(anyhow!("Failed to create digest key"))?;
        let (mpk, vks, msk_shares) = key_derivation::keygen(rng, tc);
        

        let ek = EncryptionKey {
            sig_mpk_g2: mpk.0,
            tau_g2: digest_key.tau_g2,
            id_set_capacity: max_batch_size,
        };

        Ok((ek, digest_key, vks, msk_shares))
    }

    fn encrypt<R: rand_core::CryptoRng + rand_core::RngCore>(ek: &Self::EncryptionKey, rng: &mut R, msg: &impl Plaintext) 
        -> anyhow::Result<Self::Ciphertext> {
        ek.encrypt(rng, msg)
    }

    fn digest<'a>(digest_key: &'a Self::DigestKey, cts: &[Self::Ciphertext], round: Self::Round, pool: &rayon::ThreadPool) 
        -> anyhow::Result<(Self::Digest, Self::EvalProofs<'a>)> 
    {
        let mut ids : FreeRootIdSet 
            = FreeRootIdSet::from_slice(
                &cts
                .into_iter()
                .map(|ct| ct.id())
                .collect::<Vec<FreeRootId>>())
            .ok_or(anyhow!(""))?;

        pool.install(|| digest_key.digest(&mut ids, round))
    }

    fn verify_ct(ct: &Self::Ciphertext) -> anyhow::Result<()> {
        ct.verify()
    }

    fn ct_id(ct: &Self::Ciphertext) -> Self::Id {
        ct.id()
    }

    fn eval_proofs_compute_all<'a>(proofs: &mut Self::EvalProofs<'a>, pool: &rayon::ThreadPool) {
        pool.install(|| proofs.compute_all())
    }



    fn derive_decryption_key_share(
        msk_share: &Self::MasterSecretKeyShare, 
        digest: &Self::Digest, 
        ) -> Result<Self::DecryptionKeyShare> {
        msk_share.derive_decryption_key_share(digest)
    }

    fn reconstruct_decryption_key(shares: &[Self::DecryptionKeyShare], config: &ThresholdConfig, pool: &rayon::ThreadPool)  -> anyhow::Result<Self::DecryptionKey> {
        pool.install(||
            BIBEDecryptionKey::reconstruct(shares, config)
        )
    }

    fn decrypt<'a, P: Plaintext>(
        decryption_key: &Self::DecryptionKey,
        cts: &[Self::Ciphertext], 
        proofs: &Self::EvalProofs<'a>, 
        pool: &rayon::ThreadPool
        ) -> anyhow::Result<Vec<P>> {
        pool.install(|| 
            cts.into_par_iter()
            .map(|ct| 
                {
                    let plaintext: Result<P> = decryption_key.decrypt(ct, proofs);
                    plaintext
                }
            )
            .collect::<anyhow::Result<Vec<P>>>()
        )
    }


    fn verify_decryption_key_share(
        verification_key_share: &Self::VerificationKey,
        digest: &Self::Digest,
        decryption_key_share: &Self::DecryptionKeyShare,
    ) -> anyhow::Result<()> {
        verification_key_share
            .verify_decryption_key_share(digest, decryption_key_share)
    }
}
