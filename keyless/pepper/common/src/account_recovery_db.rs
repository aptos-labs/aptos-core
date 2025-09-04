// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{Deserialize, Serialize};
use velor_crypto::compat::Sha3_256;
use ed25519_dalek::Digest;

/// The schema used in the account recovery DB.
#[derive(Debug, Serialize, Deserialize)]
pub struct AccountRecoveryDbEntry {
    pub iss: String,
    pub aud: String,
    pub uid_key: String,
    pub uid_val: String,

    pub num_requests: Option<u64>,

    /// First request time in unix milliseconds, but minus 1 quadrillion (10^15).
    ///
    /// See comments in function `update_account_recovery_db` for more context.
    pub first_request_unix_ms_minus_1q: Option<i64>,
    pub last_request_unix_ms: Option<u64>,
}

impl AccountRecoveryDbEntry {
    /// Derive a unique ID for a document/data entry.
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
