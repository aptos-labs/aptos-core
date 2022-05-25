// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::ProofRead;
use aptos_crypto::HashValue;
use aptos_types::proof::SparseMerkleProof;
use std::collections::HashMap;

#[derive(Default)]
pub struct ProofReader(HashMap<HashValue, SparseMerkleProof>);

impl ProofReader {
    pub fn new(key_with_proof: Vec<(HashValue, SparseMerkleProof)>) -> Self {
        ProofReader(key_with_proof.into_iter().collect())
    }
}

impl ProofRead for ProofReader {
    fn get_proof(&self, key: HashValue) -> Option<&SparseMerkleProof> {
        self.0.get(&key)
    }
}
