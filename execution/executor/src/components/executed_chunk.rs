// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::components::partial_state_compute_result::PartialStateComputeResult;
use aptos_types::{ledger_info::LedgerInfoWithSignatures, transaction::TransactionToCommit};

#[derive(Debug)]
pub struct ExecutedChunk {
    pub output: PartialStateComputeResult,
    pub ledger_info_opt: Option<LedgerInfoWithSignatures>,
}

impl ExecutedChunk {
    pub fn transactions_to_commit(&self) -> &[TransactionToCommit] {
        &self.output.expect_ledger_update_output().to_commit
    }
}
