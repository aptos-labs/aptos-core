// Copyright © Aptos Foundation

use std::fmt::Debug;
use crate::{Uniform, ValidCryptoMaterial};

pub trait PVSS: ValidCryptoMaterial + Clone {
    type SecretSharingConfig: SecretSharingConfig;

    type PvssPublicParameters: HasEncryptionPublicParams + Default;

    type DealtSecretKeyShare: PartialEq + Clone;
    type DealtPubKeyShare;
    type DealtSecretKey: PartialEq
    + Debug
    + IsSecretShareable<Share = Self::DealtSecretKeyShare>
    + Reconstructable<SecretSharingConfig = Self::SecretSharingConfig>;
    type DealtPubKey;

    type InputSecret: Uniform
    + Convert<Self::DealtSecretKey, Self::PvssPublicParameters>
    + Convert<Self::DealtPubKey, Self::PvssPublicParameters>;

    type EncryptPubKey: Clone;
    type DecryptPrivKey: Uniform
    + Convert<
        Self::EncryptPubKey,
        <Self::PvssPublicParameters as HasEncryptionPublicParams>::EncryptionPublicParameters,
    >;

    /// Deals the *input secret* $s$ by creating a PVSS transcript which encrypts shares of $s$ for
    /// all PVSS players.
    fn deal<R: rand_core::RngCore + rand_core::CryptoRng>(
        sc: &Self::SecretSharingConfig,
        pp: &Self::PvssPublicParameters,
        eks: &Vec<Self::EncryptPubKey>,
        s: Self::InputSecret,
        dst: &'static [u8],
        rng: &mut R,
    ) -> Self;

    /// Verifies the validity of the PVSS transcript: i.e., the transcripts correctly encrypts shares
    /// of an `InputSecret` $s$ which has been $(t, n)$ secret-shared such that only $\ge t$ players
    /// can reconstruct it as a `DealtSecret`.
    /// TODO: update comments
    fn verify(
        &self,
        sc: &Self::SecretSharingConfig,
        pp: &Self::PvssPublicParameters,
        eks: &Vec<Self::EncryptPubKey>,
        dst: &'static [u8],
    ) -> bool;

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
        player_id: &Player,
        dk: &Self::DecryptPrivKey,
    ) -> (Self::DealtSecretKeyShare, Self::DealtPubKeyShare);

    /// Generates a random looking transcript (but not a valid one).
    /// Useful for testing and benchmarking.
    fn generate<R>(sc: &Self::SecretSharingConfig, rng: &mut R) -> Self
        where
            R: rand_core::RngCore + rand_core::CryptoRng;
}
