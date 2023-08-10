// Copyright © Aptos Foundation

//! # Traits for implementing PVSS schemes
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
//! ## A note on `aptos-crypto` traits
//!
//! We do not implement the `PublicKey` and `PrivateKey` traits from `aptos-crypto` for our PVSS
//! `DealtSecretKey[Share]` and `DealtPublicKey[Share]` structs because those traits (wrongly) assume
//! that one can always derive a public key from a secret key, which in our PVSS construction's case
//! does not hold.

use crate::pvss::traits::{
    Convert, HasEncryptionPublicParams, IsSecretShareable, Reconstructable, SecretSharingConfig,
};
use crate::pvss::Player;
use aptos_crypto::{Uniform, ValidCryptoMaterial};
use std::fmt::Debug;

/// A trait for a PVSS protocol. This trait allows both for:
///
/// 1. Normal/unweighted $t$-out-of-$n$ PVSS protocols where any $t$ players (or more) can
///    reconstruct the secret (but no fewer can)
/// 2. Weighted $w$-out-of-$W$ PVSS protocols where any players with combined weight $\ge w$ can
///    reconstruct the secret (but players with combined weight $< w$ cannot)
pub trait Transcript: Debug + ValidCryptoMaterial + Clone + PartialEq + Eq {
    type SecretSharingConfig: SecretSharingConfig;

    type PvssPublicParameters: HasEncryptionPublicParams + Default + ValidCryptoMaterial;

    type DealtSecretKeyShare: PartialEq + Clone;
    type DealtPubKeyShare;
    type DealtSecretKey: Debug
        + PartialEq
        + IsSecretShareable<Share = Self::DealtSecretKeyShare>
        + Reconstructable<SecretSharingConfig = Self::SecretSharingConfig>;
    type DealtPubKey;

    type InputSecret: Uniform
        + Convert<Self::DealtSecretKey, Self::PvssPublicParameters>
        + Convert<Self::DealtPubKey, Self::PvssPublicParameters>;

    type EncryptPubKey: Debug + Clone + ValidCryptoMaterial;
    type DecryptPrivKey: Debug
        + Uniform
        + Convert<
            Self::EncryptPubKey,
            <Self::PvssPublicParameters as HasEncryptionPublicParams>::EncryptionPublicParameters,
        >;

    /// Return a developer-friendly name of the PVSS scheme (e.g., "vanilla_scrape") that can be
    /// used in, say, criterion benchmark names.
    fn scheme_name() -> String;

    /// Deals the *input secret* $s$ by creating a PVSS transcript which encrypts shares of $s$ for
    /// all PVSS players.
    fn deal<R: rand_core::RngCore + rand_core::CryptoRng>(
        sc: &Self::SecretSharingConfig,
        pp: &Self::PvssPublicParameters,
        eks: &Vec<Self::EncryptPubKey>,
        s: &Self::InputSecret,
        dst: &'static [u8],
        rng: &mut R,
    ) -> Self;

    /// Verifies the validity of the PVSS transcript: i.e., the transcripts correctly encrypts shares
    /// of an `InputSecret` $s$ which has been $(t, n)$ secret-shared such that only $\ge t$ players
    /// can reconstruct it as a `DealtSecret`.
    ///
    /// TODO(Clean): Change result type to anyhow::Result to more easily indicate to the caller what the problem was.
    fn verify(
        &self,
        sc: &Self::SecretSharingConfig,
        pp: &Self::PvssPublicParameters,
        eks: &Vec<Self::EncryptPubKey>,
        dst: &'static [u8],
    ) -> anyhow::Result<()>;

    /// Aggregates two transcripts.
    fn aggregate_with(&mut self, sc: &Self::SecretSharingConfig, other: &Self);

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
    ) -> (Self::DealtSecretKeyShare, Self::DealtPubKeyShare);

    /// Generates a random looking transcript (but not a valid one).
    /// Useful for testing and benchmarking.
    fn generate<R>(sc: &Self::SecretSharingConfig, rng: &mut R) -> Self
    where
        R: rand_core::RngCore + rand_core::CryptoRng;
}
