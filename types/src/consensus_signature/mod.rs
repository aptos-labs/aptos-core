// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The signature scheme that validators use to sign consensus messages.
//!
//! The default scheme is BLS12-381. For performance experiments, the `pq-consensus`
//! feature replaces it with ML-DSA-65, and the `pq-consensus-falcon` feature replaces it
//! with FN-DSA-512 (Falcon-512).
//!
//! Each scheme provides four types:
//! - [`ConsensusSignature`]: the signature of one validator.
//! - [`LazyConsensusSignature`]: the wire form of [`ConsensusSignature`].
//! - [`ConsensusMultiSignature`]: the signatures of a set of validators, as a certificate
//!   carries them.
//! - [`LazyConsensusMultiSignature`]: the wire form of [`ConsensusMultiSignature`].
//!
//! The post-quantum schemes also provide [`PqSigningKey`] and [`PqVerificationKey`]. Both
//! are derived from the BLS public key of a validator, which is public, so these schemes
//! give no security.

#[cfg(all(feature = "pq-consensus", feature = "pq-consensus-falcon"))]
compile_error!("Enable at most one of `pq-consensus` and `pq-consensus-falcon`.");

#[cfg(feature = "pq-consensus-falcon")]
pub mod falcon;
#[cfg(feature = "pq-consensus")]
pub mod ml_dsa;

#[cfg(not(feature = "pq-consensus-any"))]
mod scheme {
    use crate::lazy_bls::LazyBlsSignature;
    use aptos_crypto::{bls12381, hash::CryptoHash, CryptoMaterialError, Signature, SigningKey};
    use serde::Serialize;

    pub type ConsensusSignature = bls12381::Signature;
    pub type LazyConsensusSignature = LazyBlsSignature;
    pub type ConsensusMultiSignature = bls12381::Signature;
    pub type LazyConsensusMultiSignature = LazyBlsSignature;

    /// Signs `message` with the consensus key of a validator.
    pub fn sign<T: Serialize + CryptoHash>(
        private_key: &bls12381::PrivateKey,
        message: &T,
    ) -> Result<ConsensusSignature, CryptoMaterialError> {
        private_key.sign(message)
    }

    /// Verifies the signature of one validator on `message`.
    pub fn verify<T: Serialize + CryptoHash>(
        public_key: &bls12381::PublicKey,
        message: &T,
        signature: &ConsensusSignature,
    ) -> anyhow::Result<()> {
        signature.verify(message, public_key)
    }
}

#[cfg(feature = "pq-consensus-any")]
mod scheme {
    #[cfg(feature = "pq-consensus-falcon")]
    pub use super::falcon::{
        FalconMultiSignature as ConsensusMultiSignature,
        FalconMultiSignature as LazyConsensusMultiSignature, FalconSignature as ConsensusSignature,
        FalconSignature as LazyConsensusSignature, FalconSigningKey as PqSigningKey,
        FalconVerificationKey as PqVerificationKey,
    };
    #[cfg(feature = "pq-consensus")]
    pub use super::ml_dsa::{
        MlDsaMultiSignature as ConsensusMultiSignature,
        MlDsaMultiSignature as LazyConsensusMultiSignature, MlDsaSignature as ConsensusSignature,
        MlDsaSignature as LazyConsensusSignature, MlDsaSigningKey as PqSigningKey,
        MlDsaVerificationKey as PqVerificationKey,
    };
    use aptos_crypto::{bls12381, hash::CryptoHash, CryptoMaterialError, PrivateKey};
    use dashmap::DashMap;
    use once_cell::sync::Lazy;
    use serde::Serialize;
    use std::sync::Arc;

    /// Verification keys by BLS public key. Deriving a key can take milliseconds (FN-DSA
    /// key generation), and code paths build new `ValidatorVerifier`s often, so keys are
    /// derived once per process.
    static VERIFICATION_KEYS: Lazy<DashMap<Vec<u8>, Arc<PqVerificationKey>>> =
        Lazy::new(DashMap::new);

    /// Returns the verification key derived from `public_key`.
    pub fn pq_verification_key(public_key: &bls12381::PublicKey) -> Arc<PqVerificationKey> {
        let id = public_key.to_bytes().to_vec();
        if let Some(key) = VERIFICATION_KEYS.get(&id) {
            return key.clone();
        }
        let key = Arc::new(PqVerificationKey::derive(public_key));
        VERIFICATION_KEYS.entry(id).or_insert(key).clone()
    }

    /// Signs `message` with the key derived from the consensus key of a validator.
    /// This derives the key on each call. `ValidatorSigner` caches the key.
    pub fn sign<T: Serialize + CryptoHash>(
        private_key: &bls12381::PrivateKey,
        message: &T,
    ) -> Result<ConsensusSignature, CryptoMaterialError> {
        PqSigningKey::derive(&private_key.public_key()).sign(message)
    }

    /// Verifies the signature of one validator on `message`.
    pub fn verify<T: Serialize + CryptoHash>(
        public_key: &bls12381::PublicKey,
        message: &T,
        signature: &ConsensusSignature,
    ) -> anyhow::Result<()> {
        pq_verification_key(public_key)
            .verify(message, signature)
            .map_err(|_| anyhow::anyhow!("invalid post-quantum consensus signature"))
    }
}

pub use scheme::*;
