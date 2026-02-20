// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use crate::errors::MissingEvalProofError;
use anyhow::Result;
use aptos_crypto::player::Player;
use aptos_dkg::pvss::traits::TranscriptCore;
use ark_std::rand::{CryptoRng, RngCore};
use serde::{de::DeserializeOwned, Serialize};
use std::hash::Hash;

pub trait BatchThresholdEncryption {
    type ThresholdConfig: aptos_crypto::TSecretSharingConfig;
    type SubTranscript: TranscriptCore;

    /// An encryption key for the scheme. Allows for generating ciphertexts.
    type EncryptionKey;

    /// A digest key for the scheme. Allows for generating digests given a list of ciphertexts.
    /// Internally, this is a modified KZG setup.
    type DigestKey: Serialize + DeserializeOwned;

    /// A ciphertext for the scheme.
    type Ciphertext: Serialize
        + DeserializeOwned
        + Eq
        + PartialEq
        + Serialize
        + Hash
        + Sized
        + Send
        + Sync;

    /// A ciphertext may be "prepared" once a digest and an eval proof corresponding to the
    /// ciphertext has been computed. Decrypting the resulting `PreparedCiphertext`
    type PreparedCiphertext: Serialize
        + DeserializeOwned
        + Eq
        + PartialEq
        + Serialize
        + Sized
        + Send
        + Sync;

    /// The round number used when generating a digest. For security to hold, validators must only
    /// generate a single decryption key corresponding to a round number.
    type Round;

    /// The succinct commitment to the set of ciphertexts.
    type Digest: Sized + Send + Sync;

    /// A promise representing an uncomputed set of eval proofs.
    type EvalProofsPromise;

    /// The eval proofs required for decryption.
    type EvalProofs: Sized + Send + Sync;

    /// An individual eval proof.
    type EvalProof;

    /// A share of the master secret key, which allows for deriving
    /// decryption key shares.
    type MasterSecretKeyShare;

    /// Used to verify whether a specific player's decryption key share is valid w.r.t. a specific
    /// digest.
    type VerificationKey: VerificationKey;

    /// A share of the decryption key.
    type DecryptionKeyShare: DecryptionKeyShare;

    /// A decryption key that has been reconstructed by a threshold of decryption key shares.
    type DecryptionKey: Send + Sized + Sync;
    type Id: PartialEq + Eq;

    /// Generates an (insecure) setup for the batch threshold encryption scheme. Consists of
    /// an [`EncryptionKey`] which can be used to encrypt messages and to compute a digest from a list
    /// of ciphertexts, along with a vector of shares of type [`MasterSecretKeyShare`], which share
    /// the secret key according to the [`ThresholdConfig`] given as input. In production,
    fn setup(
        digest_key: &Self::DigestKey,
        pvss_public_params: &<Self::SubTranscript as TranscriptCore>::PublicParameters,
        subtranscript: &Self::SubTranscript,
        threshold_config: &Self::ThresholdConfig,
        current_player: Player,
        sk_share_decryption_key: &<Self::SubTranscript as TranscriptCore>::DecryptPrivKey,
    ) -> Result<(
        Self::EncryptionKey,
        Vec<Self::VerificationKey>,
        Self::MasterSecretKeyShare,
    )>;

    fn extract_encryption_key(
        digest_key: &Self::DigestKey,
        subtranscript: &Self::SubTranscript,
    ) -> Result<Self::EncryptionKey>;

    /// Generates an (insecure) setup for the batch threshold encryption scheme. In production,
    /// a DKG will be used to produce all parts of this setup except for [`DigestKey`], which will
    /// be produced using a single-time trusted setup ceremony.
    fn setup_for_testing(
        seed: u64,
        max_batch_size: usize,
        number_of_rounds: usize,
        threshold_config: &Self::ThresholdConfig,
    ) -> Result<(
        Self::EncryptionKey,
        Self::DigestKey,
        Vec<Self::VerificationKey>,
        Vec<Self::MasterSecretKeyShare>,
    )>;

    /// Encrypt a plaintext with respect to any arbitrary associated data. This associated data is
    /// "bound" to the resulting CT, such that it will only verify with respect to the same
    /// associated data.
    fn encrypt<R: CryptoRng + RngCore>(
        ek: &Self::EncryptionKey,
        rng: &mut R,
        msg: &impl Plaintext,
        associated_data: &impl AssociatedData,
    ) -> Result<Self::Ciphertext>;

    /// Derive a digest from a [`DigestKey`] and a slice of ciphertexts.
    fn digest(
        digest_key: &Self::DigestKey,
        cts: &[Self::Ciphertext],
        round: Self::Round,
    ) -> Result<(Self::Digest, Self::EvalProofsPromise)>;

