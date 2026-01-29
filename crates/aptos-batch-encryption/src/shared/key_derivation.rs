// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use super::symmetric;
use crate::{
    errors::BatchEncryptionError,
    group::{Fr, G1Affine, G2Affine, PairingSetting},
    shared::digest::Digest,
    traits::{DecryptionKeyShare, VerificationKey},
};
use anyhow::Result;
use aptos_crypto::{
    arkworks::{
        serialization::{ark_de, ark_se},
        shamir::{Reconstructable, ShamirGroupShare, ShamirThresholdConfig},
    },
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
pub struct BIBEDecryptionKeyShareValue {
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub(crate) signature_share_eval: G1Affine,
}

pub type BIBEDecryptionKeyShare = (Player, BIBEDecryptionKeyShareValue);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BIBEDecryptionKey {
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub signature_g1: G1Affine,
}

impl DecryptionKeyShare for BIBEDecryptionKeyShare {
    fn player(&self) -> Player {
        self.0
    }
}

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
    G2Affine,
    Vec<BIBEVerificationKey>,
    Vec<BIBEMasterSecretKeyShare>,
) {
    let mpk: G2Affine = (G2Affine::generator() * msk).into();

    let mut coeffs = vec![msk];
    coeffs.extend((0..(threshold_config.t - 1)).map(|_| Fr::rand(rng)));

    let (msk_shares, vk_shares): (Vec<BIBEMasterSecretKeyShare>, Vec<BIBEVerificationKey>) =
        threshold_config
            .share(&coeffs)
            .into_iter()
            .map(|(player, shamir_share_eval)| {
                (
                    BIBEMasterSecretKeyShare {
                        mpk_g2: mpk,
                        player,
                        shamir_share_eval,
                    },
                    BIBEVerificationKey {
                        mpk_g2: mpk,
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

        Ok((self.player, BIBEDecryptionKeyShareValue {
            signature_share_eval: G1Affine::from(
                (digest.as_g1() + hashed_encryption_key) * self.shamir_share_eval,
            ),
        }))
    }
}

/// Verify a signature under the shifted BLS variant used in our schemes.
pub fn verify_shifted_bls(
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
        verify_shifted_bls(
            self.vk_g2,
            digest,
            self.mpk_g2,
            decryption_key_share.1.signature_share_eval,
        )
        .map_err(|_| BatchEncryptionError::DecryptionKeyShareVerifyError)?;

        Ok(())
    }
}

impl Reconstructable<ShamirThresholdConfig<Fr>> for BIBEDecryptionKey {
    type ShareValue = BIBEDecryptionKeyShareValue;

    fn reconstruct(
        threshold_config: &ShamirThresholdConfig<Fr>,
        shares: &[BIBEDecryptionKeyShare],
    ) -> Result<Self> {
        let signature_g1 = G1Affine::reconstruct(
            threshold_config,
            &shares
                .iter()
                .map(|share| (share.0, share.1.signature_share_eval))
                .collect::<Vec<ShamirGroupShare<G1Affine>>>(),
        )?;

        // sanity check
        Ok(Self { signature_g1 })
    }
}

#[cfg(test)]
mod tests {
    use super::{gen_msk_shares, BIBEDecryptionKey, BIBEDecryptionKeyShare};
    use crate::{
        group::{Fr, G2Affine},
        shared::{digest::Digest, encryption_key::EncryptionKey},
    };
    use aptos_crypto::arkworks::shamir::ShamirThresholdConfig;
    use aptos_dkg::pvss::traits::Reconstructable as _;
    use ark_ec::AffineRepr;
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
        let dk = BIBEDecryptionKey::reconstruct(&tc, &shares_threshold).unwrap();

        EncryptionKey::new(mpk, G2Affine::generator())
            .verify_decryption_key(&digest, &dk)
            .expect("Decryption key should verify");
    }
}
