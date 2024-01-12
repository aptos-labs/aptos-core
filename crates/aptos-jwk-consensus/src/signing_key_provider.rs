// Copyright © Aptos Foundation

use aptos_config::config::IdentityBlob;
use aptos_crypto::bls12381::PrivateKey;
use std::sync::Arc;

pub trait SigningKeyProvider {
    fn signing_key(&self) -> &PrivateKey;
}

impl SigningKeyProvider for Arc<IdentityBlob> {
    fn signing_key(&self) -> &PrivateKey {
        self.consensus_private_key.as_ref().unwrap()
    }
}

impl SigningKeyProvider for Arc<PrivateKey> {
    fn signing_key(&self) -> &PrivateKey {
        self.as_ref()
    }
}
