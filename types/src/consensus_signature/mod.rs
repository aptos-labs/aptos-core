// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The signature scheme that validators use to sign consensus messages.
//!
//! The default scheme is BLS12-381. The `pq-consensus` feature replaces it with
//! ML-DSA-65 for performance experiments.
//!
//! Each scheme provides four types:
//! - [`ConsensusSignature`]: the signature of one validator.
//! - [`LazyConsensusSignature`]: the wire form of [`ConsensusSignature`].
//! - [`ConsensusMultiSignature`]: the signatures of a set of validators, as a certificate
//!   carries them.
//! - [`LazyConsensusMultiSignature`]: the wire form of [`ConsensusMultiSignature`].

#[cfg(feature = "pq-consensus")]
pub mod ml_dsa;

#[cfg(not(feature = "pq-consensus"))]
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

#[cfg(feature = "pq-consensus")]
mod scheme {
    use super::ml_dsa::{
        MlDsaMultiSignature, MlDsaSignature, MlDsaSigningKey, MlDsaVerificationKey,
    };
    use aptos_crypto::{bls12381, hash::CryptoHash, CryptoMaterialError, PrivateKey};
    use serde::Serialize;

    pub type ConsensusSignature = MlDsaSignature;
    pub type LazyConsensusSignature = MlDsaSignature;
    pub type ConsensusMultiSignature = MlDsaMultiSignature;
    pub type LazyConsensusMultiSignature = MlDsaMultiSignature;

    /// Signs `message` with the ML-DSA key derived from the consensus key of a validator.
    /// This derives the key on each call. `ValidatorSigner` caches the key.
    pub fn sign<T: Serialize + CryptoHash>(
        private_key: &bls12381::PrivateKey,
        message: &T,
    ) -> Result<ConsensusSignature, CryptoMaterialError> {
        MlDsaSigningKey::derive(&private_key.public_key()).sign(message)
    }

    /// Verifies the signature of one validator on `message`.
    /// This derives the key on each call. `ValidatorVerifier` caches the keys.
    pub fn verify<T: Serialize + CryptoHash>(
        public_key: &bls12381::PublicKey,
        message: &T,
        signature: &ConsensusSignature,
    ) -> anyhow::Result<()> {
        MlDsaVerificationKey::derive(public_key)
            .verify(message, signature)
            .map_err(|_| anyhow::anyhow!("invalid ML-DSA-65 signature"))
    }
}

pub use scheme::*;
