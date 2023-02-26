// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::types::PersistedValue;
use aptos_consensus_types::proof_of_store::LogicalTime;
use aptos_crypto::HashValue;
use aptos_types::{transaction::SignedTransaction, PeerId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PersistRequest {
    pub digest: HashValue,
    pub value: PersistedValue,
}

impl PersistRequest {
    pub fn new(
        author: PeerId,
        payload: Vec<SignedTransaction>,
        digest_hash: HashValue,
        num_bytes: usize,
        expiration: LogicalTime,
    ) -> Self {
        Self {
            digest: digest_hash,
            value: PersistedValue::new(Some(payload), expiration, author, num_bytes),
        }
    }
}
