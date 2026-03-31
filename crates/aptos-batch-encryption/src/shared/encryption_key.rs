// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use crate::{
    errors::BatchEncryptionError,
    group::G2Affine,
    shared::{
        digest::Digest,
        key_derivation::{self, BIBEDecryptionKey},
    },
};
use anyhow::Result;
use aptos_crypto::arkworks::serialization::{ark_de, ark_se};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptionKey {
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub(crate) sig_mpk_g2: G2Affine,
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub(crate) tau_g2: G2Affine,
}

impl EncryptionKey {
    pub fn new(sig_mpk_g2: G2Affine, tau_g2: G2Affine) -> Self {
        Self { sig_mpk_g2, tau_g2 }
    }

    #[cfg(test)]
    pub(crate) fn new_for_testing() -> Self {
        use ark_ec::AffineRepr as _;

        Self {
            sig_mpk_g2: G2Affine::generator(),
            tau_g2: G2Affine::generator(),
        }
    }

    pub fn verify_decryption_key(
        &self,
        digest: &Digest,
        decryption_key: &BIBEDecryptionKey,
    ) -> Result<()> {
        key_derivation::verify_shifted_bls(
            self.sig_mpk_g2,
            digest,
            self.sig_mpk_g2,
            decryption_key.signature_g1,
        )
        .map_err(|_| BatchEncryptionError::DecryptionKeyVerifyError)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AugmentedEncryptionKey {
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub(crate) sig_mpk_g2: G2Affine,
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub(crate) tau_g2: G2Affine,
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub(crate) tau_mpk_g2: G2Affine,
}

impl AugmentedEncryptionKey {
    pub fn new(sig_mpk_g2: G2Affine, tau_g2: G2Affine, tau_mpk_g2: G2Affine) -> Self {
        Self {
            sig_mpk_g2,
            tau_g2,
            tau_mpk_g2,
        }
    }

    pub fn verify_decryption_key(
        &self,
        digest: &Digest,
        decryption_key: &BIBEDecryptionKey,
    ) -> Result<()> {
        key_derivation::verify_shifted_bls(
            self.sig_mpk_g2,
            digest,
            self.sig_mpk_g2,
            decryption_key.signature_g1,
        )
        .map_err(|_| BatchEncryptionError::DecryptionKeyVerifyError)?;
        Ok(())
    }
}
