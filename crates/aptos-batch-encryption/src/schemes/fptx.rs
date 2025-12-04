// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{
    errors::BatchEncryptionError, group::{self, *}, shared::{
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
    }, traits::{AssociatedData, BatchThresholdEncryption, Plaintext}
};
use anyhow::{anyhow, Result};
use aptos_crypto::SecretSharingConfig as _;
use aptos_dkg::pvss::{traits::SubTranscript, Player};
use ark_ff::UniformRand as _;
use ark_std::rand::{rngs::StdRng, CryptoRng, RngCore, SeedableRng};
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
        Self { sig_mpk_g2, tau_g2 }
    }

    #[cfg(test)]
    pub(crate) fn new_for_testing() -> Self {
        use ark_ec::AffineRepr as _;

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
    type SubTranscript = aptos_dkg::pvss::chunky::SubTranscript<group::Pairing>;
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
    type ThresholdConfig = aptos_crypto::arkworks::shamir::ShamirThresholdConfig<Fr>;
    type VerificationKey = BIBEVerificationKey;

    fn setup(
        digest_key: &Self::DigestKey,
        pvss_public_params: &<Self::SubTranscript as SubTranscript>::PublicParameters,
        subtranscript_happypath: &Self::SubTranscript,
        subtranscript_slowpath: &Self::SubTranscript,
        tc_happypath: &Self::ThresholdConfig,
        tc_slowpath: &Self::ThresholdConfig,
        current_player: Player,
        msk_share_decryption_key: &<Self::SubTranscript as SubTranscript>::DecryptPrivKey,
    ) -> Result<(
        Self::EncryptionKey,
        Vec<Self::VerificationKey>,
        Self::MasterSecretKeyShare,
        Vec<Self::VerificationKey>,
        Self::MasterSecretKeyShare,
    )> {
        (subtranscript_happypath.get_dealt_public_key() ==
            subtranscript_slowpath.get_dealt_public_key())
            .then_some(())
            .ok_or(
                BatchEncryptionError::HappySlowPathMismatchError
            )?;

        let mpk_g2 : G2Affine = subtranscript_happypath.get_dealt_public_key().as_g2();

        let ek = EncryptionKey::new(mpk_g2, digest_key.tau_g2);

        let vks_happypath = tc_happypath
            .get_players()
            .into_iter()
            .map(|p|
                Self::VerificationKey {
                    player: p,
                    mpk_g2,
                    vk_g2: subtranscript_happypath.get_public_key_share(&tc_happypath, &p).as_g2()
                }
            ).collect();

        let vks_slowpath = tc_slowpath
            .get_players()
            .into_iter()
            .map(|p|
                Self::VerificationKey {
                    player: p,
                    mpk_g2,
                    vk_g2: subtranscript_slowpath.get_public_key_share(&tc_slowpath, &p).as_g2()
                }
            ).collect();

        let msk_share_happypath = BIBEMasterSecretKeyShare {
            mpk_g2,
            player: current_player,
            shamir_share_eval: subtranscript_happypath.decrypt_own_share(&tc_happypath, &current_player, &msk_share_decryption_key, &pvss_public_params).0.into_fr(),
        };

        let msk_share_slowpath = BIBEMasterSecretKeyShare {
            mpk_g2,
            player: current_player,
            shamir_share_eval: subtranscript_slowpath.decrypt_own_share(&tc_slowpath, &current_player, &msk_share_decryption_key, &pvss_public_params).0.into_fr(),
        };

        Ok((ek, vks_happypath, msk_share_happypath, vks_slowpath, msk_share_slowpath))
    }


    fn setup_for_testing(
        seed: u64,
        max_batch_size: usize,
        number_of_rounds: usize,
        tc_happypath: &Self::ThresholdConfig,
        tc_slowpath: &Self::ThresholdConfig,
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

    fn encrypt<R: CryptoRng + RngCore>(
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
    ) -> anyhow::Result<(Self::Digest, Self::EvalProofsPromise)> {
        let mut ids: FreeRootIdSet<UncomputedCoeffs> =
            FreeRootIdSet::from_slice(&cts.iter().map(|ct| ct.id()).collect::<Vec<FreeRootId>>())
                .ok_or(anyhow!(""))?;

        digest_key.digest(&mut ids, round)
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
    ) -> Self::EvalProofs {
        proofs.compute_all(digest_key)
    }

    fn eval_proofs_compute_all_vzgg_multi_point_eval(
        proofs: &Self::EvalProofsPromise,
        digest_key: &DigestKey,
    ) -> Self::EvalProofs {
        proofs.compute_all_vgzz_multi_point_eval(digest_key)
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
        config: &Self::ThresholdConfig,
    ) -> anyhow::Result<Self::DecryptionKey> {
        BIBEDecryptionKey::reconstruct(shares, config)
    }

    fn prepare_cts(
        cts: &[Self::Ciphertext],
        digest: &Self::Digest,
        eval_proofs: &Self::EvalProofs,
    ) -> Result<Vec<Self::PreparedCiphertext>> {
        cts.into_par_iter()
            .map(|ct| ct.prepare(digest, eval_proofs))
            .collect::<anyhow::Result<Vec<Self::PreparedCiphertext>>>()
    }

    fn decrypt<'a, P: Plaintext>(
        decryption_key: &Self::DecryptionKey,
        cts: &[Self::PreparedCiphertext],
    ) -> anyhow::Result<Vec<P>> {
        cts.into_par_iter()
            .map(|ct| {
                let plaintext: Result<P> = decryption_key.decrypt(ct);
                plaintext
            })
            .collect::<anyhow::Result<Vec<P>>>()
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
