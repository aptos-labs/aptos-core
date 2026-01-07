// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use crate::{
    shared::{
        ark_serialize::*, digest::Digest,  key_derivation::{
            BIBEDecryptionKey, BIBEMasterPublicKey,
        }
    },
    group::G2Affine,
};
use anyhow::Result;
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

    pub fn verify_decryption_key(
        &self,
        digest: &Digest,
        decryption_key: &BIBEDecryptionKey,
    ) -> Result<()> {
        BIBEMasterPublicKey(self.sig_mpk_g2).verify_decryption_key(digest, decryption_key)
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
        Self { sig_mpk_g2, tau_g2, tau_mpk_g2 }
    }

    pub fn verify_decryption_key(
        &self,
        digest: &Digest,
        decryption_key: &BIBEDecryptionKey,
    ) -> Result<()> {
        BIBEMasterPublicKey(self.sig_mpk_g2).verify_decryption_key(digest, decryption_key)
    }
}
