use crate::{
    group::*,
    shared::{
        algebra::shamir::ThresholdConfig,
        ark_serialize::*,
        ciphertext::{BIBEEncryptionKey, CTDecrypt, CTEncrypt, Ciphertext, PreparedCiphertext},
        digest::{Digest, DigestKey, EvalProofs, EvalProofsPromise},
        ids::{
            free_roots::{ComputedCoeffs, UncomputedCoeffs},
            FreeRootId, FreeRootIdSet, IdSet,
        },
        key_derivation::{
            self, BIBEDecryptionKey, BIBEDecryptionKeyShare, BIBEMasterPublicKey,
            BIBEMasterSecretKeyShare, BIBEVerificationKey,
        },
    },
    traits::{AssociatedData, BatchThresholdEncryption, Plaintext},
};
use anyhow::{anyhow, Result};
use ark_ec::AffineRepr;
use ark_ff::UniformRand as _;
use ark_std::rand::rngs::StdRng;
use rand_core::SeedableRng;
use rayon::iter::{IntoParallelIterator, ParallelIterator as _};
use serde::{Deserialize, Serialize};

pub struct FPTX {}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptionKey {
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    sig_mpk_g2: G2Affine,
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    tau_g2: G2Affine,
}

impl EncryptionKey {
    pub fn new(sig_mpk_g2: G2Affine, tau_g2: G2Affine) -> Self {
        Self {
            sig_mpk_g2,
            tau_g2,
        }
    }

    #[cfg(test)]
    pub(crate) fn new_for_testing() -> Self {
        Self { 
            sig_mpk_g2: G2Affine::generator(),
            tau_g2: G2Affine::generator(),
        }
    }

    pub fn verify_decryption_key(
        &self,
        digest: &Digest,
        decryption_key: &BIBEDecryptionKey,
    ) -> Result<()> {
        BIBEMasterPublicKey(self.sig_mpk_g2).verify_decryption_key(digest, decryption_key)
    }
}

impl BIBEEncryptionKey for EncryptionKey {
    fn sig_mpk_g2(&self) -> G2Affine {
        self.sig_mpk_g2
    }

    fn tau_g2(&self) -> G2Affine {
        self.tau_g2
    }
}

impl BatchThresholdEncryption for FPTX {
    type Ciphertext = Ciphertext<FreeRootId>;
    type DecryptionKey = BIBEDecryptionKey;
    type DecryptionKeyShare = BIBEDecryptionKeyShare;
    type Digest = Digest;
    type DigestKey = DigestKey;
    type EncryptionKey = EncryptionKey;
    type EvalProof = G1Affine;
    type EvalProofs = EvalProofs<FreeRootIdSet<ComputedCoeffs>>;
    type EvalProofsPromise = EvalProofsPromise<FreeRootIdSet<ComputedCoeffs>>;
    type Id = FreeRootId;
    type MasterSecretKeyShare = BIBEMasterSecretKeyShare;
    type PreparedCiphertext = PreparedCiphertext;
    type Round = u64;
    type VerificationKey = BIBEVerificationKey;

    fn setup_for_testing(
        seed: u64,
        max_batch_size: usize,
        number_of_rounds: usize,
        tc_happypath: &ThresholdConfig,
        tc_slowpath: &ThresholdConfig,
    ) -> Result<(
        Self::EncryptionKey,
        Self::DigestKey,
        Vec<Self::VerificationKey>,
        Vec<Self::MasterSecretKeyShare>,
        Vec<Self::VerificationKey>,
        Vec<Self::MasterSecretKeyShare>,
    )> {
        let mut rng = <StdRng as SeedableRng>::seed_from_u64(seed);

        let digest_key = DigestKey::new(&mut rng, max_batch_size, number_of_rounds)
            .ok_or(anyhow!("Failed to create digest key"))?;
        let msk = Fr::rand(&mut rng);
        let (mpk, vks_happypath, msk_shares_happypath) =
            key_derivation::gen_msk_shares(msk, &mut rng, tc_happypath);
        let (_, vks_slowpath, msk_shares_slowpath) =
            key_derivation::gen_msk_shares(msk, &mut rng, tc_slowpath);

        let ek = EncryptionKey {
            sig_mpk_g2: mpk.0,
            tau_g2: digest_key.tau_g2,
        };

        Ok((
            ek,
            digest_key,
            vks_happypath,
            msk_shares_happypath,
            vks_slowpath,
            msk_shares_slowpath,
        ))
    }

