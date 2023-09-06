// Copyright © Aptos Foundation

use crate::pvss::traits::{
    Convert, IsSecretShareable, Reconstructable, SecretSharingConfig, Transcript,
};
use crate::pvss::{Player, ThresholdConfig, WeightedConfig};
use aptos_crypto::{CryptoMaterialError, ValidCryptoMaterial};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher, SilentDebug, SilentDisplay};
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, CryptoHasher, BCSCryptoHash)]
/// A weighting wrapper around a `Transcript` type `T`. Given an implementation of an [unweighted
/// PVSS] `Transcript` for `T`, this wrapper can be used to easily obtain a *weighted* PVSS abiding
/// by the same `Transcript` trait.
pub struct WeightedTranscript<T> {
    trx: T,
}

#[derive(SilentDisplay, SilentDebug, PartialEq)]
/// Wrapper around a key, whether a `Transcript::DealtSecretKey` or a `Transcript::DealtSecretKeyShare`.
/// Helps us override the `Reconstructable` trait for a weighted dealt secret key, which is
/// implemented as a `Wrapper<Transcript::DealtSecretKey>` and has a
/// `Vec<Transcript::DealtSecretKeyShare>` as its associated `Share` type (via the `IsSecretShareable`
/// trait).
pub struct WeightedKey<Key> {
    key: Key,
}

// impl<Key> fmt::Debug for WeightedKey<Key>
// where
//     Key: Debug,
// {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         write!(f, "{:?}", self.key)
//     }
// }

impl<Key> WeightedKey<Key> {
    /// Helpful for debugging in tests by calling `WeightedKey<Key>::sub_key().to_bytes()` since I could
    /// not implement the `ValidCryptoMaterial` trait here due to the non-generic `DeserializeKey`
    /// procedural macro, which I could not fix up.
    pub fn sub_key(&self) -> &Key {
        &self.key
    }
}

/// Implements conversion from `T::InputSecret` to `WeightedKey<T::DealtSecretKey>` and
/// `WeightedKey<T::DealtPubKey>` where `T` is an unweighted `Transcript`.
impl<InputSecret, Key, PublicParameters> Convert<WeightedKey<Key>, PublicParameters> for InputSecret
where
    InputSecret: Convert<Key, PublicParameters>,
{
    fn to(&self, with: &PublicParameters) -> WeightedKey<Key> {
        WeightedKey { key: self.to(with) }
    }
}

/// In a weighted PVSS transcript, each player gets a number of shares proportional to that player's
/// weight. As a result, the typing of a *weighted* dealt secret key share needs to now be a vector
/// of *unweighted* dealt secret key shares.
///
/// Associates `Vec<SK::Share>` as the dealt secret key share type of a `WeightedKey<T::SK>`, where `T`
/// is in an unweighted `Transcript`.
impl<SK: IsSecretShareable> IsSecretShareable for WeightedKey<SK> {
    type Share = Vec<SK::Share>;
}

/// Implements weighted reconstruction of a secret `WeightedKey<SK>` through the existing unweighted
/// reconstruction implementation of `SK`.
impl<SK: IsSecretShareable + Reconstructable<SecretSharingConfig = ThresholdConfig>> Reconstructable
    for WeightedKey<SK>
{
    type SecretSharingConfig = WeightedConfig;

    fn reconstruct(sc: &Self::SecretSharingConfig, shares: &Vec<(Player, Self::Share)>) -> Self {
        let mut flattened_shares = Vec::with_capacity(sc.get_total_weight());

        // println!();
        for (player, sub_shares) in shares {
            // println!(
            //     "Flattening {} share(s) for player {player}",
            //     sub_shares.len()
            // );
            for (pos, share) in (*sub_shares).iter().enumerate() {
                let virtual_player = sc.get_virtual_player(player, pos);

                // println!(
                //     " + Adding share {pos} as virtual player {virtual_player}: {:?}",
                //     share
                // );
                // TODO(Performance): Avoiding the cloning here might be nice
                flattened_shares.push((virtual_player, share.clone()));
            }
        }

        WeightedKey {
            key: SK::reconstruct(sc.get_threshold_config(), &flattened_shares),
        }
    }
}

impl<T: Transcript> ValidCryptoMaterial for WeightedTranscript<T> {
    fn to_bytes(&self) -> Vec<u8> {
        self.trx.to_bytes()
    }
}

impl<T: Transcript> TryFrom<&[u8]> for WeightedTranscript<T> {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        T::try_from(bytes).map(|trx| Self { trx })
    }
}

impl<T: Transcript> WeightedTranscript<T> {
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

impl<T: Transcript<SecretSharingConfig = ThresholdConfig>> Transcript for WeightedTranscript<T> {
    type SecretSharingConfig = WeightedConfig;
    type PvssPublicParameters = T::PvssPublicParameters;

    /// In a weighted PVSS, an SK share is represented as a vector of SK shares in the unweighted
    /// PVSS, whose size is proportional to the weight of the owning player.
    type DealtSecretKeyShare = Vec<T::DealtSecretKeyShare>;
    type DealtPubKeyShare = Vec<T::DealtPubKeyShare>;
    type DealtSecretKey = WeightedKey<T::DealtSecretKey>;
    type DealtPubKey = WeightedKey<T::DealtPubKey>;
    type InputSecret = T::InputSecret;
    type EncryptPubKey = T::EncryptPubKey;
    type DecryptPrivKey = T::DecryptPrivKey;

    fn scheme_name() -> String {
        format!("weighted_{}", T::scheme_name())
    }

    fn deal<R: RngCore + CryptoRng>(
        sc: &Self::SecretSharingConfig,
        pp: &Self::PvssPublicParameters,
        eks: &Vec<Self::EncryptPubKey>,
        s: &Self::InputSecret,
        rng: &mut R,
    ) -> Self {
        // TODO(Security): This EK duplication allows an adversary to decrypt share_{i_j} / share_{i_k} for any $j$th and $k$th share of a validator $i$. Prove that security holds nonetheless or remove this.
        let duplicated_eks = WeightedTranscript::<T>::to_weighted_encryption_keys(sc, eks);

        WeightedTranscript {
            trx: T::deal(sc.get_threshold_config(), pp, &duplicated_eks, s, rng),
        }
    }

    fn verify(
        &self,
        sc: &Self::SecretSharingConfig,
        pp: &Self::PvssPublicParameters,
        eks: &Vec<Self::EncryptPubKey>,
    ) -> anyhow::Result<()> {
        let duplicated_eks = WeightedTranscript::<T>::to_weighted_encryption_keys(sc, eks);

        T::verify(
            &self.trx,
            sc.get_threshold_config(),
            pp,
            &duplicated_eks,
        )
    }

    fn aggregate_with(&mut self, sc: &Self::SecretSharingConfig, other: &Self) {
        T::aggregate_with(&mut self.trx, sc.get_threshold_config(), &other.trx)
    }

    fn get_dealt_public_key(&self) -> Self::DealtPubKey {
        WeightedKey {
            key: T::get_dealt_public_key(&self.trx),
        }
    }

    fn decrypt_own_share(
        &self,
        sc: &Self::SecretSharingConfig,
        player: &Player, // TODO: could make Player keep track of its weight and avoid passing `Self::SecretSharingConfig`?
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
        WeightedTranscript {
            trx: T::generate(sc.get_threshold_config(), rng),
        }
    }
}
