// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

/// A generic transformation from an unweighted PVSS to a weighted PVSS.
///
/// WARNING: This will **NOT** necessarily be secure for any PVSS scheme, since it will reuse encryption
/// keys, which might not be safe depending on the PVSS scheme.
use crate::pvss::{
    traits::{
        transcript::{Aggregatable, AggregatableTranscript, Aggregated, MalleableTranscript},
        Transcript,
    },
    Player, ThresholdConfigBlstrs, WeightedConfigBlstrs,
};
use aptos_crypto::{
    traits::TSecretSharingConfig as _, weighted_config::WeightedConfig, CryptoMaterialError,
    ValidCryptoMaterial,
};
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
        sc: &WeightedConfigBlstrs,
        eks: &[T::EncryptPubKey],
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

impl<T: Transcript<SecretSharingConfig = ThresholdConfigBlstrs>> Transcript
    for GenericWeighting<T>
{
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
    type SecretSharingConfig = WeightedConfigBlstrs;
    // Can probably change this to T::SecretSharingConfig, after editing Reconstructable... but not worth the effort atm
    type SigningPubKey = T::SigningPubKey;
    type SigningSecretKey = T::SigningSecretKey;

    fn dst() -> Vec<u8> {
        let mut result = b"WEIGHTED_".to_vec();
        result.extend(T::dst());
        result
    }

    fn scheme_name() -> String {
        format!("generic_weighted_{}", T::scheme_name())
    }

    fn deal<A: Serialize + Clone, R: RngCore + CryptoRng>(
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        ssk: &Self::SigningSecretKey,
        spk: &Self::SigningPubKey,
        eks: &[Self::EncryptPubKey],
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
                spk,
                &duplicated_eks,
                s,
                aux,
                dealer,
                rng,
            ),
        }
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

impl<T: AggregatableTranscript> AggregatableTranscript for GenericWeighting<T>
where
    T: Aggregatable<SecretSharingConfig = ThresholdConfigBlstrs>,
    T: Transcript<SecretSharingConfig = ThresholdConfigBlstrs>,
{
    fn verify<A: Serialize + Clone>(
        &self,
        sc: &<Self as Transcript>::SecretSharingConfig,
        pp: &Self::PublicParameters,
        spk: &[Self::SigningPubKey],
        eks: &[Self::EncryptPubKey],
        aux: &[A],
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
}

impl<T> Aggregatable for GenericWeighting<T>
where
    T: AggregatableTranscript
        + Aggregatable<SecretSharingConfig = ThresholdConfigBlstrs>
        + Transcript<SecretSharingConfig = ThresholdConfigBlstrs>,
    T::Aggregated: Aggregated<T>,
{
    type Aggregated = GenericWeighting<T::Aggregated>;
    type SecretSharingConfig = WeightedConfig<ThresholdConfigBlstrs>;

    fn to_aggregated(&self) -> Self::Aggregated {
        GenericWeighting {
            trx: self.trx.to_aggregated(),
        }
    }
}

impl<T> Aggregated<GenericWeighting<T>> for GenericWeighting<T::Aggregated>
where
    T: AggregatableTranscript
        + Aggregatable<SecretSharingConfig = ThresholdConfigBlstrs>
        + Transcript<SecretSharingConfig = ThresholdConfigBlstrs>,
    T::Aggregated: Aggregated<T>,
{
    fn aggregate_with(
        &mut self,
        sc: &WeightedConfig<ThresholdConfigBlstrs>,
        other: &GenericWeighting<T>,
    ) -> anyhow::Result<()> {
        // self.trx is T::Aggregated, other.trx is T
        // Aggregate other.trx into self.trx
        self.trx
            .aggregate_with(sc.get_threshold_config(), &other.trx)?;
        Ok(())
    }

    fn normalize(self) -> GenericWeighting<T> {
        // Convert T::Aggregated back to T
        GenericWeighting {
            trx: self.trx.normalize(),
        }
    }
}

impl<T: MalleableTranscript<SecretSharingConfig = ThresholdConfigBlstrs>> MalleableTranscript
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
