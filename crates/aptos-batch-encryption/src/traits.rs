
// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::{hash::Hash, sync::mpsc::RecvTimeoutError};

use rand_core::{CryptoRng, RngCore};
use rayon::ThreadPool;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use anyhow::Result;

use crate::shared::algebra::shamir::ThresholdConfig;


pub trait BatchThresholdEncryption {

    /// An encryption key for the scheme. Allows for generating ciphertexts. If we want to actually
    /// deploy this scheme, the functionality here will have to be implemented in the SDK.
    type EncryptionKey;

    /// A digest key for the scheme. Allows for generating digests given a list of ciphertexts.
    /// Internally, this is a modified KZG setup.
    type DigestKey: Serialize + DeserializeOwned;

    /// A ciphertext for the scheme. Internally, this is encrypted w.r.t. an ID and a round number,
    /// but I think it makes sense not to expose the ID as part of the interface. (The round number
    /// must be exposed since it must be given as input to [`PublicKey::encrypt`], and must agree
    /// with the round number used when computing a decryption key.)
    type Ciphertext: Serialize + DeserializeOwned + Eq + PartialEq + Serialize + Hash;

    /// The round number used when generating a digest. For security to hold, validators must only
    /// generate a single decryption key corresponding to a round number.
    type Round;

    /// Internally, a KZG commitment to a set of IDs.
    type Digest;

    /// Auxiliary information needed for decryption. In the scheme we will implement,
    /// this consists of the KZG eval proofs.
    type EvalProofsPromise;

    type EvalProofs;

    /// A share of the master secret key, which allows for deriving
    /// decryption key shares.
    type MasterSecretKeyShare;

    type VerificationKey : VerificationKey;

    type DecryptionKeyShare : DecryptionKeyShare;

    /// A decryption key that has been reconstructed by a threshold of decryption key shares.
    type DecryptionKey;
    type Id : PartialEq + Eq;

    /// Generates an (insecure) setup for the batch threshold encryption scheme. Consists of
    /// a [`PublicKey`] which can be used to encrypt messages and to compute a digest from a list
    /// of ciphertexts, along with a vector of shares of type [`MasterSecretKeyShare`], which share
    /// the secret key according to the [`ThresholdConfig`] given as input. Eventually, this will
    /// need to be replaced by a DKG.
    fn setup_for_testing(
        seed: u64,
        max_batch_size: usize,
        number_of_rounds: usize,
        tc_happypath: &ThresholdConfig,
        tc_slowpath: &ThresholdConfig
    ) -> Result<(Self::EncryptionKey, Self::DigestKey, Vec<Self::VerificationKey>, Vec<Self::MasterSecretKeyShare>, Vec<Self::VerificationKey>, Vec<Self::MasterSecretKeyShare>)>;


    /// Encrypt a plaintext with respect to a specific round number.
    fn encrypt<R: CryptoRng + RngCore>(ek: &Self::EncryptionKey, rng: &mut R, msg: &impl Plaintext)
        -> Result<Self::Ciphertext>;


    /// Derive a digest from a [`DigestKey`] and a slice of ciphertexts.
    fn digest(digest_key: &Self::DigestKey, cts: &[Self::Ciphertext], round: Self::Round, pool: &ThreadPool)
        -> Result<(Self::Digest, Self::EvalProofsPromise)>;

    /// Validators *must* verify each ciphertext before approving it to be decrypted, in order to
    /// prevent malleability attacks.
    fn verify_ct(ct: &Self::Ciphertext) -> Result<()>;

    /// Although I'd like to expose as little of the identities as possible, Daniel told me that
    /// knowing the ID of a ciphertext will potentially help with deduplication.
    fn ct_id(ct: &Self::Ciphertext) -> Self::Id;

    /// Compute KZG eval proofs. This will be the most expensive operation in the scheme.
    fn eval_proofs_compute_all(proofs: &Self::EvalProofsPromise, digest_key: &Self::DigestKey, pool: &ThreadPool) -> Self::EvalProofs;

    /// Derive a decryption key share given a [`SuccinctDigest`] and a round number, whose
    /// corresponding reconstructed decryption key will be able to decrypt any ciphertext encrypted
    /// to that round number and committed to by that digest.
    fn derive_decryption_key_share(
        msk_share: &Self::MasterSecretKeyShare,
        digest: &Self::Digest,
        ) -> Result<Self::DecryptionKeyShare>;


    fn verify_decryption_key_share(
        verification_key: &Self::VerificationKey,
        digest: &Self::Digest,
        decryption_key_share: &Self::DecryptionKeyShare,
    ) -> Result<()>;


    /// Reconstruct a decryption key from a set of [`DecryptionKeyShare`]s assuming the set of
    /// shares surpasses the threshold.
    fn reconstruct_decryption_key(shares: &[Self::DecryptionKeyShare], config: &ThresholdConfig, pool: &ThreadPool)
        -> Result<Self::DecryptionKey>;

    // TODO: verify decryption key?

    /// Decrypt a set of ciphertext using a decryption key and advice.
    fn decrypt<P: Plaintext>(
        decryption_key: &Self::DecryptionKey,
        cts: &[Self::Ciphertext],
        aux_info: &Self::EvalProofs,
        pool: &ThreadPool
        ) -> Result<Vec<P>>;
}


/// An element of the plaintext space. Does not depend on the specific scheme; any struct that is
/// serializeable should allow for being used as a plaintext.
pub trait Plaintext: Serialize + DeserializeOwned + Send + Sync {}

impl Plaintext for String {}

impl Plaintext for Vec<u8> {}


#[derive(Debug, PartialEq, Eq, PartialOrd, Copy, Clone, Serialize, Deserialize)]
pub struct Player {
    id: usize,
}

impl Player {
    pub fn new(id: usize) -> Self { Self { id } }
    pub fn id(&self) -> usize { self.id }
}

pub trait VerificationKey: Serialize + DeserializeOwned {
    fn player(&self) -> Player;
}

pub trait DecryptionKeyShare: Serialize + DeserializeOwned {
    fn player(&self) -> Player;
}
