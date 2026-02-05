// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use crate::{
    errors::BatchEncryptionError,
    group::*,
    shared::{
        ciphertext::{CTDecrypt, CTEncrypt, PreparedCiphertext, StandardCiphertext},
        digest::{Digest, DigestKey, EvalProof, EvalProofs, EvalProofsPromise},
        encryption_key::EncryptionKey,
        ids::{Id, IdSet, UncomputedCoeffs},
        key_derivation::{
            self, BIBEDecryptionKey, BIBEDecryptionKeyShareValue, BIBEMasterSecretKeyShare,
            BIBEVerificationKey,
        },
    },
    traits::{
        AssociatedData, BatchThresholdEncryption, DecryptionKeyShare, Plaintext, VerificationKey,
    },
};
use anyhow::{anyhow, Result};
use aptos_crypto::{
    arkworks::serialization::{ark_de, ark_se},
    weighted_config::WeightedConfigArkworks,
    TSecretSharingConfig as _,
};
use aptos_dkg::pvss::{
    traits::{Reconstructable as _, Subtranscript},
    Player,
};
use ark_ec::AffineRepr;
use ark_ff::UniformRand as _;
use ark_std::rand::{rngs::StdRng, CryptoRng, RngCore, SeedableRng};
use rayon::iter::{IntoParallelIterator, ParallelIterator as _};
use serde::{Deserialize, Serialize};

pub struct FPTXWeighted {}

pub type WeightedBIBEDecryptionKeyShare = (Player, Vec<BIBEDecryptionKeyShareValue>);

