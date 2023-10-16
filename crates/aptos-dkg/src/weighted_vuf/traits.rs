//! Notes: Unlike PVSS, we do NOT want to use a generic unweighted-to-weighted VUF transformation,
//! since we have a more optimized transformation for some VRF schemes (e.g., BLS and [GJM+21e]).
//!
//! As a result, we only define weighted VUF traits here.

use crate::pvss::{Player, WeightedConfig};
use std::fmt::Debug;

/// Weighted (not-verifiable) unpredictable function (WUF) traits.
pub trait WeightedUF {
    type PublicParameters;
    type SecretKey;
    type PubKeyShare: Clone;
    type SecretKeyShare;

    type Delta: Clone;

    type AugmentedPubKeyShare: Clone + Debug + Eq;
    type AugmentedSecretKeyShare;

    type ProofShare;
    type Proof;

    /// NOTE: Not unsafe to have Debug here, since if an evaluation was aggregated, more than 33% of
    /// the stake must've contributed to create it.
    type Evaluation: Debug + Eq;

    fn augment_key_pair<R: rand_core::RngCore + rand_core::CryptoRng>(
        pp: &Self::PublicParameters,
        sk: Self::SecretKeyShare,
        pk: Self::PubKeyShare,
        rng: &mut R,
    ) -> (Self::AugmentedSecretKeyShare, Self::AugmentedPubKeyShare);

    fn get_public_delta(pk: &Self::AugmentedPubKeyShare) -> &Self::Delta;

    fn augment_pubkey(
        pp: &Self::PublicParameters,
        pk: Self::PubKeyShare,
        delta: Self::Delta,
    ) -> anyhow::Result<Self::AugmentedPubKeyShare>;

    fn create_share(ask: &Self::AugmentedSecretKeyShare, msg: &[u8]) -> Self::ProofShare;

    fn verify_share(
        pp: &Self::PublicParameters,
        apk: &Self::AugmentedPubKeyShare,
        msg: &[u8],
        proof: &Self::ProofShare,
    ) -> anyhow::Result<()>;

    fn aggregate_shares(
        wc: &WeightedConfig,
        apks_and_proofs: &[(Player, Self::AugmentedPubKeyShare, Self::ProofShare)],
    ) -> Self::Proof;

    /// Used for testing only.
    fn eval(sk: &Self::SecretKey, msg: &[u8]) -> Self::Evaluation;

    fn derive_eval(msg: &[u8], proof: &Self::Proof) -> Self::Evaluation;
}

/// Extends the `WUF` trait with publicy-verifiability of the aggregated proof and evaluation.
pub trait VerifiableWUF {
    type SecretKey;
    type PubKey;
    type Proof;
    type Evaluation;

    /// Used for testing only.
    fn create_proof(sk: &Self::SecretKey, msg: &[u8]) -> Self::Proof;

    fn verify_eval(
        pk: &Self::PubKey,
        msg: &[u8],
        proof: &Self::Proof,
        eval: &Self::Evaluation,
    ) -> anyhow::Result<()>;
}