    fn encrypt<R: rand_core::CryptoRng + rand_core::RngCore>(
        ek: &Self::EncryptionKey,
        rng: &mut R,
        msg: &impl Plaintext,
        associated_data: &impl AssociatedData,
    ) -> anyhow::Result<Self::Ciphertext> {
        ek.encrypt(rng, msg, associated_data)
    }

    fn digest(
        digest_key: &Self::DigestKey,
        cts: &[Self::Ciphertext],
        round: Self::Round,
        pool: &rayon::ThreadPool,
    ) -> anyhow::Result<(Self::Digest, Self::EvalProofsPromise)> {
        let mut ids: FreeRootIdSet<UncomputedCoeffs> = FreeRootIdSet::from_slice(
            &cts.into_iter()
                .map(|ct| ct.id())
                .collect::<Vec<FreeRootId>>(),
        )
        .ok_or(anyhow!(""))?;

        pool.install(|| digest_key.digest(&mut ids, round))
    }

    fn verify_ct(
        ct: &Self::Ciphertext,
        associated_data: &impl AssociatedData,
    ) -> anyhow::Result<()> {
        ct.verify(associated_data)
    }

    fn ct_id(ct: &Self::Ciphertext) -> Self::Id {
        ct.id()
    }

    fn eval_proofs_compute_all(
        proofs: &Self::EvalProofsPromise,
        digest_key: &DigestKey,
        pool: &rayon::ThreadPool,
    ) -> Self::EvalProofs {
        pool.install(|| proofs.compute_all(digest_key))
    }

    fn eval_proofs_compute_all_2(
        proofs: &Self::EvalProofsPromise,
        digest_key: &DigestKey,
        pool: &rayon::ThreadPool,
    ) -> Self::EvalProofs {
        pool.install(|| proofs.compute_all_2(digest_key))
    }

    fn eval_proof_for_ct(
        proofs: &Self::EvalProofs,
        ct: &Self::Ciphertext,
    ) -> Option<Self::EvalProof> {
        proofs.get(&ct.id())
    }

    fn derive_decryption_key_share(
        msk_share: &Self::MasterSecretKeyShare,
        digest: &Self::Digest,
    ) -> Result<Self::DecryptionKeyShare> {
        msk_share.derive_decryption_key_share(digest)
    }

    fn reconstruct_decryption_key(
        shares: &[Self::DecryptionKeyShare],
        config: &ThresholdConfig,
        pool: &rayon::ThreadPool,
    ) -> anyhow::Result<Self::DecryptionKey> {
        pool.install(|| BIBEDecryptionKey::reconstruct(shares, config))
    }

    fn prepare_cts(
        cts: &[Self::Ciphertext],
        digest: &Self::Digest,
        eval_proofs: &Self::EvalProofs,
        pool: &rayon::ThreadPool,
    ) -> Result<Vec<Self::PreparedCiphertext>> {
        pool.install(|| {
            cts.into_par_iter()
                .map(|ct| ct.prepare(digest, eval_proofs))
                .collect::<anyhow::Result<Vec<Self::PreparedCiphertext>>>()
        })
    }

    fn decrypt<'a, P: Plaintext>(
        decryption_key: &Self::DecryptionKey,
        cts: &[Self::PreparedCiphertext],
        pool: &rayon::ThreadPool,
    ) -> anyhow::Result<Vec<P>> {
        pool.install(|| {
            cts.into_par_iter()
                .map(|ct| {
                    let plaintext: Result<P> = decryption_key.decrypt(ct);
                    plaintext
                })
                .collect::<anyhow::Result<Vec<P>>>()
        })
    }

    fn verify_decryption_key_share(
        verification_key_share: &Self::VerificationKey,
        digest: &Self::Digest,
        decryption_key_share: &Self::DecryptionKeyShare,
    ) -> anyhow::Result<()> {
        verification_key_share.verify_decryption_key_share(digest, decryption_key_share)
    }

    fn decrypt_individual<P: Plaintext>(
        decryption_key: &Self::DecryptionKey,
        ct: &Self::Ciphertext,
        digest: &Self::Digest,
        eval_proof: &Self::EvalProof,
    ) -> Result<P> {
        decryption_key.decrypt(&ct.prepare_individual(digest, eval_proof)?)
    }
}
