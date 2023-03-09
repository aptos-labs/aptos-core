// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::FuzzTargetImpl;
use aptos_consensus::round_manager_fuzzing::{fuzz_proposal, generate_corpus_proposal};
use aptos_proptest_helpers::ValueGenerator;

#[derive(Clone, Debug, Default)]
pub struct ConsensusProposal;

impl FuzzTargetImpl for ConsensusProposal {
    fn description(&self) -> &'static str {
        "Consensus proposal messages"
    }

    fn generate(&self, _idx: usize, _gen: &mut ValueGenerator) -> Option<Vec<u8>> {
        Some(generate_corpus_proposal())
    }

    fn fuzz(&self, data: &[u8]) {
        fuzz_proposal(data);
    }
}
