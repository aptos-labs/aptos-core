// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{corpus_from_strategy, fuzz_data_to_value, FuzzTargetImpl};
use aptos_crypto::HashValue;
use aptos_proptest_helpers::ValueGenerator;
use aptos_types::{ledger_info::LedgerInfoWithSignatures, transaction::Transaction};
use executor::fuzzing::fuzz_execute_and_commit_blocks;
use proptest::{collection::vec, prelude::*};

#[derive(Clone, Debug, Default)]
pub struct ExecuteAndCommitChunk;

#[derive(Clone, Debug, Default)]
pub struct ExecuteAndCommitBlocks;

impl FuzzTargetImpl for ExecuteAndCommitBlocks {
    fn description(&self) -> &'static str {
        "LEC > executor::execute_block & executor::commit_blocks"
    }

    fn generate(&self, _idx: usize, _gen: &mut ValueGenerator) -> Option<Vec<u8>> {
        Some(corpus_from_strategy(execute_and_commit_blocks_input()))
    }

    fn fuzz(&self, data: &[u8]) {
        let (blocks, li_with_sigs) = fuzz_data_to_value(data, execute_and_commit_blocks_input());
        fuzz_execute_and_commit_blocks(blocks, li_with_sigs);
    }
}

prop_compose! {
    fn execute_and_commit_blocks_input()(
        blocks in vec((any::<HashValue>(), vec(any::<Transaction>(), 0..10)), 0..10),
        li_with_sigs in any::<LedgerInfoWithSignatures>()
    ) -> (Vec<(HashValue, Vec<Transaction>)>, LedgerInfoWithSignatures) {
        (blocks, li_with_sigs)
    }
}