    /// Validators *must* verify each ciphertext before approving it to be decrypted, in order to
    /// prevent malleability attacks. Verification happens w.r.t. some associated data that was
    /// passed into the encrypt fn.
    fn verify_ct(ct: &Self::Ciphertext, associated_data: &impl AssociatedData) -> Result<()>;

    /// Although I'd like to expose as little of the identities as possible, Daniel told me that
    /// knowing the ID of a ciphertext will potentially help with deduplication.
    fn ct_id(ct: &Self::Ciphertext) -> Self::Id;

    /// Compute KZG eval proofs. This will be the most expensive operation in the scheme.
    fn eval_proofs_compute_all(
        proofs: &Self::EvalProofsPromise,
        digest_key: &Self::DigestKey,
    ) -> Self::EvalProofs;

    /// Compute KZG eval proofs. This will be the most expensive operation in the scheme. This
    /// version uses a different (slower for our parameter regime) multi-point-eval algorithm,
    /// from von zur Gathen and Gerhardt. Currently for benchmarking only, not for production use.
    fn eval_proofs_compute_all_vzgg_multi_point_eval(
        proofs: &Self::EvalProofsPromise,
        digest_key: &Self::DigestKey,
    ) -> Self::EvalProofs;

    fn eval_proof_for_ct(
        proofs: &Self::EvalProofs,
        ct: &Self::Ciphertext,
    ) -> Option<Self::EvalProof>;

    /// Derive a decryption key share given a [`Digest`] and a round number, whose
    /// corresponding reconstructed decryption key will be able to decrypt any ciphertext encrypted
    /// to that round number and committed to by that digest.
    fn derive_decryption_key_share(
        msk_share: &Self::MasterSecretKeyShare,
        digest: &Self::Digest,
    ) -> Result<Self::DecryptionKeyShare>;

    /// With respect to a verification key and a digest, verify that a decryption key share was
    /// honestly derived.
    fn verify_decryption_key_share(
        verification_key: &Self::VerificationKey,
        digest: &Self::Digest,
        decryption_key_share: &Self::DecryptionKeyShare,
    ) -> Result<()>;

    /// Reconstruct a decryption key from a set of [`DecryptionKeyShare`]s assuming the set of
    /// shares surpasses the threshold.
    fn reconstruct_decryption_key(
        shares: &[Self::DecryptionKeyShare],
        config: &Self::ThresholdConfig,
    ) -> Result<Self::DecryptionKey>;

    /// With respect to the scheme's encryption key and a digest, verify that the decryption key
    /// was honestly reconstructed from honestly-derived decryption key shares.
    fn verify_decryption_key(
        encryption_key: &Self::EncryptionKey,
        digest: &Self::Digest,
        decryption_key: &Self::DecryptionKey,
    ) -> Result<()>;

    /// Take a ciphertext, digest, and eval proofs as input and output a prepared ciphertext. This
    /// can be done before reconstructing the decryption key.
    fn prepare_ct(
        ct: &Self::Ciphertext,
        digest: &Self::Digest,
        eval_proofs: &Self::EvalProofs,
    ) -> std::result::Result<Self::PreparedCiphertext, MissingEvalProofError>;

    /// Decrypt a prepared ciphertext using the reconstructed decryption key.
    fn decrypt<P: Plaintext>(
        decryption_key: &Self::DecryptionKey,
        ct: &Self::PreparedCiphertext,
    ) -> Result<P>;

    /// Convenience method which performs both prepare and decrypt steps. As performing the steps
    /// individually results in a lower critical-path latency, since `prepare` can be done before
    /// reconstructing the decryption key, this should only be used during state sync for verifying
    /// correct decryption.
    fn decrypt_slow<P: Plaintext>(
        decryption_key: &Self::DecryptionKey,
        ct: &Self::Ciphertext,
        digest: &Self::Digest,
        eval_proof: &Self::EvalProof,
    ) -> Result<P>;
}

/// An element of the plaintext space. Does not depend on the specific scheme; any struct that is
/// serializeable should allow for being used as a plaintext.
pub trait Plaintext: Serialize + DeserializeOwned + Send + Sync {}

pub trait AssociatedData:
    Clone + Serialize + DeserializeOwned + Eq + PartialEq + Serialize + Hash
{
}

impl Plaintext for String {}
impl AssociatedData for String {}

pub trait VerificationKey: Serialize + DeserializeOwned {
    fn player(&self) -> Player;
}

pub trait DecryptionKeyShare: Serialize + DeserializeOwned {
    fn player(&self) -> Player;
}
