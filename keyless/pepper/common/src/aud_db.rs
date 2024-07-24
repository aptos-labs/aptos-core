// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{Deserialize, Serialize};
use aptos_crypto::compat::Sha3_256;
use ed25519_dalek::Digest;

/// The schema used in the account recovery DB.
#[derive(Debug, Serialize, Deserialize)]
pub struct AccountRecoveryDbEntry {
    pub iss: String,
    pub aud: String,
    pub uid_key: String,
    pub uid_val: String,
    pub last_request_unix_ms: u64,
}

impl AccountRecoveryDbEntry {
    pub fn document_id(&self) -> String {
        let mut hasher = Sha3_256::new();
        hasher.update((self.iss.len() as u64).to_be_bytes());
        hasher.update(&self.iss);
        hasher.update((self.aud.len() as u64).to_be_bytes());
        hasher.update(&self.aud);
        hasher.update((self.uid_key.len() as u64).to_be_bytes());
        hasher.update(&self.uid_key);
        hasher.update((self.uid_val.len() as u64).to_be_bytes());
        hasher.update(&self.uid_val);
        let digest = hasher.finalize();
        hex::encode(digest.as_slice())
    }
}
