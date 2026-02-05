// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! # Traits for authenticated PVSS transcripts
//!
//! ## `InputSecret`, `DealtSecretKey` and `DealtPublicKey`
//!
//! The PVSS dealer picks a uniform *input secret* (`InputSecret`), inputs it into the PVSS dealing
//! algorithm, which deals a *dealt secret key* (`DealtSecretKey`) such that any $t$ or more subset
//! of the $n$ players can reconstruct this *dealt secret key*. Furthermore, the dealing algorithm
//! outputs to every player the *dealt public key* (`DealtPublicKey`) associated with this secret-shared
//! *dealt secret key*!
//!
//! In some PVSS protocols, the *dealt secret key* (e.g., $h_1^a\in G_1$) is a one-way function of the
//! *input secret* $a\in F$. As a result, such protocols only allows for the reconstruction of the
//! *dealt secret*, while the *input secret* cannot be reconstructed efficiently (in polynomial time).
//!
//! ## `EncryptPubKey` and `DecryptPrivKey` traits
//!
//! In a PVSS protocol, the PVSS transcript typically *encrypts* for each player their *dealt secret
//! keys share*. As a result, each player must pick an encryption key-pair: a private *decryption
//! key* (`DecryptPrivKey`) and its associated public *encryption key* (`EncryptPubKey`).
//! The dealer is assumed to have received all player's public encryption keys. This way, the dealer
//! can encrypt the shares for each player in the transcript.
//!
//! ## `DealtSecretShare` and `DealtPubKeyShare`
//!
//! The dealing algorithm outputs a *transcript* which encrypts, for each player $i$, its *share* of
//! the *dealt secret key*. We refer to this as a *dealt secret key share* (`DealtSecretKeyShare`) for
//! player $i$. Furthermore, the transcript also exposes an associated *dealt public key share*
//! (`DealtPubKeyShare`) for each *dealt secret key share*, which will be useful for efficiently
//! implementing threshold verifiable random functions.
//!
//! ## `SigningSecretKey` and `SigningPubKey`
//!
//! When using the PVSS protocol to build a $t$-out-of-$n$ distributed key generation (DKG) protocol,
//! it is necessary for each DKG player to sign their PVSS transcript so as to authenticate that
//! they contributed to the final DKG secret.
//!
//! To prevent replay of signed PVSS transcripts inside higher-level protocols, the PVSS dealer can
//! include some auxiliary data to compute the signature over too.
//!
//! ## A note on `aptos-crypto` traits
//!
//! We do not implement the `PublicKey` and `PrivateKey` traits from `aptos-crypto` for our PVSS
//! `DealtSecretKey[Share]` and `DealtPublicKey[Share]` structs because those traits (wrongly) assume
//! that one can always derive a public key from a secret key, which in our PVSS construction's case
//! does not hold.

use crate::pvss::{
    traits::{Convert, HasEncryptionPublicParams},
    Player,
};
use anyhow::bail;
use aptos_crypto::{
    arkworks::shamir::Reconstructable, SigningKey, TSecretSharingConfig, Uniform,
    ValidCryptoMaterial, VerifyingKey,
};
use num_traits::Zero;
use serde::{de::DeserializeOwned, Serialize};
use std::{fmt::Debug, ops::AddAssign};

// TODO: get rid of all the copy-paste here
pub trait Subtranscript: Debug + ValidCryptoMaterial + Clone + PartialEq + Eq {
    type PublicParameters: HasEncryptionPublicParams
        + WithMaxNumShares
        + Default
        + ValidCryptoMaterial
        + DeserializeOwned
        + Serialize
        + Debug
        + PartialEq
        + Eq;
    type SecretSharingConfig: TSecretSharingConfig
        + DeserializeOwned
        + Serialize
        + Debug
        + PartialEq
        + Eq;
    type DealtPubKeyShare: Debug + PartialEq + Clone;
    type DealtSecretKeyShare: PartialEq + Clone;
    type DealtPubKey: Serialize; // So it can get signed
    type DealtSecretKey: PartialEq
        + Reconstructable<Self::SecretSharingConfig, ShareValue = Self::DealtSecretKeyShare>;
    type EncryptPubKey: Debug
        + Clone
        + ValidCryptoMaterial
        + DeserializeOwned
        + Serialize
        + PartialEq
        + Eq;
    type DecryptPrivKey: Uniform
        + Convert<
            Self::EncryptPubKey,
            <Self::PublicParameters as HasEncryptionPublicParams>::EncryptionPublicParameters,
        >;

    /// Returns the dealt pubkey shore of `player`.
    fn get_public_key_share(
        &self,
        sc: &Self::SecretSharingConfig,
        player: &Player,
    ) -> Self::DealtPubKeyShare;

    /// Given a valid transcript, returns the `DealtPublicKey` of that transcript: i.e., the public
    /// key associated with the secret key dealt in the transcript.
    fn get_dealt_public_key(&self) -> Self::DealtPubKey;

