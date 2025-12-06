// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::secret_sharing::SecretShareMetadata;
use serde::{Deserialize, Serialize};

pub const NUM_THREADS_FOR_WVUF_DERIVATION: usize = 8;
pub const FUTURE_ROUNDS_TO_ACCEPT: u64 = 200;

#[derive(Clone, Serialize, Deserialize)]
pub struct RequestSecretShare {
    metadata: SecretShareMetadata,
}

impl RequestSecretShare {
    pub fn new(metadata: SecretShareMetadata) -> Self {
        Self { metadata }
    }

    pub fn epoch(&self) -> u64 {
        self.metadata.epoch
    }

    pub fn metadata(&self) -> &SecretShareMetadata {
        &self.metadata
    }
}
