// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// A generic transformation from an unweighted PVSS to a weighted PVSS.
///
/// WARNING: This will **NOT** necessarily be secure for any PVSS scheme, since it will reuse encryption
/// keys, which might not be safe depending on the PVSS scheme.
use crate::pvss::{
    traits::{transcript::MalleableTranscript, Reconstructable, SecretSharingConfig, Transcript},
    Player, ThresholdConfig, WeightedConfig,
};
use aptos_crypto::{CryptoMaterialError, ValidCryptoMaterial};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, CryptoHasher, BCSCryptoHash)]
/// A weighting wrapper around a `Transcript` type `T`. Given an implementation of an [unweighted
/// PVSS] `Transcript` for `T`, this wrapper can be used to easily obtain a *weighted* PVSS abiding
/// by the same `Transcript` trait.
pub struct GenericWeighting<T> {
    trx: T,
}

/// Implements weighted reconstruction of a secret `SK` through the existing unweighted reconstruction
/// implementation of `SK`.
impl<SK: Reconstructable<ThresholdConfig>> Reconstructable<WeightedConfig> for SK {
    type Share = Vec<SK::Share>;

    fn reconstruct(sc: &WeightedConfig, shares: &Vec<(Player, Self::Share)>) -> Self {
        let mut flattened_shares = Vec::with_capacity(sc.get_total_weight());

        // println!();
        for (player, sub_shares) in shares {
            // println!(
            //     "Flattening {} share(s) for player {player}",
            //     sub_shares.len()
            // );
            for (pos, share) in sub_shares.iter().enumerate() {
                let virtual_player = sc.get_virtual_player(player, pos);

                // println!(
                //     " + Adding share {pos} as virtual player {virtual_player}: {:?}",
                //     share
                // );
                // TODO(Performance): Avoiding the cloning here might be nice
                let tuple = (virtual_player, share.clone());
                flattened_shares.push(tuple);
            }
        }

        SK::reconstruct(sc.get_threshold_config(), &flattened_shares)
    }
}

impl<T: Transcript> ValidCryptoMaterial for GenericWeighting<T> {
    const AIP_80_PREFIX: &'static str = "";

    fn to_bytes(&self) -> Vec<u8> {
        self.trx.to_bytes()
    }
}

impl<T: Transcript> TryFrom<&[u8]> for GenericWeighting<T> {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        T::try_from(bytes).map(|trx| Self { trx })
    }
}

impl<T: Transcript> GenericWeighting<T> {
    fn to_weighted_encryption_keys(
        sc: &WeightedConfig,
        eks: &Vec<T::EncryptPubKey>,
    ) -> Vec<T::EncryptPubKey> {
        // Re-organize the encryption key vector so that we deal multiple shares to each player,
        // proportional to their weight.
        let mut duplicated_eks = Vec::with_capacity(sc.get_total_weight());

        for (player_id, ek) in eks.iter().enumerate() {
            let player = sc.get_player(player_id);
            let num_shares = sc.get_player_weight(&player);
            for _ in 0..num_shares {
                duplicated_eks.push(ek.clone());
            }
        }

        duplicated_eks
    }
}

impl<T: Transcript<SecretSharingConfig = ThresholdConfig>> Transcript for GenericWeighting<T> {
    type DealtPubKey = T::DealtPubKey;
    type DealtPubKeyShare = Vec<T::DealtPubKeyShare>;
    type DealtSecretKey = T::DealtSecretKey;
    /// In a weighted PVSS, an SK share is represented as a vector of SK shares in the unweighted
    /// PVSS, whose size is proportional to the weight of the owning player.
    type DealtSecretKeyShare = Vec<T::DealtSecretKeyShare>;
    type DecryptPrivKey = T::DecryptPrivKey;
    type EncryptPubKey = T::EncryptPubKey;
    type InputSecret = T::InputSecret;
    type PublicParameters = T::PublicParameters;
    type SecretSharingConfig = WeightedConfig;
    type SigningPubKey = T::SigningPubKey;
    type SigningSecretKey = T::SigningSecretKey;

    fn scheme_name() -> String {
        format!("generic_weighted_{}", T::scheme_name())
    }

    fn deal<A: Serialize + Clone, R: RngCore + CryptoRng>(
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        ssk: &Self::SigningSecretKey,
        eks: &Vec<Self::EncryptPubKey>,
        s: &Self::InputSecret,
        aux: &A,
        dealer: &Player,
        rng: &mut R,
    ) -> Self {
        // WARNING: This duplication of encryption keys will NOT be secure in some PVSS schemes.
        let duplicated_eks = GenericWeighting::<T>::to_weighted_encryption_keys(sc, eks);

        GenericWeighting {
            trx: T::deal(
                sc.get_threshold_config(),
                pp,
                ssk,
                &duplicated_eks,
                s,
                aux,
                dealer,
                rng,
            ),
        }
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

    fn aggregate_with(&mut self, sc: &Self::SecretSharingConfig, other: &Self) {
        T::aggregate_with(&mut self.trx, sc.get_threshold_config(), &other.trx)
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
    ) -> (Self::DealtSecretKeyShare, Self::DealtPubKeyShare) {
        let weight = sc.get_player_weight(player);

        let mut weighted_dsk_share = Vec::with_capacity(weight);
        let mut weighted_dpk_share = Vec::with_capacity(weight);

        for i in 0..weight {
            // println!("Decrypting share {i} for player {player} with DK {:?}", dk);
            let virtual_player = sc.get_virtual_player(player, i);
            let (dsk_share, dpk_share) =
                T::decrypt_own_share(&self.trx, sc.get_threshold_config(), &virtual_player, dk);
            weighted_dsk_share.push(dsk_share);
            weighted_dpk_share.push(dpk_share);
        }

        (weighted_dsk_share, weighted_dpk_share)
    }

    fn generate<R>(sc: &Self::SecretSharingConfig, rng: &mut R) -> Self
    where
        R: RngCore + CryptoRng,
    {
        GenericWeighting {
            trx: T::generate(sc.get_threshold_config(), rng),
        }
    }
}

impl<T: MalleableTranscript<SecretSharingConfig = ThresholdConfig>> MalleableTranscript
    for GenericWeighting<T>
{
    fn maul_signature<A: Serialize + Clone>(
        &mut self,
        ssk: &Self::SigningSecretKey,
        aux: &A,
        dealer: &Player,
    ) {
        <T as MalleableTranscript>::maul_signature(&mut self.trx, ssk, aux, dealer);
    }
}
