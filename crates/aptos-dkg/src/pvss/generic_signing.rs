// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::traits::Transcript;
use aptos_crypto::bls12381;
use serde::Serialize;
use rand_core::CryptoRng;
use rand_core::RngCore;
use aptos_crypto::player::Player;
use aptos_crypto::SigningKey;
use serde::Deserialize;
use aptos_crypto_derive::CryptoHasher;
use aptos_crypto_derive::BCSCryptoHash;
use aptos_crypto::ValidCryptoMaterial;
use aptos_crypto::CryptoMaterialError;

/// A generic transformation from a non-malleable PVSS to a signed and non-malleable PVSS.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GenericSigning<T> {
    trs: T,
    sig: bls12381::Signature
}

impl<T: Transcript> ValidCryptoMaterial for GenericSigning<T> {
    const AIP_80_PREFIX: &'static str = "";

    fn to_bytes(&self) -> Vec<u8> {
        // TODO: using `Result<Vec<u8>>` and `.map_err(|_| CryptoMaterialError::DeserializationError)` would be more consistent here?
        bcs::to_bytes(&self).expect("Unexpected error during PVSS transcript serialization")
    }
}

impl<T: Transcript> TryFrom<&[u8]> for GenericSigning<T> {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        bcs::from_bytes::<GenericSigning<T>>(bytes)
            .map_err(|_| CryptoMaterialError::DeserializationError)
    }
}

#[derive(Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct Contribution<T, S> {
    pub trs: T,
    pub session_id: S,
}

/// Currently has requirements on the `SigningPubKey` and `SigningSecretKey`, in order 
/// to get a signature of type `bls12381::Signature`; this can be relaxed
impl<T: Transcript<SigningPubKey = bls12381::PublicKey, SigningSecretKey = bls12381::PrivateKey>> Transcript
    for GenericSigning<T>
{
    type DealtPubKey = T::DealtPubKey;
    type DealtPubKeyShare = T::DealtPubKeyShare;
    type DealtSecretKey = T::DealtSecretKey;
    type DealtSecretKeyShare = T::DealtSecretKeyShare;
    type DecryptPrivKey = T::DecryptPrivKey;
    type EncryptPubKey = T::EncryptPubKey;
    type InputSecret = T::InputSecret;
    type PublicParameters = T::PublicParameters;
    type SecretSharingConfig = T::SecretSharingConfig;
    type SigningPubKey = T::SigningPubKey;
    type SigningSecretKey = T::SigningSecretKey;

    fn dst() -> Vec<u8> {
        let mut result = b"SIGNED_".to_vec();
        result.extend(T::dst());
        result
    }

    fn scheme_name() -> String {
        format!("signed_{}", T::scheme_name())
    }

    fn deal<A: Serialize + Clone, R: rand_core::RngCore + rand_core::CryptoRng>(
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        ssk: &Self::SigningSecretKey,
        spk: &Self::SigningPubKey,
        eks: &Vec<Self::EncryptPubKey>,
        s: &Self::InputSecret,
        session_id: &A,
        dealer: &Player,
        rng: &mut R,
    ) -> Self {

        let trs = T::deal(
                sc,
                pp,
                ssk,
                spk,
                eks,
                s,
                session_id,
                dealer,
                rng,
            );

        // Sign the contribution
        let sig = ssk
            .sign(&Contribution {
                trs: trs.clone(), session_id
            })
            .expect("signing of `chunky` PVSS transcript failed");

        GenericSigning {
            trs,
            sig
        }
    }

    #[allow(non_snake_case)]
    fn verify<A: Serialize + Clone>(
        &self,
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        spks: &Vec<Self::SigningPubKey>,
        eks: &Vec<Self::EncryptPubKey>,
        session_ids: &Vec<A>,
    ) -> anyhow::Result<()> {
        if eks.len() != sc.n {
            bail!("Expected {} encryption keys, but got {}", sc.n, eks.len());
        }
        if self.subtranscript.Cs.len() != sc.n {
            bail!(
                "Expected {} arrays of chunked ciphertexts, but got {}",
                sc.n,
                self.subtranscript.Cs.len()
            );
        }
        if self.subtranscript.Vs.len() != sc.n + 1 {
            bail!(
                "Expected {} commitment elements, but got {}",
                sc.n + 1,
                self.subtranscript.Vs.len()
            );
        }

        // Initialize the **identical** PVSS SoK context
        let sok_cntxt = (
            *sc,
            &spks[self.dealer.id],
            session_ids[self.dealer.id].clone(),
            self.dealer.id,
            DST.to_vec(),
        ); // As above, this is a bit hacky... though we have access to `self` now

        // Verify the transcript signature
        self.sgn.verify(
            &Contribution::<E> {
                comm: *self.subtranscript.Vs.last().unwrap(),
            },
            &spks[self.dealer.id],
        )?;
    }

    fn verify<A: Serialize + Clone>(
        &self,
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        spk: &Vec<Self::SigningPubKey>,
        eks: &Vec<Self::EncryptPubKey>,
        aux: &Vec<A>,
    ) -> anyhow::Result<()> {
        let duplicated_eks = GenericWeighting::<T>::to_weighted_encryption_keys(sc, eks);

        T::verify(
            &self.trx,
            sc.get_threshold_config(),
            pp,
            spk,
            &duplicated_eks,
            aux,
        )
    }

    fn get_dealers(&self) -> Vec<Player> {
        T::get_dealers(&self.trx)
    }

    fn get_public_key_share(
        &self,
        sc: &Self::SecretSharingConfig,
        player: &Player,
    ) -> Self::DealtPubKeyShare {
        let weight = sc.get_player_weight(player);

        let mut dpk_share = Vec::with_capacity(weight);

        for i in 0..weight {
            // println!("Decrypting share {i} for player {player} with DK {:?}", dk);
            let virtual_player = sc.get_virtual_player(player, i);
            dpk_share.push(T::get_public_key_share(
                &self.trx,
                sc.get_threshold_config(),
                &virtual_player,
            ));
        }

        dpk_share
    }

    fn get_dealt_public_key(&self) -> Self::DealtPubKey {
        T::get_dealt_public_key(&self.trx)
    }

    fn decrypt_own_share(
        &self,
        sc: &Self::SecretSharingConfig,
        player: &Player,
        dk: &Self::DecryptPrivKey,
        pp: &Self::PublicParameters,
    ) -> (Self::DealtSecretKeyShare, Self::DealtPubKeyShare) {
        let weight = sc.get_player_weight(player);

        let mut weighted_dsk_share = Vec::with_capacity(weight);
        let mut weighted_dpk_share = Vec::with_capacity(weight);

        for i in 0..weight {
            // println!("Decrypting share {i} for player {player} with DK {:?}", dk);
            let virtual_player = sc.get_virtual_player(player, i);
            let (dsk_share, dpk_share) = T::decrypt_own_share(
                &self.trx,
                sc.get_threshold_config(),
                &virtual_player,
                dk,
                pp,
            );
            weighted_dsk_share.push(dsk_share);
            weighted_dpk_share.push(dpk_share);
        }

        (weighted_dsk_share, weighted_dpk_share)
    }

    fn generate<R>(sc: &Self::SecretSharingConfig, pp: &Self::PublicParameters, rng: &mut R) -> Self
    where
        R: RngCore + CryptoRng,
    {
        GenericWeighting {
            trx: T::generate(sc.get_threshold_config(), pp, rng),
        }
    }
}