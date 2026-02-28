// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_types::secret_sharing::SecretShareMetadata;
use serde::{Deserialize, Serialize};

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
