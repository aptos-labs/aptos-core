// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0


use std::sync::mpsc::RecvTimeoutError;

use rand::RngCore;
use rayon::ThreadPool;
use serde::Serialize;
use anyhow::Result;


pub trait BatchThresholdEncryption {

    /// An encryption key for the scheme. Allows for generating ciphertexts. If we want to actually
    /// deploy this scheme, the functionality here will have to be implemented in the SDK.
    type EncryptionKey;

    /// A digest key for the scheme. Allows for generating digests given a list
    /// of ciphertexts. Internally, this is a KZG setup.
    type DigestKey;

    /// A ciphertext for the scheme. Internally, this is encrypted w.r.t.
    /// an ID and a round number, but I think it makes sense not to expose
    /// the ID as part of the interface. (The round number must be exposed
    /// since it must be given as input to [`PublicKey::encrypt`], and must
    /// agree with the round number used when computing a decryption key.)
    type Ciphertext: Serialize + Clone;

    /// The unique round number used when encrypting and generating decryption
    /// keys. Properties are:
    /// 1. Validators must only generate a single decryption key for a single
    ///    round number, otherwise privacy will be broken
    /// 2. A decryption key given for round number `t` will only work for
    ///    ciphertexts generated w.r.t. the same round number `t`.
    type RoundNumber: Default;

    type RoundNumberRange;

    /// Internally, a KZG commitment to a set of IDs.
    type Digest;

    /// Auxiliary information needed for decryption. In the scheme we will implement,
    /// this consists of the
    type DecryptionAuxInfo;

    /// A share of the master secret key, which allows for deriving
    /// decryption key shares.
    type MasterSecretKeyShare;

    type DecryptionKeyShare: Default;

    /// A decryption key that has been reconstructed by a threshold of decryption key shares.
    type DecryptionKey: Serialize;
    type Id : PartialEq + Eq + Default;

    /// Generates an (insecure) setup for the batch threshold encryption scheme.
    /// Consists of a [`PublicKey`] which can be used to encrypt messages and to
    /// compute a digest from a list of ciphertexts, along with a vector of shares
    /// of type [`MasterSecretKeyShare`], which share the secret key according to
    /// the [`ThresholdConfig`] given as input. Eventually, this will need to be
    /// replaced by a DKG.
    fn setup(rng: &mut impl RngCore, max_batch_size: usize, tc: &ThresholdConfig)
        -> (Self::EncryptionKey, Self::DigestKey, Vec<Self::MasterSecretKeyShare>);

    /// Encrypt a plaintext with respect to a specific round number.
    fn encrypt(ek: &Self::EncryptionKey, msg: impl Plaintext, t: Self::RoundNumberRange)
        -> Self::Ciphertext;


    /// Derive a digest from a [`DigestKey`] and a slice of verified
    /// ciphertexts.
    fn digest(&self, cts: &[Self::Ciphertext], pool: &ThreadPool)
        -> Result<(Self::Digest, Self::DecryptionAuxInfo)>;

    /// Validators *must* verify each ciphertext before approving it to
    /// be decrypted, in order to prevent malleability attacks. I think
    /// it might be good to encode this in the typesystem via the following
    /// `verify` function, and then having decrypt require
    /// a vector of [`Ciphertext`]. But will need to discuss the
    /// implications of this design choice.
    fn verify_ct(unverified_ct: &Self::Ciphertext) -> Result<()>;

    /// Every ciphertext has a corresponding [`RoundNumber`], and all the ciphertexts in a digest
    /// must have matching round numbers.
    fn ct_round_number_range(ct: &Self::Ciphertext) -> Self::RoundNumberRange;
    /// Although I'd like to expose as little of the identities as possible, Daniel told me that
    /// knowing the ID of a ciphertext will potentially help with deduplication.
    fn ct_id(ct: &Self::Ciphertext) -> Self::Id;

    /// Compute KZG eval proofs. This will be the most expensive operation in the scheme.
    fn prepare_decryption_aux_info(aux: &mut Self::DecryptionAuxInfo, pool: &ThreadPool);


    /// Derive a decryption key share given a [`SuccinctDigest`] and a round number, whose
    /// corresponding reconstructed decryption key will be able to decrypt any ciphertext encrypted
    /// to that round number and committed to by that digest.
    fn derive_decryption_key_share(
        msk_share: &Self::MasterSecretKeyShare,
        config: &ThresholdConfig,
        digest: &Self::Digest,
        t: Self::RoundNumber
        ) -> Self::DecryptionKeyShare;


    /// Reconstruct a decryption key from a set of [`DecryptionKeyShare`]s assuming the set of
    /// shares surpasses the threshold.
    fn reconstruct_decryption_key(shares: &[Self::DecryptionKeyShare], config: &ThresholdConfig)
        -> Result<Self::DecryptionKey>;


    /// Decrypt a set of ciphertext using a decryption key and advice.
    ///
    /// Note: I'm allowing decrypting of unverified ciphertexts here, because I'm assuming that the
    /// decryption key was derived w.r.t. a digest that only contains verified ciphertexts. If that
    /// invariant holds, anyone should be able to compute plaintexts, even if they themselves
    /// haven't verified the ciphertexts.
    fn decrypt(
        cts: &[Self::Ciphertext],
        aux_info: Self::DecryptionAuxInfo,
        pool: ThreadPool
        ) -> Result<Vec<impl Plaintext>>;
}

/// Defines the parameters of the threshold secret sharing used for the master secret key.
pub struct ThresholdConfig {  }


/// An element of the plaintext space. Does not depend on the specific scheme; any struct that is
/// serializeable should allow for being used as a plaintext.
pub trait Plaintext : Serialize {}
