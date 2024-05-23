// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::ProofRead;
use aptos_crypto::HashValue;
use std::collections::HashMap;

#[derive(Default)]
pub struct ProofReader(HashMap<HashValue, Option<HashValue>>);

impl ProofReader {
    pub fn new(key_with_proof: Vec<(HashValue, Option<HashValue>)>) -> Self {
        ProofReader(key_with_proof.into_iter().collect())
    }
}

impl ProofRead for ProofReader {
    fn get_proof(&self, key: HashValue) -> Option<HashValue> {
        *self.0.get(&key).unwrap()
    }
}