    /// Given a valid transcript, returns the decrypted `DealtSecretShare` for the player with ID
    /// `player_id`.
    fn decrypt_own_share(
        &self,
        sc: &Self::SecretSharingConfig,
        player: &Player,
        dk: &Self::DecryptPrivKey,
        pp: &Self::PublicParameters,
    ) -> (Self::DealtSecretKeyShare, Self::DealtPubKeyShare);
}

/// A trait for a PVSS protocol transcript. This trait allows both for:
///
/// 1. Normal/unweighted $t$-out-of-$n$ PVSS protocols where any $t$ players (or more) can
///    reconstruct the secret (but no fewer can)
/// 2. Weighted $w$-out-of-$W$ PVSS protocols where any players with combined weight $\ge w$ can
///    reconstruct the secret (but players with combined weight $< w$ cannot)
pub trait Transcript: Debug + ValidCryptoMaterial + Clone + PartialEq + Eq {
    type SigningSecretKey: Uniform + SigningKey<VerifyingKeyMaterial = Self::SigningPubKey>;
    type SigningPubKey: VerifyingKey<SigningKeyMaterial = Self::SigningSecretKey>;

    type DealtSecretKey: PartialEq
        + Reconstructable<Self::SecretSharingConfig, ShareValue = Self::DealtSecretKeyShare>;

    type InputSecret: Uniform
        + Zero
        + for<'a> AddAssign<&'a Self::InputSecret>
        + Convert<Self::DealtSecretKey, Self::PublicParameters>
        + Convert<Self::DealtPubKey, Self::PublicParameters>;

    type PublicParameters: HasEncryptionPublicParams
        + WithMaxNumShares
        + Default
        + ValidCryptoMaterial
        + DeserializeOwned
        + Serialize
        + Debug
        + PartialEq
        + Eq;
    type SecretSharingConfig: TSecretSharingConfig
        + DeserializeOwned
        + Serialize
        + Debug
        + PartialEq
        + Eq;
    type DealtPubKeyShare: Debug + PartialEq + Clone;
    type DealtSecretKeyShare: PartialEq + Clone;
    type DealtPubKey: Serialize; // So it can get signed
    type EncryptPubKey: Debug
        + Clone
        + ValidCryptoMaterial
        + DeserializeOwned
        + Serialize
        + PartialEq
        + Eq;
    type DecryptPrivKey: Uniform
        + Convert<
            Self::EncryptPubKey,
            <Self::PublicParameters as HasEncryptionPublicParams>::EncryptionPublicParameters,
        >;

    /// Domain-separator tag (DST) for the Fiat-Shamir hashing used to derive randomness from the transcript.
    fn dst() -> Vec<u8>;

    /// Return a developer-friendly name of the PVSS scheme (e.g., "vanilla_scrape") that can be
    /// used in, say, criterion benchmark names.
    fn scheme_name() -> String;

    /// Deals the *input secret* $s$ by creating a PVSS transcript which encrypts shares of $s$ for
    /// all PVSS players. Signs the transcript with `ssk`.
    ///
    /// The dealer will sign the transcript (or part of it; typically just a commitment to the dealt
    /// secret) together with his player ID in `dealer` and the auxiliary data in `aux` (which might
    /// be needed for the security of higher-level protocols; e.g., replay protection).
    fn deal<A: Serialize + Clone, R: rand_core::RngCore + rand_core::CryptoRng>(
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        ssk: &Self::SigningSecretKey,
        spk: &Self::SigningPubKey,
        eks: &[Self::EncryptPubKey],
        s: &Self::InputSecret,
        sid: &A,
        dealer: &Player,
        rng: &mut R,
    ) -> Self;

    /// Verifies the validity of the PVSS transcript: i.e., the transcripts correctly encrypts shares
    /// of an `InputSecret` $s$ which has been $(t, n)$ secret-shared such that only $\ge t$ players
    /// can reconstruct it as a `DealtSecret`.
    ///
    /// Additionally, verifies that the transcript was indeed aggregated from a set of players
    /// identified by the public keys in `spks`, by verifying each player $i$'s signature on the
    /// transcript and on `aux[i]`.
    // fn verify<A: Serialize + Clone>(
    //     &self,
    //     sc: &Self::SecretSharingConfig,
    //     pp: &Self::PublicParameters,
    //     spks: &[Self::SigningPubKey],
    //     eks: &[Self::EncryptPubKey],
    //     sids: &[A],
    // ) -> anyhow::Result<()>;

    /// Returns the set of player IDs who have contributed to this transcript.
    /// In other words, the transcript could have been dealt by one player, in which case
    /// the set is of size 1, or the transcript could have been obtained by aggregating `n`
    /// other transcripts, in which case the set will be of size `n`.
    fn get_dealers(&self) -> Vec<Player>;

