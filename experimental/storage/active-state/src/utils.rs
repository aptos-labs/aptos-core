// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use aptos_crypto::hash::HashValue;
use aptos_experimental_scratchpad::sparse_merkle::ProofRead;
use aptos_types::proof::SparseMerkleProofExt;
use std::collections::HashMap;
pub struct BasicProofReader {
    key_to_proof: HashMap<HashValue, SparseMerkleProofExt>,
}

impl BasicProofReader {
    pub fn new() -> Self {
        BasicProofReader {
            key_to_proof: HashMap::new(),
        }
    }

    pub fn add_proof(&mut self, key: HashValue, proof: SparseMerkleProofExt) {
        self.key_to_proof.insert(key, proof);
    }
}

impl ProofRead for BasicProofReader {
    fn get_proof(&self, key: HashValue) -> Option<&SparseMerkleProofExt> {
        self.key_to_proof.get(&key)
    }
}