impl DecryptionKeyShare for WeightedBIBEDecryptionKeyShare {
    fn player(&self) -> Player {
        self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WeightedBIBEMasterSecretKeyShare {
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub(crate) mpk_g2: G2Affine,
    pub(crate) weighted_player: Player,
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub(crate) shamir_share_evals: Vec<Fr>,
}

impl WeightedBIBEMasterSecretKeyShare {
    pub fn from_virtualized_sk_shares(
        weighted_player: Player,
        virtualized_msk_shares: &[BIBEMasterSecretKeyShare],
    ) -> Self {
        Self {
            mpk_g2: virtualized_msk_shares[0].mpk_g2,
            weighted_player,
            shamir_share_evals: virtualized_msk_shares
                .iter()
                .map(|share| share.shamir_share_eval)
                .collect(),
        }
    }

    pub fn virtualized_sk_shares(
        &self,
        tc: &WeightedConfigArkworks<Fr>,
    ) -> Vec<BIBEMasterSecretKeyShare> {
        tc.get_all_virtual_players(&self.weighted_player)
            .into_iter()
            .enumerate()
            .map(|(i, virt_player)| BIBEMasterSecretKeyShare {
                mpk_g2: self.mpk_g2,
                player: virt_player,
                shamir_share_eval: self.shamir_share_evals[i],
            })
            .collect()
    }

    pub fn derive_decryption_key_share(
        &self,
        digest: &Digest,
    ) -> Result<WeightedBIBEDecryptionKeyShare> {
        let evals_raw: Vec<G1Affine> = self
            .shamir_share_evals
            .iter()
            .map(|eval| {
                Ok(BIBEMasterSecretKeyShare {
                    mpk_g2: self.mpk_g2,
                    player: self.weighted_player, // arbitrary
                    shamir_share_eval: *eval,
                }
                .derive_decryption_key_share(digest)?
                .1
                .signature_share_eval)
            })
            .collect::<Result<Vec<G1Affine>>>()?;

        Ok((
            self.weighted_player,
            evals_raw
                .into_iter()
                .map(|eval| BIBEDecryptionKeyShareValue {
                    signature_share_eval: eval,
                })
                .collect(),
        ))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WeightedBIBEVerificationKey {
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub(crate) mpk_g2: G2Affine,
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub(crate) vks_g2: Vec<G2Affine>,
    pub(crate) weighted_player: Player,
}

impl WeightedBIBEVerificationKey {
    pub fn from_virtualized_vks(
        weighted_player: Player,
        virtualized_vks: &[BIBEVerificationKey],
    ) -> Self {
        Self {
            mpk_g2: virtualized_vks[0].mpk_g2,
            weighted_player,
            vks_g2: virtualized_vks.iter().map(|share| share.vk_g2).collect(),
        }
    }

    pub fn virtualized_vks(&self, tc: &WeightedConfigArkworks<Fr>) -> Vec<BIBEVerificationKey> {
        tc.get_all_virtual_players(&self.weighted_player)
            .into_iter()
            .enumerate()
            .map(|(i, virt_player)| BIBEVerificationKey {
                mpk_g2: self.mpk_g2,
                player: virt_player,
                vk_g2: self.vks_g2[i],
            })
            .collect()
    }

    pub fn verify_decryption_key_share(
        &self,
        digest: &Digest,
        dk_share: &WeightedBIBEDecryptionKeyShare,
    ) -> Result<()> {
        (self.vks_g2.len() == dk_share.1.len())
            .then_some(())
            .ok_or(BatchEncryptionError::DecryptionKeyVerifyError)?;

        self.vks_g2
            .iter()
            .map(|vk_g2| BIBEVerificationKey {
                mpk_g2: self.mpk_g2,
                vk_g2: *vk_g2,
                player: self.weighted_player, // arbitrary
            })
            .zip(&dk_share.1)
            .try_for_each(|(vk, dk_share)| {
                vk.verify_decryption_key_share(digest, &(self.weighted_player, dk_share.clone()))
            })
    }
}

impl VerificationKey for WeightedBIBEVerificationKey {
    fn player(&self) -> Player {
        self.weighted_player
    }
}

fn gen_weighted_msk_shares<R: RngCore + CryptoRng>(
    msk: Fr,
    rng: &mut R,
    tc: &WeightedConfigArkworks<Fr>,
) -> (
    G2Affine,
    Vec<WeightedBIBEVerificationKey>,
    Vec<WeightedBIBEMasterSecretKeyShare>,
) {
    let (mpk, virtualized_vks, virtualized_msk_shares) =
        key_derivation::gen_msk_shares(msk, rng, tc.get_threshold_config());

    let weighted_vks: Vec<WeightedBIBEVerificationKey> = tc
        .group_by_player(&virtualized_vks)
        .into_iter()
        .zip(tc.get_players())
        .map(|(vks_for_player, player)| {
            WeightedBIBEVerificationKey::from_virtualized_vks(player, &vks_for_player)
        })
        .collect();

    let weighted_msk_shares: Vec<WeightedBIBEMasterSecretKeyShare> = tc
        .group_by_player(&virtualized_msk_shares)
        .into_iter()
        .zip(tc.get_players())
        .map(|(shares_for_player, player)| {
            WeightedBIBEMasterSecretKeyShare::from_virtualized_sk_shares(player, &shares_for_player)
        })
        .collect();

    (mpk, weighted_vks, weighted_msk_shares)
}

impl BatchThresholdEncryption for FPTXWeighted {
    type Ciphertext = StandardCiphertext;
    type DecryptionKey = BIBEDecryptionKey;
    type DecryptionKeyShare = WeightedBIBEDecryptionKeyShare;
    type Digest = Digest;
    type DigestKey = DigestKey;
    type EncryptionKey = EncryptionKey;
    type EvalProof = EvalProof;
    type EvalProofs = EvalProofs;
    type EvalProofsPromise = EvalProofsPromise;
    type Id = Id;
    type MasterSecretKeyShare = WeightedBIBEMasterSecretKeyShare;
    type PreparedCiphertext = PreparedCiphertext;
    type Round = u64;
    type SubTranscript = aptos_dkg::pvss::chunky::WeightedSubtranscript<Pairing>;
    type ThresholdConfig = aptos_crypto::weighted_config::WeightedConfigArkworks<Fr>;
    type VerificationKey = WeightedBIBEVerificationKey;

    fn setup(
        digest_key: &Self::DigestKey,
        pvss_public_params: &<Self::SubTranscript as Subtranscript>::PublicParameters,
        subtranscript: &Self::SubTranscript,
        threshold_config: &Self::ThresholdConfig,
        current_player: Player,
        msk_share_decryption_key: &<Self::SubTranscript as Subtranscript>::DecryptPrivKey,
    ) -> Result<(
        Self::EncryptionKey,
        Vec<Self::VerificationKey>,
        Self::MasterSecretKeyShare,
    )> {
        let mpk_g2: G2Affine = subtranscript.get_dealt_public_key().as_g2();

        let ek = EncryptionKey::new(mpk_g2, digest_key.tau_g2);

        let vks: Vec<Self::VerificationKey> = threshold_config
            .get_players()
            .into_iter()
            .map(|p| Self::VerificationKey {
                weighted_player: p,
                mpk_g2,
                vks_g2: subtranscript
                    .get_public_key_share(threshold_config, &p)
                    .into_iter()
                    .map(|s| s.as_g2())
                    .collect(),
            })
            .collect();

        let msk_share = Self::MasterSecretKeyShare {
            mpk_g2,
            weighted_player: current_player,
            shamir_share_evals: subtranscript
                .decrypt_own_share(
                    threshold_config,
                    &current_player,
                    msk_share_decryption_key,
                    pvss_public_params,
                )
                .0
                .into_iter()
                .map(|s| s.into_fr())
                .collect(),
        };

        vks[msk_share.weighted_player.get_id()]
            .vks_g2
            .iter()
            .zip(msk_share.shamir_share_evals.clone())
            .try_for_each(|(vk_raw, msk_share_raw)| {
                (G2Projective::from(*vk_raw) == G2Affine::generator() * msk_share_raw)
                    .then_some(())
                    .ok_or(BatchEncryptionError::VKMSKMismatchError)
            })?;

        Ok((ek, vks, msk_share))
    }

    fn setup_for_testing(
        seed: u64,
        max_batch_size: usize,
        number_of_rounds: usize,
        threshold_config: &Self::ThresholdConfig,
    ) -> Result<(
        Self::EncryptionKey,
        Self::DigestKey,
        Vec<Self::VerificationKey>,
        Vec<Self::MasterSecretKeyShare>,
    )> {
        let mut rng = <StdRng as SeedableRng>::seed_from_u64(seed);

        let digest_key = DigestKey::new(&mut rng, max_batch_size, number_of_rounds)?;
        let msk = Fr::rand(&mut rng);
        let (mpk, vks, msk_shares) = gen_weighted_msk_shares(msk, &mut rng, threshold_config);

        let ek = EncryptionKey::new(mpk, digest_key.tau_g2);

        Ok((ek, digest_key, vks, msk_shares))
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
        let mut ids: IdSet<UncomputedCoeffs> =
            IdSet::from_slice(&cts.iter().map(|ct| ct.id()).collect::<Vec<Id>>())
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
        BIBEDecryptionKey::reconstruct(config, shares)
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
        verification_key: &Self::VerificationKey,
        digest: &Self::Digest,
        decryption_key_share: &Self::DecryptionKeyShare,
    ) -> anyhow::Result<()> {
        verification_key.verify_decryption_key_share(digest, decryption_key_share)
    }

    fn verify_decryption_key(
        encryption_key: &Self::EncryptionKey,
        digest: &Self::Digest,
        decryption_key: &Self::DecryptionKey,
    ) -> Result<()> {
        encryption_key.verify_decryption_key(digest, decryption_key)
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
