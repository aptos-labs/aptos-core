// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use super::symmetric;
use crate::{
    errors::{BatchEncryptionError, ReconstructError},
    group::{Fr, G1Affine, G2Affine, PairingSetting},
    shared::{ark_serialize::*, digest::Digest},
    traits::{DecryptionKeyShare, VerificationKey},
};
use anyhow::Result;
use aptos_crypto::{
    arkworks::shamir::{Reconstructable, ShamirGroupShare, ShamirThresholdConfig},
    player::Player,
};
use ark_ec::{pairing::Pairing as _, AffineRepr};
use ark_ff::UniformRand as _;
use ark_std::rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BIBEMasterSecretKeyShare {
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub(crate) mpk_g2: G2Affine,
    pub(crate) player: Player,
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub(crate) shamir_share_eval: Fr,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BIBEDecryptionKeyShare {
    player: Player,
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    signature_share_eval: G1Affine,
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    digest_g1: G1Affine,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BIBEDecryptionKey {
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub digest_g1: G1Affine,
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub signature_g1: G1Affine,
}

impl DecryptionKeyShare for BIBEDecryptionKeyShare {
    fn player(&self) -> Player {
        self.player
    }
}

pub struct BIBEMasterPublicKey(pub(crate) G2Affine);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BIBEVerificationKey {
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub(crate) mpk_g2: G2Affine,
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub(crate) vk_g2: G2Affine,
    pub(crate) player: Player,
}

impl VerificationKey for BIBEVerificationKey {
    fn player(&self) -> Player {
        self.player
    }
}

pub fn gen_msk_shares<R: RngCore + CryptoRng>(
    msk: Fr,
    rng: &mut R,
    threshold_config: &ShamirThresholdConfig<Fr>,
) -> (
    BIBEMasterPublicKey,
    Vec<BIBEVerificationKey>,
    Vec<BIBEMasterSecretKeyShare>,
) {
    let mpk = BIBEMasterPublicKey((G2Affine::generator() * msk).into());

    let mut coeffs = vec![msk];
    coeffs.extend((0..(threshold_config.t - 1)).map(|_| Fr::rand(rng)));

    let (msk_shares, vk_shares): (Vec<BIBEMasterSecretKeyShare>, Vec<BIBEVerificationKey>) =
        threshold_config
            .share(&coeffs)
            .into_iter()
            .map(|(player, shamir_share_eval)| {
                (
                    BIBEMasterSecretKeyShare {
                        mpk_g2: mpk.0,
                        player,
                        shamir_share_eval,
                    },
                    BIBEVerificationKey {
                        mpk_g2: mpk.0,
                        vk_g2: (G2Affine::generator() * shamir_share_eval).into(),
                        player,
                    },
                )
            })
            .collect();

    (mpk, vk_shares, msk_shares)
}

impl BIBEMasterSecretKeyShare {
    pub fn derive_decryption_key_share(&self, digest: &Digest) -> Result<BIBEDecryptionKeyShare> {
        let hashed_encryption_key: G1Affine = symmetric::hash_g2_element(self.mpk_g2)?;

        Ok(BIBEDecryptionKeyShare {
            player: self.player,
            signature_share_eval: G1Affine::from(
                (digest.as_g1() + hashed_encryption_key) * self.shamir_share_eval,
            ),
            digest_g1: digest.as_g1(),
        })
    }
}



fn verify_bls(
    verification_key_g2: G2Affine,
    digest: &Digest,
    offset: G2Affine,
    signature: G1Affine,
) -> Result<()> {
    let hashed_offset: G1Affine = symmetric::hash_g2_element(offset)?;

    if PairingSetting::pairing(digest.as_g1() + hashed_offset, verification_key_g2)
        == PairingSetting::pairing(signature, G2Affine::generator())
    {
        Ok(())
    } else {
        Err(anyhow::anyhow!("bls verification error"))
    }
}

impl BIBEVerificationKey {
    pub fn verify_decryption_key_share(
        &self,
        digest: &Digest,
        decryption_key_share: &BIBEDecryptionKeyShare,
    ) -> Result<()> {
        verify_bls(
            self.vk_g2,
            digest,
            self.mpk_g2,
            decryption_key_share.signature_share_eval,
        )
        .map_err(|_| BatchEncryptionError::DecryptionKeyShareVerifyError)?;

        Ok(())
    }
}

impl BIBEMasterPublicKey {
    pub fn verify_decryption_key(
        &self,
        digest: &Digest,
        decryption_key: &BIBEDecryptionKey,
    ) -> Result<()> {
        verify_bls(self.0, digest, self.0, decryption_key.signature_g1)
            .map_err(|_| BatchEncryptionError::DecryptionKeyVerifyError)?;

        Ok(())
    }
}

impl BIBEDecryptionKey {
    pub fn reconstruct(
        shares: &[BIBEDecryptionKeyShare],
        threshold_config: &ShamirThresholdConfig<Fr>,
    ) -> Result<Self> {
        let signature_g1 = G1Affine::reconstruct(
            threshold_config,
            &shares
                .iter()
                .map(|share| (share.player, share.signature_share_eval))
                .collect::<Vec<ShamirGroupShare<G1Affine>>>(),
        )?;

        let digest_g1 = shares[0].digest_g1;

        // sanity check
        if !shares.iter().all(|share| share.digest_g1 == digest_g1) {
            Err(ReconstructError::ReconstructDigestsDontMatch)?
        } else {
            Ok(Self {
                digest_g1,
                signature_g1,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{gen_msk_shares, BIBEDecryptionKey, BIBEDecryptionKeyShare};
    use crate::{group::Fr, shared::digest::Digest};
    use aptos_crypto::arkworks::shamir::ShamirThresholdConfig;
    use ark_ff::UniformRand as _;
    use ark_std::rand::{seq::SliceRandom, thread_rng};

    #[test]
    fn test_reconstruct_verify() {
        let mut rng = thread_rng();
        let n = 8;
        let t = 6;
        let tc = ShamirThresholdConfig::new(t, n);
        let msk = Fr::rand(&mut rng);
        let (mpk, vks, msk_shares) = gen_msk_shares(msk, &mut rng, &tc);
        let digest = Digest::new_for_testing(&mut rng);

        let mut dk_shares = vec![];

        for (msk_share, vk) in msk_shares.into_iter().zip(vks) {
            let dk_share = msk_share.derive_decryption_key_share(&digest).unwrap();
            vk.verify_decryption_key_share(&digest, &dk_share)
                .expect("Each decryption key share should verify");
            dk_shares.push(dk_share);
        }

        let shares_threshold: Vec<BIBEDecryptionKeyShare> =
            dk_shares.choose_multiple(&mut rng, 6).cloned().collect();
        let dk = BIBEDecryptionKey::reconstruct(&shares_threshold, &tc).unwrap();

        mpk.verify_decryption_key(&digest, &dk)
            .expect("Decryption key should verify");
    }
}
