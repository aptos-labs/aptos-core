// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::ProofRead;
use velor_crypto::HashValue;
use velor_types::proof::SparseMerkleProofExt;
use std::collections::HashMap;

#[derive(Default)]
pub struct ProofReader(HashMap<HashValue, SparseMerkleProofExt>);

impl ProofReader {
    pub fn new(key_with_proof: Vec<(HashValue, SparseMerkleProofExt)>) -> Self {
        ProofReader(key_with_proof.into_iter().collect())
    }
}

impl ProofRead for ProofReader {
    fn get_proof(&self, key: &HashValue, root_depth: usize) -> Option<SparseMerkleProofExt> {
        let ret = self.0.get(key);
        if let Some(proof) = ret {
            assert!(proof.root_depth() <= root_depth);
        }
        ret.cloned()
    }
}
