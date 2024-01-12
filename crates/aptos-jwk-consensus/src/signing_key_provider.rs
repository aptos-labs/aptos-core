// Copyright © Aptos Foundation

use anyhow::{anyhow, Result};
use aptos_config::config::IdentityBlob;
use aptos_crypto::bls12381::PrivateKey;
use std::sync::Arc;

pub trait SigningKeyProvider {
    fn signing_key(&self) -> Result<&PrivateKey>;
}

impl SigningKeyProvider for Arc<IdentityBlob> {
    fn signing_key(&self) -> Result<&PrivateKey> {
        self.consensus_private_key
            .as_ref()
            .ok_or_else(|| anyhow!("signing key is missing"))
    }
}

impl SigningKeyProvider for Arc<PrivateKey> {
    fn signing_key(&self) -> Result<&PrivateKey> {
        Ok(self.as_ref())
    }
}