    /// Generates a random looking transcript (but not a valid one).
    /// Useful for testing and benchmarking.
    fn generate<R>(
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        rng: &mut R,
    ) -> Self
    where
        R: rand_core::RngCore + rand_core::CryptoRng;

    /// Returns the dealt pubkey shore of `player`.
    fn get_public_key_share(
        &self,
        sc: &Self::SecretSharingConfig,
        player: &Player,
    ) -> Self::DealtPubKeyShare;

    /// Given a valid transcript, returns the `DealtPublicKey` of that transcript: i.e., the public
    /// key associated with the secret key dealt in the transcript.
    fn get_dealt_public_key(&self) -> Self::DealtPubKey;

    /// Given a valid transcript, returns the decrypted `DealtSecretShare` for the player with ID
    /// `player_id`.
    fn decrypt_own_share(
        &self,
        sc: &Self::SecretSharingConfig,
        player: &Player,
        dk: &Self::DecryptPrivKey,
        pp: &Self::PublicParameters,
    ) -> (Self::DealtSecretKeyShare, Self::DealtPubKeyShare);
}

/// Trait for types that can be aggregated.
/// Supports efficient intermediate aggregation in a separate associated type.
pub trait Aggregatable: Sized {
    /// Config type for secret-sharing aggregation
    type SecretSharingConfig;

    /// The intermediate/projective aggregation type
    type Aggregated: Aggregated<Self>;

    /// Convert into the intermediate/projective type for aggregation
    fn to_aggregated(&self) -> Self::Aggregated;

    /// Aggregate a vector of items into a single affine value
    fn aggregate(sc: &Self::SecretSharingConfig, items: Vec<Self>) -> anyhow::Result<Self> {
        if items.is_empty() {
            bail!("Cannot aggregate empty vector");
        }

        // Convert the first item into the intermediate type
        let mut iter = items.into_iter();
        let first = iter.next().unwrap();
        let mut acc = first.to_aggregated();

        // Aggregate the rest
        for item in iter {
            acc.aggregate_with(&sc, &item)?;
        }

        // Convert back to affine/final type
        Ok(acc.normalize())
    }
}

/// Trait for the intermediate aggregation type
pub trait Aggregated<T: Aggregatable>: Sized {
    /// Aggregate another item into this intermediate aggregation
    fn aggregate_with(&mut self, sc: &T::SecretSharingConfig, other: &T) -> anyhow::Result<()>;

    /// Convert the intermediate/projective aggregation back into the final affine type
    fn normalize(self) -> T;
}

pub trait AggregatableTranscript:
    Transcript + Aggregatable<SecretSharingConfig = <Self as Transcript>::SecretSharingConfig>
{
    // The signature here is slightly different from `NonAggregatableTranscript`, because our aggregatable PVSSs needs all of the session ids
    fn verify<A: Serialize + Clone>(
        &self,
        sc: &<Self as Transcript>::SecretSharingConfig,
        pp: &Self::PublicParameters,
        spks: &[Self::SigningPubKey],
        eks: &[Self::EncryptPubKey],
        sids: &[A],
    ) -> anyhow::Result<()>;
}

pub trait HasAggregatableSubtranscript: Transcript {
    type Subtranscript: Aggregatable
        + Subtranscript<
            PublicParameters = Self::PublicParameters,
            SecretSharingConfig = Self::SecretSharingConfig,
            DealtPubKeyShare = Self::DealtPubKeyShare,
            DealtSecretKeyShare = Self::DealtSecretKeyShare,
            DealtSecretKey = Self::DealtSecretKey,
            DealtPubKey = Self::DealtPubKey,
            EncryptPubKey = Self::EncryptPubKey,
            DecryptPrivKey = Self::DecryptPrivKey,
        >;

    fn get_subtranscript(&self) -> Self::Subtranscript;

    fn verify<A: Serialize + Clone>(
        &self,
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        spks: &[Self::SigningPubKey],
        eks: &[Self::EncryptPubKey],
        sid: &A, // Note the different function signature heres
    ) -> anyhow::Result<()>;
}

/// This traits defines testing-only and benchmarking-only interfaces.
pub trait MalleableTranscript: Transcript {
    /// This is useful for generating many PVSS transcripts from different dealers from a single
    /// PVSS transcript by recomputing its signature. It is used to deal quickly when benchmarking
    /// aggregated PVSS transcript verification
    fn maul_signature<A: Serialize + Clone>(
        &mut self,
        ssk: &Self::SigningSecretKey,
        session_id: &A,
        dealer: &Player,
    );
}

/// This is needed instead of Default because `max_n` influences the public parameters of the DeKARTv2 range proof, and hence the public parameters of `chunky`
pub trait WithMaxNumShares {
    fn with_max_num_shares(n: u32) -> Self;

    fn with_max_num_shares_and_bit_size(n: u32, ell: u8) -> Self;

    /// This is a modified function which might create public parameters that are fairly nonsensical, but which are sufficient for `generate()`
    fn with_max_num_shares_for_generate(n: u32) -> Self;
}
